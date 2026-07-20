//! Battery SipJuice — 库入口。

mod app_power;
mod battery;
mod commands;
mod history;
mod insights;
mod power;
mod reminder;
mod settings;
mod system_accent;

use std::time::Duration;
use tauri::{
    menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItem, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;

/// 后台采样间隔: 30 秒。
const SAMPLE_INTERVAL: Duration = Duration::from_secs(30);

struct TrayMenuItems {
    show: MenuItem<tauri::Wry>,
    lightweight: CheckMenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}

struct TrayLabels {
    show: &'static str,
    lightweight: &'static str,
    quit: &'static str,
}

fn tray_labels(language: &str) -> TrayLabels {
    if language == "en-US" {
        TrayLabels {
            show: "Show Window",
            lightweight: "Lightweight Mode",
            quit: "Quit",
        }
    } else {
        TrayLabels {
            show: "显示窗口",
            lightweight: "轻量模式",
            quit: "退出",
        }
    }
}

pub(crate) fn update_tray_menu_language(app: &tauri::AppHandle, language: &str) {
    let labels = tray_labels(language);
    let items = app.state::<TrayMenuItems>();
    let _ = items.show.set_text(labels.show);
    let _ = items.lightweight.set_text(labels.lightweight);
    let _ = items.quit.set_text(labels.quit);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 单实例：必须最先注册。再次启动(如从应用菜单点开)时,
        // 不会开新进程, 而是回调里唤起已有窗口。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            // 历史数据落在应用数据目录下的 history.rdb (定长 RRD 风格归档)。
            // 若存在旧版 history.jsonl 会在 load 时自动迁移。
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let hist = history::HistoryStore::load(data_dir.join("history.rdb"));
            app.manage(hist);
            let insights = insights::InsightsStore::load(data_dir.join("insights.json"));
            app.manage(insights);

            // 设置落在应用配置目录下的 settings.json。
            let cfg_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&cfg_dir)?;
            let settings_store = settings::SettingsStore::load(cfg_dir.join("settings.json"));
            let current = settings_store.get();
            app.manage(settings_store);

            // 按应用耗电估算：状态只由后台线程写入，命令层只读。
            app.manage(app_power::AppPowerStore::default());

            // 系统托盘 + 右键菜单（显示窗口 / 轻量模式 / 退出）。
            let labels = tray_labels(&current.language);
            let show_item = MenuItemBuilder::with_id("show", labels.show).build(app)?;
            let light_item = CheckMenuItemBuilder::with_id("lightweight", labels.lightweight)
                .checked(false)
                .build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", labels.quit).build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&light_item)
                .separator()
                .item(&quit_item)
                .build()?;
            let light_item_for_menu = light_item.clone();
            let light_item_for_tray = light_item.clone();
            let mut tray = TrayIconBuilder::with_id("main-tray")
                .tooltip("Battery SipJuice")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        let _ = light_item_for_menu.set_checked(false);
                        show_main(app);
                    }
                    "lightweight" => {
                        let enabled = light_item_for_menu.is_checked().unwrap_or(false);
                        apply_lightweight_mode(app, enabled);
                    }
                    "quit" => {
                        if let Err(error) = app.state::<insights::InsightsStore>().flush() {
                            eprintln!("insights: 退出前保存失败: {error}");
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(move |tray, event| {
                    // 左键点击托盘图标显示窗口（部分 Linux 托盘不发此事件，菜单为兜底）。
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let _ = light_item_for_tray.set_checked(false);
                        show_main(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon().cloned() {
                tray = tray.icon(icon);
            }
            tray.build(app)?;
            app.manage(TrayMenuItems {
                show: show_item,
                lightweight: light_item,
                quit: quit_item,
            });

            // 关闭按钮(X)拦截：按设置决定退出 / 最小化到托盘 / 弹框询问。
            if let Some(window) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        let action = handle.state::<settings::SettingsStore>().get().close_action;
                        match action {
                            // Exit: 不拦截，窗口关闭 → 应用退出。
                            settings::CloseAction::Exit => {
                                if let Err(error) =
                                    handle.state::<insights::InsightsStore>().flush()
                                {
                                    eprintln!("insights: 退出前保存失败: {error}");
                                }
                            }
                            settings::CloseAction::Tray => {
                                api.prevent_close();
                                if let Some(w) = handle.get_webview_window("main") {
                                    let _ = w.hide();
                                }
                            }
                            settings::CloseAction::Ask => {
                                api.prevent_close();
                                // 通知前端弹出确认框。
                                let _ = handle.emit("close-requested", ());
                            }
                        }
                    }
                });

                // 静默启动：不显示窗口，仅托盘（窗口配置默认 visible:false）。
                if !current.silent_start {
                    let _ = window.show();
                }
            }

            // 后台同步线程定时采样(读 sysfs + 写文件均为同步操作，用 std 线程即可)。
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut fired = reminder::Fired::default();
                let mut snapshot = |h: &tauri::AppHandle| {
                    let batteries = battery::collect_all();
                    let sources = power::collect();
                    h.state::<history::HistoryStore>()
                        .tick(&batteries, &sources);
                    h.state::<insights::InsightsStore>().tick(
                        &batteries,
                        &sources,
                        history::now_ms(),
                    );
                    let settings = h.state::<settings::SettingsStore>().get();
                    reminder::evaluate(h, &settings, &batteries, &mut fired);
                    let battery_power_w = batteries
                        .iter()
                        .filter(|battery| battery.status.as_deref() == Some("Discharging"))
                        .filter_map(|battery| battery.power_now.map(f64::abs))
                        .reduce(|total, power| total + power);
                    h.state::<app_power::AppPowerStore>().tick(battery_power_w);
                };
                snapshot(&handle);
                loop {
                    std::thread::sleep(SAMPLE_INTERVAL);
                    snapshot(&handle);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::get_history,
            commands::get_settings,
            commands::get_app_version,
            commands::get_system_accent_color,
            commands::save_settings,
            commands::get_app_power_report,
            commands::get_battery_insights,
            commands::hide_window,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}

/// 显示并聚焦主窗口。
fn show_main<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// 轻量模式：只留托盘运行；取消后恢复并聚焦主窗口。
fn apply_lightweight_mode<R: tauri::Runtime>(app: &tauri::AppHandle<R>, enabled: bool) {
    if let Some(w) = app.get_webview_window("main") {
        if enabled {
            let _ = w.hide();
        } else {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_labels_follow_language() {
        assert_eq!(tray_labels("en-US").show, "Show Window");
        assert_eq!(tray_labels("zh-CN").lightweight, "轻量模式");
        assert_eq!(tray_labels("unknown").quit, "退出");
    }
}
