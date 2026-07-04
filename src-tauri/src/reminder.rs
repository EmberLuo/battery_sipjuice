//! 电池养护提醒 — 纯软件、全设备通用的可选提醒。
//!
//! 这里只按电量阈值弹系统通知，不触碰任何硬件接口，
//! 因此在任何带电池的 Linux 设备上都能用。两个提醒彼此独立、各有开关:
//!   · 低电量(放电中) → 提醒接上电源
//!   · 高电量(充电中) → 提醒拔掉电源，避免长期满电浮充加速老化
//!
//! 边沿触发: 进入阈值区间时只提醒一次，离开后复位，避免每次采样都重复打扰。

use crate::battery;
use crate::settings::Settings;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// 记录两个提醒是否已发出(待复位)。由后台采样线程独占持有，故无需加锁。
#[derive(Default)]
pub struct Fired {
    unplug: bool,
    charge: bool,
}

/// 读取当前电量/状态，按设置决定是否弹通知。后台采样线程每次 tick 调用。
pub fn evaluate(app: &AppHandle, s: &Settings, fired: &mut Fired) {
    if !s.remind_charge && !s.remind_unplug {
        return;
    }
    let Some(dev) = battery::find_battery() else {
        return;
    };
    let Some(cap) = battery::read_i64(&dev, "capacity") else {
        return;
    };
    let status = battery::read_raw(&dev, "status");
    let plugged = matches!(status.as_deref(), Some("Charging") | Some("Full"));
    let discharging = status.as_deref() == Some("Discharging");

    // 高电量 → 提醒拔电源(仅充电/满电时有意义)。
    if s.remind_unplug && plugged && cap >= s.remind_unplug_at {
        if !fired.unplug {
            notify(
                app,
                "电量充足",
                &format!("电量已达 {cap}%，建议拔掉电源。长期满电浮充会加速电池老化。"),
            );
            fired.unplug = true;
        }
    } else if discharging || cap < s.remind_unplug_at {
        fired.unplug = false;
    }

    // 低电量 → 提醒充电(仅放电时有意义)。
    if s.remind_charge && discharging && cap <= s.remind_charge_at {
        if !fired.charge {
            notify(
                app,
                "电量偏低",
                &format!("电量已降至 {cap}%，建议接上电源充电。"),
            );
            fired.charge = true;
        }
    } else if plugged || cap > s.remind_charge_at {
        fired.charge = false;
    }
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}
