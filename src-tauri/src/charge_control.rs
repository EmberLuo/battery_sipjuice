//! 充电阈值控制 — 实验性功能。
//!
//! 接口: /sys/class/power_supply/<bat>/charge_control_{start,end}_threshold
//!
//! ⚠️ 重要说明（诚实披露，勿删）:
//!   1. 本机 (qcom-battmgr) 暴露了这两个文件，但写入是否被固件真正执行
//!      **尚未经过实测验证**。值可能写进去却不生效。
//!   2. 文件属主为 root (0644)，普通用户无写权限。写入需要 root（pkexec/sudo），
//!      或预先放置 udev/polkit 规则放开权限。
//!   3. 因此本模块默认仅“读取 + 尝试写入并如实报告结果”，不静默吞掉失败，
//!      也不在 UI 上谎称成功。前端将其标注为“实验性”。

use serde::Serialize;
use std::fs;
use std::io::Write;

const PS_BASE: &str = "/sys/class/power_supply";
const END_ATTR: &str = "charge_control_end_threshold";
const START_ATTR: &str = "charge_control_start_threshold";

fn path(dev: &str, attr: &str) -> String {
    format!("{PS_BASE}/{dev}/{attr}")
}

fn read_i64(dev: &str, attr: &str) -> Option<i64> {
    fs::read_to_string(path(dev, attr))
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// 该文件当前用户是否可写（用于 UI 判断是否需要提权）。
fn writable(dev: &str, attr: &str) -> bool {
    // 以追加模式尝试打开但不写入任何字节，立即关闭；仅探测权限。
    fs::OpenOptions::new()
        .write(true)
        .open(path(dev, attr))
        .is_ok()
}

#[derive(Serialize, Default, Clone)]
pub struct ChargeControl {
    pub supported: bool,      // 两个 sysfs 文件是否存在
    pub end_threshold: i64,   // 当前封顶值 (%)，0 通常表示未设限
    pub start_threshold: i64, // 当前恢复充电值 (%)
    pub writable: bool,       // 当前进程是否有写权限
    pub experimental_note: String,
}

pub fn status(dev: &str) -> ChargeControl {
    let end_exists = std::path::Path::new(&path(dev, END_ATTR)).exists();
    let start_exists = std::path::Path::new(&path(dev, START_ATTR)).exists();
    let supported = end_exists && start_exists;

    ChargeControl {
        supported,
        end_threshold: read_i64(dev, END_ATTR).unwrap_or(0),
        start_threshold: read_i64(dev, START_ATTR).unwrap_or(0),
        writable: supported && writable(dev, END_ATTR),
        experimental_note: if supported {
            "接口存在，但本机固件是否真正执行充电封顶尚未实测验证。属实验性功能。".into()
        } else {
            "本设备未暴露充电阈值接口。".into()
        },
    }
}

/// 校验阈值取值（借鉴 TLP：end 必须 > start，范围 0..=100）。
fn validate(start: i64, end: i64) -> Result<(), String> {
    if !(0..=100).contains(&start) || !(0..=100).contains(&end) {
        return Err("阈值必须在 0–100 之间".into());
    }
    if end == 0 {
        return Err("封顶值不能为 0（0 表示取消限制，请用 clear 操作）".into());
    }
    if start >= end {
        return Err(format!("恢复充电值({start}) 必须小于封顶值({end})"));
    }
    Ok(())
}

/// 尝试写入阈值。如实返回结果，权限不足时不掩盖。
///
/// 注意: 本函数确实会写 sysfs（若有权限），从而改变充电行为。
/// 仅在前端用户明确开启实验开关并确认时调用。
pub fn apply(dev: &str, start: i64, end: i64) -> Result<String, String> {
    validate(start, end)?;
    let st = status(dev);
    if !st.supported {
        return Err("本设备不支持充电阈值控制".into());
    }
    if !st.writable {
        return Err(format!(
            "无写入权限：{} 属 root 所有。需以 root 运行或配置 polkit/udev 规则后重试。",
            path(dev, END_ATTR)
        ));
    }
    // 先写 start 再写 end（部分驱动要求 start ≤ end 始终成立）。
    write_one(dev, START_ATTR, start)?;
    write_one(dev, END_ATTR, end)?;
    Ok(format!(
        "已写入：恢复 {start}% / 封顶 {end}%（请观察实际充电是否在封顶处停止以确认是否生效）"
    ))
}

fn write_one(dev: &str, attr: &str, val: i64) -> Result<(), String> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .open(path(dev, attr))
        .map_err(|e| format!("打开 {attr} 失败: {e}"))?;
    write!(f, "{val}").map_err(|e| format!("写入 {attr} 失败: {e}"))?;
    Ok(())
}
