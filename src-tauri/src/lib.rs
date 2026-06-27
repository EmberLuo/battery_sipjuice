//! 本机电源助手 — 库入口。

mod battery;
mod charge_control;
mod commands;
mod power;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::set_charge_threshold,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
