//! Tauri 命令层 — 暴露给前端的 IPC 接口。

use crate::{battery, charge_control, power};
use serde::Serialize;

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
    let cc = bat
        .as_ref()
        .map(|b| charge_control::status(&b.device));
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
