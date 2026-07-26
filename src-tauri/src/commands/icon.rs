use base64::Engine;
use tauri::{image::Image, AppHandle, Manager};

use crate::{quota::ConnectionStatus, tray};

fn meter_icon() -> Image<'static> {
    Image::new(
        crate::app_icon_rgba::APP_ICON_RGBA,
        crate::app_icon_rgba::APP_ICON_WIDTH,
        crate::app_icon_rgba::APP_ICON_HEIGHT,
    )
    .to_owned()
}

fn orb_icon() -> Image<'static> {
    tray::quota_orb_icon(72.0, ConnectionStatus::Healthy, false)
}

fn image_from_data_url(data_url: &str) -> Result<Image<'static>, String> {
    let comma = data_url
        .find(',')
        .ok_or_else(|| "图标数据格式不正确".to_string())?;
    let (header, payload) = data_url.split_at(comma);
    if header.contains("svg") {
        return Err("运行时程序图标暂不支持 SVG，请选择 PNG、JPG 或 WebP".to_string());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload[1..].as_bytes())
        .map_err(|error| format!("图标 Base64 解码失败：{error}"))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("图标图片解析失败：{error}"))?;
    let resized = image
        .resize_exact(32, 32, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let (width, height) = resized.dimensions();
    Ok(Image::new_owned(resized.into_raw(), width, height))
}

fn icon_for_style(style: &str, custom_data_url: Option<&str>) -> Result<Image<'static>, String> {
    match style {
        "orb" => Ok(orb_icon()),
        "custom" => {
            let data_url = custom_data_url
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "请选择自定义图标文件".to_string())?;
            image_from_data_url(data_url)
        }
        _ => Ok(meter_icon()),
    }
}

#[tauri::command]
pub fn app_icon_set_style(
    app: AppHandle,
    style: String,
    custom_data_url: Option<String>,
) -> Result<(), String> {
    let icon = icon_for_style(&style, custom_data_url.as_deref())?;
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_icon(icon)
            .map_err(|error| format!("设置窗口图标失败：{error}"))?;
    }
    Ok(())
}
