use std::{
    fs,
    path::{Path, PathBuf},
};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;
use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

use crate::error::{AppError, AppResult};

pub async fn read_chrome_cookie_header(host_filter: &str) -> AppResult<String> {
    let user_data_dir = chrome_user_data_dir()?;
    let key = chrome_master_key(&user_data_dir)?;
    let mut cookies = Vec::new();

    for profile in chrome_profiles(&user_data_dir)? {
        let cookie_db = profile.join("Network").join("Cookies");
        if !cookie_db.exists() {
            continue;
        }
        let copied_db = std::env::temp_dir().join(format!(
            "tokenball-chrome-cookies-{}.sqlite",
            Uuid::new_v4()
        ));
        fs::copy(&cookie_db, &copied_db).map_err(|error| {
            AppError::Message(format!("复制 Chrome Cookie 数据库失败：{error}"))
        })?;
        let profile_cookies = read_cookie_db(&copied_db, host_filter, &key).await;
        let _ = fs::remove_file(&copied_db);
        cookies.extend(profile_cookies?);
    }

    if cookies.is_empty() {
        return Err(AppError::Message(format!(
            "未在 Chrome 中找到 {host_filter} 的 Cookie，请先在 Chrome 登录火山控制台"
        )));
    }
    cookies.sort_by(|a, b| a.0.cmp(&b.0));
    cookies.dedup_by(|a, b| a.0 == b.0);
    Ok(cookies
        .into_iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; "))
}

async fn read_cookie_db(
    db_path: &Path,
    host_filter: &str,
    key: &[u8],
) -> AppResult<Vec<(String, String)>> {
    let url = format!("sqlite:{}?mode=ro", db_path.to_string_lossy());
    let pool = SqlitePool::connect(&url).await?;
    let pattern = format!("%{host_filter}%");
    let rows = sqlx::query(
        r#"
        SELECT name, value, encrypted_value
        FROM cookies
        WHERE host_key LIKE ?1
        ORDER BY host_key ASC, path ASC, name ASC
        "#,
    )
    .bind(pattern)
    .fetch_all(&pool)
    .await?;
    let mut cookies = Vec::new();
    for row in rows {
        let name: String = row.try_get("name")?;
        let value: String = row.try_get("value")?;
        let encrypted_value: Vec<u8> = row.try_get("encrypted_value")?;
        let value = if !value.is_empty() {
            value
        } else {
            decrypt_chrome_cookie(&encrypted_value, key)?
        };
        if !name.is_empty() && !value.is_empty() {
            cookies.push((name, value));
        }
    }
    Ok(cookies)
}

fn chrome_user_data_dir() -> AppResult<PathBuf> {
    let local = dirs::data_local_dir()
        .ok_or_else(|| AppError::Message("无法定位 LocalAppData 目录".to_string()))?;
    Ok(local.join("Google").join("Chrome").join("User Data"))
}

fn chrome_profiles(user_data_dir: &Path) -> AppResult<Vec<PathBuf>> {
    let mut profiles = Vec::new();
    for entry in fs::read_dir(user_data_dir)
        .map_err(|error| AppError::Message(format!("读取 Chrome 用户目录失败：{error}")))?
    {
        let entry = entry
            .map_err(|error| AppError::Message(format!("读取 Chrome Profile 失败：{error}")))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name == "Default" || name.starts_with("Profile ") {
            profiles.push(path);
        }
    }
    Ok(profiles)
}

fn chrome_master_key(user_data_dir: &Path) -> AppResult<Vec<u8>> {
    let raw = fs::read_to_string(user_data_dir.join("Local State"))
        .map_err(|error| AppError::Message(format!("读取 Chrome Local State 失败：{error}")))?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| AppError::Message(format!("解析 Chrome Local State 失败：{error}")))?;
    let encrypted_key = value
        .get("os_crypt")
        .and_then(|value| value.get("encrypted_key"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| AppError::Message("Chrome Local State 缺少 encrypted_key".to_string()))?;
    let mut encrypted_key = base64::engine::general_purpose::STANDARD
        .decode(encrypted_key)
        .map_err(|error| AppError::Message(format!("解码 Chrome encrypted_key 失败：{error}")))?;
    if encrypted_key.starts_with(b"DPAPI") {
        encrypted_key.drain(..5);
    }
    dpapi_unprotect(&encrypted_key)
}

fn decrypt_chrome_cookie(encrypted_value: &[u8], key: &[u8]) -> AppResult<String> {
    if encrypted_value.starts_with(b"v10") || encrypted_value.starts_with(b"v11") {
        if encrypted_value.len() < 3 + 12 + 16 {
            return Err(AppError::Message(
                "Chrome Cookie 加密数据长度异常".to_string(),
            ));
        }
        let nonce = Nonce::from_slice(&encrypted_value[3..15]);
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|error| AppError::Message(format!("初始化 Cookie 解密器失败：{error}")))?;
        let plain = cipher
            .decrypt(nonce, &encrypted_value[15..])
            .map_err(|error| AppError::Message(format!("解密 Chrome Cookie 失败：{error}")))?;
        return String::from_utf8(plain)
            .map_err(|error| AppError::Message(format!("Cookie UTF-8 解码失败：{error}")));
    }
    let plain = dpapi_unprotect(encrypted_value)?;
    String::from_utf8(plain)
        .map_err(|error| AppError::Message(format!("Cookie UTF-8 解码失败：{error}")))
}

fn dpapi_unprotect(data: &[u8]) -> AppResult<Vec<u8>> {
    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = CryptUnprotectData(
            &mut input,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut output,
        );
        if ok == 0 {
            return Err(AppError::Message("Windows DPAPI 解密失败".to_string()));
        }
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        Ok(bytes)
    }
}
