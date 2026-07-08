//! 按应用估算耗电 -- CPU 时间占比加权分配电池瞬时功率。
//!
//! 设计原则:
//! - Linux 桌面/平板没有类似 Android BatteryStats 的逐应用耗电统计，RAPL 又只有
//!   x86 才有（这台设备要兼容 ARM 平板），所以用与 PowerTOP/Scaphandre 相同思路的
//!   CPU-jiffies 加权估算：应用在采样区间内占用的 CPU 时间比例 × 电池瞬时功率
//!   (battery_power_w) ≈ 该应用的当前功率估算值。这是估算，不是精确硬件测量。
//! - 采样在后台线程按 SAMPLE_INTERVAL 周期进行，两次快照的 utime+stime 差值
//!   即为区间内的 CPU 时间消耗，除以 /proc/stat 总 jiffies 差值得到占比。
//! - 状态只由后台线程写入 (tick)，命令层只读取缓存结果 (latest)，用 Mutex 仅为
//!   满足 Tauri State 的 Send+Sync 要求，不存在高频竞争。

use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const PROC_BASE: &str = "/proc";
/// 前端每组展示的最多应用数。
const TOP_APP_COUNT: usize = 8;
/// 单应用 CPU tick 占比低于此比例视为噪声，不纳入当前功率展示。
const MIN_CPU_SHARE: f64 = 0.005;
const MIN_ENERGY_WH: f64 = 0.00001;

#[derive(Serialize, Clone)]
pub struct AppPowerEstimate {
    pub name: String,
    pub power_w: f64,
    pub energy_wh: f64,
    pub cpu_share: f64, // 0.0 ~ 1.0，占采样区间总 CPU 时间的比例
    pub process_count: usize,
}

#[derive(Serialize, Clone, Default)]
pub struct AppPowerReport {
    pub total_energy: Vec<AppPowerEstimate>,
    pub current_power: Vec<AppPowerEstimate>,
}

/// 一次快照: 每个 pid 的累计 CPU ticks (utime+stime) + 系统总 ticks。
struct ProcSnapshot {
    ticks: HashMap<i32, u64>,
    total: u64,
    timestamp_ms: u64,
}

#[derive(Default)]
struct EnergyAccumulator {
    energy_wh: f64,
}

#[derive(Default)]
struct StoreInner {
    prev: Option<ProcSnapshot>,
    energy_by_app: HashMap<String, EnergyAccumulator>,
    latest: AppPowerReport,
}

#[derive(Default)]
pub struct AppPowerStore {
    inner: Mutex<StoreInner>,
}

impl AppPowerStore {
    /// 采一次快照并与上次比较，更新缓存的估算结果。由后台线程定时调用。
    pub fn tick(&self, battery_power_w: Option<f64>) {
        let snap = collect_snapshot();
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };

        if let (Some(prev), Some(power_w)) = (inner.prev.as_ref(), battery_power_w) {
            let total_delta = snap.total.saturating_sub(prev.total);
            let elapsed_ms = snap.timestamp_ms.saturating_sub(prev.timestamp_ms);
            if total_delta > 0 && elapsed_ms > 0 {
                let mut current_power = rank_app_power(prev, &snap, total_delta, power_w.abs());
                let hours = elapsed_ms as f64 / 3_600_000.0;

                for app in &mut current_power {
                    let entry = inner.energy_by_app.entry(app.name.clone()).or_default();
                    entry.energy_wh += app.power_w * hours;
                    app.energy_wh = entry.energy_wh;
                }

                let current_power_by_name = current_power
                    .iter()
                    .map(|app| (app.name.clone(), app.clone()))
                    .collect::<HashMap<_, _>>();
                let mut total_energy = inner
                    .energy_by_app
                    .iter()
                    .filter_map(|(name, entry)| {
                        if entry.energy_wh < MIN_ENERGY_WH {
                            return None;
                        }
                        let current = current_power_by_name.get(name);
                        Some(AppPowerEstimate {
                            name: name.clone(),
                            power_w: current.map(|app| app.power_w).unwrap_or(0.0),
                            energy_wh: entry.energy_wh,
                            cpu_share: current.map(|app| app.cpu_share).unwrap_or(0.0),
                            process_count: current.map(|app| app.process_count).unwrap_or(0),
                        })
                    })
                    .collect::<Vec<_>>();
                total_energy.sort_by(|a, b| b.energy_wh.total_cmp(&a.energy_wh));
                total_energy.truncate(TOP_APP_COUNT);
                current_power.sort_by(|a, b| b.power_w.total_cmp(&a.power_w));
                current_power.truncate(TOP_APP_COUNT);

                inner.latest = AppPowerReport {
                    total_energy,
                    current_power,
                };
            }
        }
        inner.prev = Some(snap);
    }

    /// 取当前缓存的估算结果，供前端命令读取。
    pub fn latest(&self) -> AppPowerReport {
        self.inner
            .lock()
            .map(|i| i.latest.clone())
            .unwrap_or_default()
    }
}

#[derive(Default)]
struct AppCpuBucket {
    ticks: u64,
    process_count: usize,
}

