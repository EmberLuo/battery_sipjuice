//! Tauri 命令层 — 暴露给前端的 IPC 接口。

use crate::{battery, charge_control, history, power, settings};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

/// 一次完整的系统电源快照。
#[derive(Serialize)]
pub struct Snapshot {
    pub battery: Option<battery::BatteryInfo>,
    pub sources: Vec<power::PowerSource>,
    pub charge_control: Option<charge_control::ChargeControl>,
    pub timestamp_ms: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 采集电池 + 电源 + 充电控制的完整快照。前端定时轮询此命令。
#[tauri::command]
pub fn get_snapshot() -> Snapshot {
    let bat = battery::collect();
    let cc = bat.as_ref().map(|b| charge_control::status(&b.device));
    Snapshot {
        battery: bat,
        sources: power::collect(),
        charge_control: cc,
        timestamp_ms: now_ms(),
    }
}

/// 应用充电阈值（实验性，可能改变充电行为，可能需要 root）。
#[tauri::command]
pub fn set_charge_threshold(start: i64, end: i64) -> Result<String, String> {
    let dev = battery::find_battery().ok_or("未找到电池设备")?;
    charge_control::apply(&dev, start, end)
}

/// 查询最近 range_ms 内的历史采样（已降采样），供前端画曲线。
#[tauri::command]
pub fn get_history(
    range_ms: u64,
    store: tauri::State<'_, history::HistoryStore>,
) -> Vec<history::Sample> {
    store.query(range_ms)
}

/// 读取当前应用设置。
#[tauri::command]
pub fn get_settings(store: tauri::State<'_, settings::SettingsStore>) -> settings::Settings {
    store.get()
}

/// 整体保存应用设置，并同步开机自启状态到系统。
#[tauri::command]
pub fn save_settings(
    new_settings: settings::Settings,
    app: AppHandle,
    store: tauri::State<'_, settings::SettingsStore>,
) -> Result<(), String> {
    // 先确保设置落盘成功，再同步系统状态，避免两者不一致。
    let want = new_settings.autostart;
    store.set(new_settings)?;

    // 同步 autostart 到系统（LaunchAgent / XDG autostart）。
    let launcher = app.autolaunch();
    let is_on = launcher.is_enabled().unwrap_or(false);
    if want && !is_on {
        launcher.enable().map_err(|e| e.to_string())?;
    } else if !want && is_on {
        launcher.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 隐藏主窗口（关闭按钮选择"最小化到托盘"时调用）。
#[tauri::command]
pub fn hide_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

/// 退出整个应用。
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
