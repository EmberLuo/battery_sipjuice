//! 通用电源输入采集 — 扫描 Linux power_supply 里的非电池设备。
//!
//! 不再硬编码 qcom-battmgr-usb / qcom-battmgr-wls；这些只是普通 power_supply
//! 设备的一种命名。通用逻辑根据 `type` 分类 USB / Mains / Wireless 等输入源。

use crate::battery;
use serde::Serialize;
use std::path::Path;

const PS_BASE: &str = "/sys/class/power_supply";

#[derive(Serialize, Clone)]
pub struct PowerSource {
    pub name: String,
    pub kind: String,
    pub online: Option<bool>,
    pub voltage_now: Option<f64>, // V
    pub current_now: Option<f64>, // mA
    pub current_max: Option<f64>, // mA
    pub power_now: Option<f64>,   // W
    pub usb_type: Option<String>,
}

/// 采集全部输入电源；返回所有已知接口（不论是否在线）。
pub fn collect() -> Vec<PowerSource> {
    battery::list_devices()
        .into_iter()
        .filter_map(|dev| collect_one(&dev))
        .collect()
}

fn collect_one(dev: &str) -> Option<PowerSource> {
    if !Path::new(&format!("{PS_BASE}/{dev}")).exists() {
        return None;
    }
    let kind = battery::read_raw(dev, "type")?;
    if kind == "Battery" {
        return None;
    }

    let voltage_now = battery::read_i64(dev, "voltage_now").map(|v| v as f64 / 1_000_000.0);
    let current_now = battery::read_i64(dev, "current_now").map(|v| v as f64 / 1000.0);
    let power_now = battery::read_i64(dev, "power_now")
        .map(|v| v as f64 / 1_000_000.0)
        .or_else(|| Some(voltage_now? * current_now? / 1000.0));

    Some(PowerSource {
        name: dev.to_string(),
        kind,
        online: battery::read_i64(dev, "online").map(|v| v == 1),
        voltage_now,
        current_now,
        current_max: battery::read_i64(dev, "current_max").map(|v| v as f64 / 1000.0),
        power_now,
        usb_type: battery::read_raw(dev, "usb_type").and_then(|s| parse_usb_type(&s)),
    })
}

/// usb_type 形如 "[Unknown] SDP DCP CDP ... PD"，方括号内为当前选中项。
fn parse_usb_type(raw: &str) -> Option<String> {
    if let (Some(s), Some(e)) = (raw.find('['), raw.find(']')) {
        if e > s + 1 {
            let val = raw[s + 1..e].to_string();
            return (!val.is_empty()).then_some(val);
        }
    }
    None
}
