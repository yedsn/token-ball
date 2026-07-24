use tauri::{AppHandle, Emitter};

use crate::quota::QuotaSummary;

pub fn emit_quota_updated(app: &AppHandle, summary: &QuotaSummary) {
    let _ = app.emit("quota://updated", summary);
}

pub fn emit_refresh_started(app: &AppHandle) {
    let _ = app.emit("quota://refresh-started", ());
}

pub fn emit_refresh_completed(app: &AppHandle, summary: &QuotaSummary) {
    let _ = app.emit("quota://refresh-completed", summary);
}

pub fn emit_provider_error(app: &AppHandle, message: &str) {
    let _ = app.emit("provider://error", message);
}
