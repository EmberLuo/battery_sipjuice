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
use std::collections::HashSet;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// 记录两个提醒是否已发出(待复位)。由后台采样线程独占持有，故无需加锁。
#[derive(Default)]
pub struct Fired {
    unplug: HashSet<String>,
    charge: HashSet<String>,
}

/// 读取当前电量/状态，按设置决定是否弹通知。后台采样线程每次 tick 调用。
pub fn evaluate(
    app: &AppHandle,
    s: &Settings,
    batteries: &[battery::BatteryInfo],
    fired: &mut Fired,
) {
    if !s.remind_charge {
        fired.charge.clear();
    }
    if !s.remind_unplug {
        fired.unplug.clear();
    }
    if !s.remind_charge && !s.remind_unplug {
        return;
    }

    let active_devices = batteries
        .iter()
        .map(|battery| battery.device.as_str())
        .collect::<HashSet<_>>();
    fired
        .unplug
        .retain(|device| active_devices.contains(device.as_str()));
    fired
        .charge
        .retain(|device| active_devices.contains(device.as_str()));

    for battery in batteries {
        let Some(capacity) = battery.capacity else {
            continue;
        };
        let status = battery.status.as_deref();
        let plugged = matches!(status, Some("Charging") | Some("Full"));
        let discharging = status == Some("Discharging");
        let name = battery
            .model
            .as_deref()
            .or(battery.manufacturer.as_deref())
            .unwrap_or(&battery.device);
        let label = if batteries.len() > 1 && name != battery.device {
            format!("{name} · {}", battery.device)
        } else {
            name.to_string()
        };

        // 高电量 → 提醒拔电源(仅充电/满电时有意义)。
        if s.remind_unplug && plugged && capacity >= s.remind_unplug_at {
            if fired.unplug.insert(battery.device.clone()) {
                let (title, body) = high_battery_text(&s.language, &label, capacity);
                notify(app, &title, &body);
            }
        } else if discharging || capacity < s.remind_unplug_at {
            fired.unplug.remove(&battery.device);
        }

        // 低电量 → 提醒充电(仅放电时有意义)。
        if s.remind_charge && discharging && capacity <= s.remind_charge_at {
            if fired.charge.insert(battery.device.clone()) {
                let (title, body) = low_battery_text(&s.language, &label, capacity);
                notify(app, &title, &body);
            }
        } else if plugged || capacity > s.remind_charge_at {
            fired.charge.remove(&battery.device);
        }
    }
}

fn high_battery_text(language: &str, label: &str, capacity: i64) -> (String, String) {
    if language == "en-US" {
        (
            "Battery Charged".to_string(),
            format!(
                "{label} has reached {capacity}%. Consider unplugging it; keeping a battery fully charged for long periods can accelerate aging."
            ),
        )
    } else {
        (
            "电量充足".to_string(),
            format!("{label} 电量已达 {capacity}%，建议拔掉电源。长期满电浮充会加速电池老化。"),
        )
    }
}

fn low_battery_text(language: &str, label: &str, capacity: i64) -> (String, String) {
    if language == "en-US" {
        (
            "Low Battery".to_string(),
            format!("{label} has dropped to {capacity}%. Connect a power source to charge it."),
        )
    } else {
        (
            "电量偏低".to_string(),
            format!("{label} 电量已降至 {capacity}%，建议接上电源充电。"),
        )
    }
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminder_text_follows_language() {
        let (title, body) = high_battery_text("en-US", "BAT0", 80);
        assert_eq!(title, "Battery Charged");
        assert!(body.contains("BAT0 has reached 80%"));

        let (title, body) = low_battery_text("zh-CN", "BAT1", 20);
        assert_eq!(title, "电量偏低");
        assert!(body.contains("BAT1 电量已降至 20%"));
    }
}
