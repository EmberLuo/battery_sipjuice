//! Tauri 命令层 — 暴露给前端的 IPC 接口。

use crate::{app_power, battery, history, insights, power, settings, system_accent};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

/// 一次完整的系统电源快照。
#[derive(Serialize)]
pub struct Snapshot {
    pub batteries: Vec<battery::BatteryInfo>,
    pub sources: Vec<power::PowerSource>,
    pub timestamp_ms: u64,
}

/// 采集电池 + 电源的完整快照。前端定时轮询此命令。
#[tauri::command]
pub fn get_snapshot() -> Snapshot {
    Snapshot {
        batteries: battery::collect_all(),
        sources: power::collect(),
        timestamp_ms: history::now_ms(),
    }
}

/// 查询最近 range_ms 内的历史采样（已降采样），供前端画曲线。
#[tauri::command]
pub fn get_history(
    range_ms: u64,
    source_kind: Option<String>,
    battery_device_id: Option<String>,
    input_source_id: Option<String>,
    store: tauri::State<'_, history::HistoryStore>,
) -> Vec<history::Sample> {
    store.query(
        range_ms,
        source_kind.as_deref().unwrap_or("battery"),
        battery_device_id.as_deref(),
        input_source_id.as_deref(),
    )
}

/// 读取当前应用设置。
#[tauri::command]
pub fn get_settings(store: tauri::State<'_, settings::SettingsStore>) -> settings::Settings {
    store.get()
}

/// 读取当前应用版本，避免前端硬编码版本号。
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 读取系统强调色。优先使用 GTK 当前主题的真实选中色，gsettings 映射作为兜底。
#[tauri::command]
pub fn get_system_accent_color() -> Option<system_accent::SystemAccentColor> {
    system_accent::detect()
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
    let language = new_settings.language.clone();
    store.set(new_settings)?;
    crate::update_tray_menu_language(&app, &language);

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

/// 读取按应用耗电估算 (CPU 时间占比加权分配电池瞬时功率，Top N)。
#[tauri::command]
pub fn get_app_power_report(
    store: tauri::State<'_, app_power::AppPowerStore>,
) -> app_power::AppPowerReport {
    store.latest()
}

/// 查询指定电池的充电会话和长期健康快照。
#[tauri::command]
pub fn get_battery_insights(
    battery_id: Option<String>,
    store: tauri::State<'_, insights::InsightsStore>,
) -> insights::InsightsView {
    store.view(battery_id.as_deref())
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
pub fn quit_app(app: AppHandle, store: tauri::State<'_, insights::InsightsStore>) {
    if let Err(error) = store.flush() {
        eprintln!("insights: 退出前保存失败: {error}");
    }
    app.exit(0);
}
