//! 通用电池采集 — 读取 Linux power_supply sysfs 并归一化。
//!
//! 设计原则:
//! - 优先使用 Linux 统一接口 `/sys/class/power_supply`，不围绕某个机型建模。
//! - 字段缺失用 `Option` 表达，避免把“不支持”误显示为 0。
//! - 同时兼容手机/平板常见的 `charge_*`(µAh) 与笔记本常见的 `energy_*`(µWh)。
//! - 专有接口只作为能力增强，不能污染通用数据模型。

use serde::Serialize;
use std::fs;

const PS_BASE: &str = "/sys/class/power_supply";

fn path(dev: &str, attr: &str) -> String {
    format!("{PS_BASE}/{dev}/{attr}")
}

/// 读取某设备的 sysfs 属性并去除首尾空白。
pub fn read_raw(dev: &str, attr: &str) -> Option<String> {
    fs::read_to_string(path(dev, attr))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 读取整数型属性。sysfs 中通常是 µA / µV / µAh / µWh / 0.1°C。
pub fn read_i64(dev: &str, attr: &str) -> Option<i64> {
    read_raw(dev, attr)?.parse().ok()
}

/// 枚举所有 power_supply 设备名。
pub fn list_devices() -> Vec<String> {
    let mut names: Vec<String> = match fs::read_dir(PS_BASE) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names
}

/// 枚举所有电池设备名。
pub fn list_batteries() -> Vec<String> {
    list_devices()
        .into_iter()
        .filter(|n| read_raw(n, "type").as_deref() == Some("Battery"))
        .collect()
}

/// 返回主电池设备名。当前 UI 仍显示单电池；后端已经支持枚举多个电池。
pub fn find_battery() -> Option<String> {
    list_batteries().into_iter().next()
}

#[derive(Serialize, Clone)]
pub struct CapacityValue {
    /// 用户可读数值。charge 系列为 mAh，energy 系列为 Wh。
    pub value: f64,
    /// `mAh` 或 `Wh`。
    pub unit: String,
    /// 原始字段名前缀: `charge` / `energy`。
    pub source_kind: String,
}

#[derive(Serialize, Clone)]
pub struct BatteryInfo {
    pub device: String,
    pub model: Option<String>,
    pub manufacturer: Option<String>,
    pub technology: Option<String>,
    pub present: Option<bool>,
    pub capacity: Option<i64>,
    pub status: Option<String>,
    pub health_status: Option<String>,

    // 健康 / 容量。charge 与 energy 保留原始归一化后的单位，不强行互转。
    pub full_capacity: Option<CapacityValue>,
    pub design_capacity: Option<CapacityValue>,
    pub health_percent: Option<f64>,
    pub cycle_count: Option<i64>,
    pub state_of_health: Option<i64>,

    // 实时电气量。
    pub voltage_now: Option<f64>,         // V
    pub voltage_ocv: Option<f64>,         // V
    pub voltage_max: Option<f64>,         // V
    pub current_now: Option<f64>,         // mA (负=放电, 正=充电)
    pub power_now: Option<f64>,           // W
    pub temperature: Option<f64>,         // °C
    pub internal_resistance: Option<f64>, // mΩ

    // 时间估算 (分钟)。没有足够数据时为 None。
    pub time_to_empty_min: Option<i64>,
    pub time_to_full_min: Option<i64>,
}

/// 采集主电池快照。无电池设备时返回 None。
pub fn collect() -> Option<BatteryInfo> {
    collect_device(&find_battery()?)
}

/// 采集指定电池设备并归一化。
pub fn collect_device(dev: &str) -> Option<BatteryInfo> {
    if read_raw(dev, "type").as_deref() != Some("Battery") {
        return None;
    }

    let full_capacity = read_full_capacity(dev);
    let design_capacity =
        read_design_capacity(dev, full_capacity.as_ref().map(|v| v.source_kind.as_str()));
    let health_percent = health_percent(dev, full_capacity.as_ref(), design_capacity.as_ref());

    let voltage_now = micro_to_unit(read_i64(dev, "voltage_now"));
    let current_now = read_i64(dev, "current_now").map(|v| v as f64 / 1000.0);
    let power_now = read_power(dev, voltage_now, current_now);
    let capacity = read_i64(dev, "capacity");
    let status = read_raw(dev, "status");

    let mut b = BatteryInfo {
        device: dev.to_string(),
        model: read_raw(dev, "model_name"),
        manufacturer: read_raw(dev, "manufacturer"),
        technology: read_raw(dev, "technology"),
        present: read_i64(dev, "present").map(|v| v == 1),
        capacity,
        status,
        health_status: read_raw(dev, "health"),
        full_capacity,
        design_capacity,
        health_percent,
        cycle_count: read_i64(dev, "cycle_count"),
        state_of_health: read_i64(dev, "state_of_health"),
        voltage_now,
        voltage_ocv: micro_to_unit(read_i64(dev, "voltage_ocv")),
        voltage_max: micro_to_unit(read_i64(dev, "voltage_max")),
        current_now,
        power_now,
        temperature: read_i64(dev, "temp").map(|v| v as f64 / 10.0),
        internal_resistance: read_i64(dev, "internal_resistance").map(|v| v as f64 / 1000.0),
        time_to_empty_min: None,
        time_to_full_min: None,
    };

    let (tte, ttf) = estimate_times(&b);
    b.time_to_empty_min = tte;
    b.time_to_full_min = ttf;
    Some(b)
}

fn micro_to_unit(v: Option<i64>) -> Option<f64> {
    v.map(|n| n as f64 / 1_000_000.0)
}

fn read_full_capacity(dev: &str) -> Option<CapacityValue> {
    if let Some(v) = read_i64(dev, "charge_full") {
        return Some(CapacityValue {
            value: v as f64 / 1000.0,
            unit: "mAh".into(),
            source_kind: "charge".into(),
        });
    }
    read_i64(dev, "energy_full").map(|v| CapacityValue {
        value: v as f64 / 1_000_000.0,
        unit: "Wh".into(),
        source_kind: "energy".into(),
    })
}

fn read_design_capacity(dev: &str, preferred: Option<&str>) -> Option<CapacityValue> {
    let charge = || {
        read_i64(dev, "charge_full_design").map(|v| CapacityValue {
            value: v as f64 / 1000.0,
            unit: "mAh".into(),
            source_kind: "charge".into(),
        })
    };
    let energy = || {
        read_i64(dev, "energy_full_design").map(|v| CapacityValue {
            value: v as f64 / 1_000_000.0,
            unit: "Wh".into(),
            source_kind: "energy".into(),
        })
    };

    match preferred {
        Some("energy") => energy().or_else(charge),
        _ => charge().or_else(energy),
    }
}

fn health_percent(
    dev: &str,
    full: Option<&CapacityValue>,
    design: Option<&CapacityValue>,
) -> Option<f64> {
    let (full, design) = (full?, design?);
    if full.unit == design.unit && design.value > 0.0 {
        Some(full.value / design.value * 100.0)
    } else if let Some(hw) = read_i64(dev, "state_of_health") {
        // 仅在缺少可比容量数据时兜底使用厂商/驱动 SOH。
        (1..=100).contains(&hw).then_some(hw as f64)
    } else {
        None
    }
}

fn read_power(dev: &str, voltage_now: Option<f64>, current_now: Option<f64>) -> Option<f64> {
    let raw = read_i64(dev, "power_now").map(|v| v as f64 / 1_000_000.0);
    if raw.is_some_and(|v| v.abs() > 0.01) {
        raw
    } else {
        Some(voltage_now? * current_now? / 1000.0)
    }
}

/// 由剩余容量与瞬时功率估算放电或充电剩余时间。
fn estimate_times(b: &BatteryInfo) -> (Option<i64>, Option<i64>) {
    let Some(capacity) = b.capacity else {
        return (None, None);
    };
    let Some(full) = b.full_capacity.as_ref() else {
        return (None, None);
    };

    let pct = capacity as f64 / 100.0;
    let status = b.status.as_deref().unwrap_or("");

    let (full_amount, rate_per_hour) =
        if let (Some(full_wh), Some(power)) = (full_energy_wh(b), b.power_now.map(f64::abs)) {
            (full_wh, power)
        } else if full.unit == "mAh" {
            let Some(current) = b.current_now.map(f64::abs) else {
                return (None, None);
            };
            (full.value, current)
        } else if full.unit == "Wh" {
            let Some(power) = b.power_now.map(f64::abs) else {
                return (None, None);
            };
            (full.value, power)
        } else {
            return (None, None);
        };
    if rate_per_hour < 0.01 {
        return (None, None);
    }

    let now = pct * full_amount;
    if status == "Charging" {
        let remain = (full_amount - now).max(0.0);
        (None, Some((remain / rate_per_hour * 60.0) as i64))
    } else if status == "Discharging" {
        (Some((now / rate_per_hour * 60.0) as i64), None)
    } else {
        (None, None)
    }
}

fn full_energy_wh(b: &BatteryInfo) -> Option<f64> {
    let full = b.full_capacity.as_ref()?;
    if full.unit == "Wh" {
        Some(full.value)
    } else if full.unit == "mAh" {
        let voltage = b.voltage_now.or(b.voltage_ocv).or(b.voltage_max)?;
        Some(full.value * voltage / 1000.0)
    } else {
        None
    }
}
