//! 电池养护提醒 — 纯软件、全设备通用的可选提醒。
//!
//! 这里只按阈值弹系统通知，不触碰任何硬件接口，
//! 因此在能上报对应电量或温度字段的 Linux 设备上都能用。四个提醒彼此独立、各有开关:
//!   · 低电量(放电中) → 提醒接上电源
//!   · 高电量(充电中) → 提醒拔掉电源，避免长期满电浮充加速老化
//!   · 高温 → 提醒散热，高温是电池老化的首要因素
//!   · 低温 → 提醒回暖，低温下充电可能损伤电池
//!
//! 边沿触发: 进入阈值区间时只提醒一次，离开后复位，避免每次采样都重复打扰。
//! 温度判定带 1°C 回差，避免温度在阈值附近抖动时反复提醒。

use crate::battery;
use crate::settings::Settings;
use std::collections::HashSet;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// 记录各提醒是否已发出(待复位)。由后台采样线程独占持有，故无需加锁。
#[derive(Default)]
pub struct Fired {
    unplug: HashSet<String>,
    charge: HashSet<String>,
    temp_high: HashSet<String>,
    temp_low: HashSet<String>,
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
    if !s.remind_temp_high {
        fired.temp_high.clear();
    }
    if !s.remind_temp_low {
        fired.temp_low.clear();
    }
    if !s.remind_charge && !s.remind_unplug && !s.remind_temp_high && !s.remind_temp_low {
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
    fired
        .temp_high
        .retain(|device| active_devices.contains(device.as_str()));
    fired
        .temp_low
        .retain(|device| active_devices.contains(device.as_str()));

    for battery in batteries {
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

        if let Some(capacity) = battery.capacity {
            let status = battery.status.as_deref();
            let plugged = matches!(status, Some("Charging") | Some("Full"));
            let discharging = status == Some("Discharging");

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
        } else {
            // 电量字段暂时不可用时不保留旧边沿状态；温度提醒仍独立执行。
            fired.unplug.remove(&battery.device);
            fired.charge.remove(&battery.device);
        }

        for (title, body) in
            temperature_notifications(s, &battery.device, battery.temperature, &label, fired)
        {
            notify(app, &title, &body);
        }
    }
}

/// 高/低温提醒与电量、充放电状态完全独立。带 1°C 回差，避免阈值附近抖动。
fn temperature_notifications(
    s: &Settings,
    device: &str,
    temperature: Option<f64>,
    label: &str,
    fired: &mut Fired,
) -> Vec<(String, String)> {
    let Some(temp) = temperature else {
        return Vec::new();
    };
    let mut notifications = Vec::with_capacity(1);

    let high_at = s.remind_temp_high_at as f64;
    if s.remind_temp_high && temp >= high_at {
        if fired.temp_high.insert(device.to_string()) {
            notifications.push(high_temp_text(&s.language, label, temp));
        }
    } else if temp < high_at - 1.0 {
        fired.temp_high.remove(device);
    }

    let low_at = s.remind_temp_low_at as f64;
    if s.remind_temp_low && temp <= low_at {
        if fired.temp_low.insert(device.to_string()) {
            notifications.push(low_temp_text(&s.language, label, temp));
        }
    } else if temp > low_at + 1.0 {
        fired.temp_low.remove(device);
    }

    notifications
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

fn high_temp_text(language: &str, label: &str, temp: f64) -> (String, String) {
    if language == "en-US" {
        (
            "Battery Temperature High".to_string(),
            format!(
                "{label} has reached {temp:.1}°C. Reduce load and keep the device away from heat sources; high temperature accelerates battery aging."
            ),
        )
    } else {
        (
            "电池温度偏高".to_string(),
            format!("{label} 温度已达 {temp:.1}°C，建议降低负载并远离热源，高温会加速电池老化。"),
        )
    }
}

fn low_temp_text(language: &str, label: &str, temp: f64) -> (String, String) {
    if language == "en-US" {
        (
            "Battery Temperature Low".to_string(),
            format!(
                "{label} has dropped to {temp:.1}°C. Charging a cold battery can damage it; let the device warm up first."
            ),
        )
    } else {
        (
            "电池温度偏低".to_string(),
            format!("{label} 温度已降至 {temp:.1}°C，低温下充电可能损伤电池，建议先让设备回暖。"),
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

        let (title, body) = high_temp_text("en-US", "BAT0", 45.3);
        assert_eq!(title, "Battery Temperature High");
        assert!(body.contains("BAT0 has reached 45.3°C"));

        let (title, body) = low_temp_text("zh-CN", "BAT1", 4.8);
        assert_eq!(title, "电池温度偏低");
        assert!(body.contains("BAT1 温度已降至 4.8°C"));
    }

    #[test]
    fn temperature_reminders_are_independent_and_hysteretic() {
        let settings = Settings::default();
        let mut fired = Fired::default();

        let first = temperature_notifications(&settings, "BAT0", Some(46.0), "BAT0", &mut fired);
        assert_eq!(first.len(), 1, "无需电量字段即可单独判定温度");
        assert!(first[0].0.contains("电池温度偏高"));

        let repeated = temperature_notifications(&settings, "BAT0", Some(45.5), "BAT0", &mut fired);
        assert!(repeated.is_empty(), "保持在高温区间时不应重复提醒");

        temperature_notifications(&settings, "BAT0", Some(43.9), "BAT0", &mut fired);
        let after_reset =
            temperature_notifications(&settings, "BAT0", Some(45.0), "BAT0", &mut fired);
        assert_eq!(after_reset.len(), 1, "回落超过 1°C 后应允许再次触发");
    }
}
