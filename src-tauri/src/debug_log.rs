use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::OnceLock;

use chrono::Local;

static LOG_PATH: OnceLock<std::path::PathBuf> = OnceLock::new();

pub fn init() {
    let _ = path();
}

pub fn line(message: impl AsRef<str>) {
    let path = path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{timestamp}] {}\n", message.as_ref());
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
}

fn path() -> std::path::PathBuf {
    LOG_PATH
        .get_or_init(|| {
            let base = dirs::data_local_dir().unwrap_or_else(|| std::env::temp_dir());
            base.join("TokenBall").join("tokenball-debug.log")
        })
        .clone()
}