fn rank_app_power(
    prev: &ProcSnapshot,
    cur: &ProcSnapshot,
    total_delta: u64,
    battery_power_w: f64,
) -> Vec<AppPowerEstimate> {
    let mut buckets = HashMap::<String, AppCpuBucket>::new();
    for (&pid, &ticks) in &cur.ticks {
        let before = prev.ticks.get(&pid).copied().unwrap_or(0);
        let delta = ticks.saturating_sub(before);
        if delta == 0 {
            continue;
        }
        let bucket = buckets.entry(app_group_name(pid)).or_default();
        bucket.ticks += delta;
        bucket.process_count += 1;
    }

    buckets
        .into_iter()
        .filter_map(|(name, bucket)| {
            let share = bucket.ticks as f64 / total_delta as f64;
            if share < MIN_CPU_SHARE {
                return None;
            }
            Some(AppPowerEstimate {
                name,
                power_w: share * battery_power_w,
                energy_wh: 0.0,
                cpu_share: share,
                process_count: bucket.process_count,
            })
        })
        .collect()
}

/// 遍历 /proc 下的数字目录，读取每个进程的 utime+stime，并累加系统总 ticks。
fn collect_snapshot() -> ProcSnapshot {
    let mut ticks = HashMap::new();
    if let Ok(entries) = fs::read_dir(PROC_BASE) {
        for entry in entries.filter_map(|e| e.ok()) {
            let Ok(pid) = entry.file_name().to_string_lossy().parse::<i32>() else {
                continue;
            };
            if let Some(t) = read_proc_ticks(pid) {
                ticks.insert(pid, t);
            }
        }
    }
    ProcSnapshot {
        ticks,
        total: read_total_ticks().unwrap_or(0),
        timestamp_ms: now_ms(),
    }
}

/// 读取单进程的 utime+stime (ticks)。comm 字段可能含空格/括号，从最后一个 ')' 定位。
fn read_proc_ticks(pid: i32) -> Option<u64> {
    let raw = fs::read_to_string(format!("{PROC_BASE}/{pid}/stat")).ok()?;
    let after_comm = raw.rsplit_once(')')?.1;
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    // after_comm: state(0) ppid(1) ... cmajflt(10) utime(11) stime(12) ...
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime + stime)
}

/// /proc/stat 第一行 "cpu  user nice system idle iowait irq softirq steal ..."。
/// 只累加前 8 个字段；guest/guest_nice 已被计入 user/nice，不能重复累加。
fn read_total_ticks() -> Option<u64> {
    let raw = fs::read_to_string(format!("{PROC_BASE}/stat")).ok()?;
    let line = raw.lines().next()?;
    let sum: u64 = line
        .split_whitespace()
        .skip(1) // "cpu" 标签
        .take(8)
        .filter_map(|v| v.parse::<u64>().ok())
        .sum();
    Some(sum)
}

fn app_group_name(pid: i32) -> String {
    let cmdline = fs::read(format!("{PROC_BASE}/{pid}/cmdline"))
        .ok()
        .map(parse_cmdline)
        .unwrap_or_default();
    let exe_name = cmdline.first().and_then(|arg| basename(arg));
    let comm = fs::read_to_string(format!("{PROC_BASE}/{pid}/comm"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let raw = exe_name
        .or(comm.clone())
        .unwrap_or_else(|| format!("pid {pid}"));
    normalize_app_group_name(&raw, &cmdline, comm.as_deref())
}

fn parse_cmdline(bytes: Vec<u8>) -> Vec<String> {
    bytes
        .split(|b| *b == 0)
        .filter(|part| !part.is_empty())
        .filter_map(|part| String::from_utf8(part.to_vec()).ok())
        .collect()
}

fn basename(path: &str) -> Option<String> {
    let trimmed = path.split_whitespace().next().unwrap_or(path);
    Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .filter(|name| !name.is_empty())
}

fn normalize_app_group_name(raw: &str, cmdline: &[String], comm: Option<&str>) -> String {
    let name = raw.trim();
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "code" | "chrome_crashpad_handler" => "Code".to_string(),
        "battery-sipjuice" | "battery-sipjuic" => "Battery SipJuice".to_string(),
        "gnome-shell" => "GNOME Shell".to_string(),
        "gjs" if cmdline.iter().any(|arg| arg.contains("gnome-shell")) => "GNOME Shell".to_string(),
        "webkitwebprocess" | "webkitwebproces" => {
            infer_webkit_app_name(cmdline).unwrap_or_else(|| {
                comm.map(|s| s.to_string())
                    .unwrap_or_else(|| "WebKitWebProcess".to_string())
            })
        }
        _ => clean_title(name),
    }
}

fn infer_webkit_app_name(cmdline: &[String]) -> Option<String> {
    if cmdline.iter().any(|arg| arg.contains("battery-sipjuice")) {
        Some("Battery SipJuice".to_string())
    } else {
        None
    }
}

fn clean_title(name: &str) -> String {
    let token = name.split_whitespace().next().unwrap_or(name);
    token
        .split(['/', '\\'])
        .next_back()
        .unwrap_or(token)
        .trim_matches(|c: char| c == '[' || c == ']')
        .to_string()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
