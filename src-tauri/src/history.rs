//! 历史趋势采集 — 定时采样电池快照，存入定长 RRD 风格环形归档，供前端画曲线。
//!
//! 设计 (round-robin / 多分辨率归档，借鉴 RRDtool 思想):
//! - 两级归档，文件大小固定 (~363KB)，不随时间增长、无需压实扫描:
//!     · fine   : 30s 步长 × 2880 槽 = 最近 24 小时
//!     · coarse : 5min 步长 × 2016 槽 = 最近 7 天 (写入时按平均合并)
//! - 写入即合并 (consolidate-on-write): 每个时间桶对落入的样本累加 sum/count，
//!   查询时算平均；app 关闭期间不采样，未写入的槽视为 None → 前端断线。
//! - 存储格式为定长小端二进制 (history.rdb)，写入用临时文件 + rename 原子替换。
//! - 旧版 history.jsonl 存在时自动迁移到归档并改名为 .bak。
//! - query 选合适分辨率的归档，再降采样到约 MAX_POINTS 个点，避免前端绘制过密。

use crate::battery;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// 保留时长: 7 天 (毫秒)。
const RETENTION_MS: u64 = 7 * 24 * 60 * 60 * 1000;
/// 查询降采样目标点数。
const MAX_POINTS: usize = 240;

/// fine 归档: 30s 步长，保留 24 小时。
const STEP_FINE_MS: u64 = 30_000;
const ROWS_FINE: usize = 2880;
/// coarse 归档: 5min 步长，保留 7 天。
const STEP_COARSE_MS: u64 = 300_000;
const ROWS_COARSE: usize = 2016;

/// 每槽二进制大小: t(8)+n(4)+chg_n(4)+5×(sum f64(8)+n u32(4)) = 76 字节。
const ROW_BYTES: usize = 8 + 4 + 4 + 5 * (8 + 4);
/// 文件魔数，用于校验。
const MAGIC: &[u8; 4] = b"BSR1";

/// 落盘节流: 每 N 次采样落盘一次 (10 × 30s ≈ 5 分钟)，降低写放大。
/// 内存归档始终是查询来源，崩溃至多丢失最近 ~5 分钟的细粒度点。
const FLUSH_EVERY: u32 = 10;

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 单个历史采样点 (查询输出)。字段名缩短以贴合前端既有约定。
#[derive(Serialize, Deserialize, Clone)]
pub struct Sample {
    pub t: u64,            // unix 毫秒
    pub cap: Option<i64>,  // 电量 %
    pub temp: Option<f64>, // 温度 °C
    pub pow: Option<f64>,  // 功率 W (带符号: 放电为负, 充电为正)
    pub volt: Option<f64>, // 电压 V
    pub curr: Option<f64>, // 电流 mA (带符号)
    pub chg: bool,         // 是否充电中 (桶内多数表决)
}

/// 可空字段的均值累加器: 忽略 None，记录 sum 与样本数。
#[derive(Clone, Copy, Default)]
struct Acc {
    sum: f64,
    cnt: u32,
}

impl Acc {
    #[inline]
    fn add(&mut self, v: Option<f64>) {
        if let Some(v) = v {
            self.sum += v;
            self.cnt += 1;
        }
    }
    #[inline]
    fn avg(&self) -> Option<f64> {
        (self.cnt > 0).then(|| self.sum / self.cnt as f64)
    }
}

/// 环形归档中的一个时间桶 (consolidate-on-write 累加器)。
#[derive(Clone, Copy, Default)]
struct Slot {
    t: u64,     // 桶对齐起始时间 (毫秒); 0 表示空槽
    n: u32,     // 落入该桶的样本总数
    chg_n: u32, // 其中处于充电状态的样本数
    cap: Acc,
    temp: Acc,
    pow: Acc,
    volt: Acc,
    curr: Acc,
}

impl Slot {
    /// 转为查询输出样本; 空槽返回 None。
    fn to_sample(&self) -> Option<Sample> {
        if self.n == 0 {
            return None;
        }
        Some(Sample {
            t: self.t,
            cap: self.cap.avg().map(|v| v.round() as i64),
            temp: self.temp.avg(),
            pow: self.pow.avg(),
            volt: self.volt.avg(),
            curr: self.curr.avg(),
            chg: self.chg_n * 2 > self.n, // 桶内多数表决
        })
    }

    fn write_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.t.to_le_bytes());
        out.extend_from_slice(&self.n.to_le_bytes());
        out.extend_from_slice(&self.chg_n.to_le_bytes());
        for a in [&self.cap, &self.temp, &self.pow, &self.volt, &self.curr] {
            out.extend_from_slice(&a.sum.to_le_bytes());
            out.extend_from_slice(&a.cnt.to_le_bytes());
        }
    }

    /// 从恰好 ROW_BYTES 长的切片解析一个槽。
    fn read_bytes(buf: &[u8]) -> Slot {
        let u64a = |o: usize| u64::from_le_bytes(buf[o..o + 8].try_into().unwrap());
        let u32a = |o: usize| u32::from_le_bytes(buf[o..o + 4].try_into().unwrap());
        let f64a = |o: usize| f64::from_le_bytes(buf[o..o + 8].try_into().unwrap());
        let mut s = Slot {
            t: u64a(0),
            n: u32a(8),
            chg_n: u32a(12),
            ..Slot::default()
        };
        let mut o = 16;
        for a in [
            &mut s.cap,
            &mut s.temp,
            &mut s.pow,
            &mut s.volt,
            &mut s.curr,
        ] {
            a.sum = f64a(o);
            a.cnt = u32a(o + 8);
            o += 12;
        }
        s
    }
}

/// 单级定长环形归档: step 步长, 固定 rows 个槽。
/// 槽索引 = (桶对齐时间 / step) % rows —— 同一索引在 rows 个桶后被新桶覆盖，
/// 天然实现 round-robin 滚动，无需显式裁剪。
struct Archive {
    step: u64,
    slots: Vec<Slot>,
}

impl Archive {
    fn new(step: u64, rows: usize) -> Self {
        Archive {
            step,
            slots: vec![Slot::default(); rows],
        }
    }

    /// 把一个样本并入对应时间桶 (consolidate-on-write)。
    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        t: u64,
        cap: Option<f64>,
        temp: Option<f64>,
        pow: Option<f64>,
        volt: Option<f64>,
        curr: Option<f64>,
        chg: bool,
    ) {
        let bt = t - (t % self.step);
        let idx = (bt / self.step) as usize % self.slots.len();
        let slot = &mut self.slots[idx];
        if slot.t != bt {
            // 该索引上残留的是已滚出的旧桶，重置后归入新桶。
            *slot = Slot {
                t: bt,
                ..Slot::default()
            };
        }
        slot.n += 1;
        if chg {
            slot.chg_n += 1;
        }
        slot.cap.add(cap);
        slot.temp.add(temp);
        slot.pow.add(pow);
        slot.volt.add(volt);
        slot.curr.add(curr);
    }

    /// 时间 >= cutoff 且未超出保留期的样本，按时间升序。
    fn samples_since(&self, cutoff: u64, now: u64) -> Vec<Sample> {
        let oldest = now.saturating_sub(RETENTION_MS);
        let mut out: Vec<Sample> = self
            .slots
            .iter()
            .filter(|s| s.t >= cutoff && s.t >= oldest)
            .filter_map(Slot::to_sample)
            .collect();
        out.sort_by_key(|s| s.t);
        out
    }
}

/// 两级归档容器: 一份样本同时并入 fine 与 coarse。
struct Archives {
    fine: Archive,
    coarse: Archive,
}

impl Archives {
    fn new() -> Self {
        Archives {
            fine: Archive::new(STEP_FINE_MS, ROWS_FINE),
            coarse: Archive::new(STEP_COARSE_MS, ROWS_COARSE),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        t: u64,
        cap: Option<f64>,
        temp: Option<f64>,
        pow: Option<f64>,
        volt: Option<f64>,
        curr: Option<f64>,
        chg: bool,
    ) {
        self.fine.add(t, cap, temp, pow, volt, curr, chg);
        self.coarse.add(t, cap, temp, pow, volt, curr, chg);
    }

    /// 序列化为定长二进制: MAGIC + fine 槽 + coarse 槽。
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(MAGIC.len() + (ROWS_FINE + ROWS_COARSE) * ROW_BYTES);
        out.extend_from_slice(MAGIC);
        for s in &self.fine.slots {
            s.write_bytes(&mut out);
        }
        for s in &self.coarse.slots {
            s.write_bytes(&mut out);
        }
        out
    }

    /// 从二进制还原; 魔数或长度不符则返回 None (当作无历史)。
    fn from_bytes(buf: &[u8]) -> Option<Archives> {
        let expected = MAGIC.len() + (ROWS_FINE + ROWS_COARSE) * ROW_BYTES;
        if buf.len() != expected || &buf[..MAGIC.len()] != MAGIC {
            return None;
        }
        let mut a = Archives::new();
        let mut o = MAGIC.len();
        for slot in a.fine.slots.iter_mut().chain(a.coarse.slots.iter_mut()) {
            *slot = Slot::read_bytes(&buf[o..o + ROW_BYTES]);
            o += ROW_BYTES;
        }
        Some(a)
    }
}

pub struct HistoryStore {
    path: PathBuf,
    archives: Mutex<Archives>,
    writes: AtomicU32,
}

impl HistoryStore {
    /// 从磁盘载入归档; 不存在则尝试迁移同目录旧版 history.jsonl, 否则空归档。
    pub fn load(path: PathBuf) -> Self {
        let jsonl = path.with_extension("jsonl");
        let (archives, migrated) = match fs::read(&path)
            .ok()
            .and_then(|buf| Archives::from_bytes(&buf))
        {
            Some(a) => (a, false),
            None => migrate_legacy_jsonl(&jsonl),
        };
        let store = HistoryStore {
            path,
            archives: Mutex::new(archives),
            writes: AtomicU32::new(0),
        };
        if migrated {
            // 仅在新格式确实落盘成功后才归档旧文件，避免写盘失败时两边都丢。
            if store.flush() {
                let _ = fs::rename(&jsonl, jsonl.with_extension("jsonl.bak"));
            } else {
                eprintln!("history: 迁移后落盘失败，保留旧 history.jsonl 待下次重试");
            }
        }
        store
    }

    /// 采样当前电池状态并并入归档。由后台线程定时调用。
    pub fn tick(&self) {
        let Some(b) = battery::collect() else {
            return;
        };
        let chg = b.status.as_deref() == Some("Charging");
        // 功率带符号: 放电(Discharging)为负，便于曲线区分方向。
        let pow = b.power_now.map(|p| if chg { p.abs() } else { -p.abs() });
        let t = now_ms();

        if let Ok(mut a) = self.archives.lock() {
            a.add(
                t,
                b.capacity.map(|v| v as f64),
                b.temperature,
                pow,
                b.voltage_now,
                b.current_now,
                chg,
            );
        }

        // 节流落盘: 内存归档是查询来源，磁盘只需周期性持久化。
        if self.writes.fetch_add(1, Ordering::Relaxed) % FLUSH_EVERY == 0 {
            self.flush();
        }
    }

    /// 原子落盘: 写临时文件后 rename 替换，避免写一半被读到。
    /// 返回是否成功，供迁移流程据此决定是否归档旧文件。
    fn flush(&self) -> bool {
        let bytes = {
            let Ok(a) = self.archives.lock() else {
                return false;
            };
            a.to_bytes()
        }; // 锁在此释放，再写盘
        let tmp = self.path.with_extension("rdb.tmp");
        if let Err(e) = fs::write(&tmp, &bytes) {
            eprintln!("history: 落盘写临时文件失败: {e}");
            return false;
        }
        if let Err(e) = fs::rename(&tmp, &self.path) {
            eprintln!("history: 落盘 rename 失败: {e}");
            let _ = fs::remove_file(&tmp);
            return false;
        }
        true
    }

    /// 查询最近 range_ms 内的样本，自动选分辨率并降采样到约 MAX_POINTS 点。
    pub fn query(&self, range_ms: u64) -> Vec<Sample> {
        let now = now_ms();
        let cutoff = now.saturating_sub(range_ms);
        let Ok(a) = self.archives.lock() else {
            return Vec::new();
        };
        // 范围超出 fine 归档覆盖 (24h) 时用 coarse, 否则用 fine。
        let src = if range_ms > STEP_FINE_MS * ROWS_FINE as u64 {
            &a.coarse
        } else {
            &a.fine
        };
        let samples = src.samples_since(cutoff, now);
        downsample(samples)
    }
}

/// 迁移旧版 JSON-lines 历史到归档。返回 (归档, 是否确有数据被迁移)。
/// 实际的旧文件改名由调用方在落盘成功后执行，以免中途崩溃丢数据。
fn migrate_legacy_jsonl(path: &Path) -> (Archives, bool) {
    let mut a = Archives::new();
    let Ok(content) = fs::read_to_string(path) else {
        return (a, false);
    };
    #[derive(Deserialize)]
    struct LegacySample {
        t: u64,
        cap: Option<i64>,
        temp: Option<f64>,
        pow: Option<f64>,
        volt: Option<f64>,
        curr: Option<f64>,
        chg: bool,
    }
    let cutoff = now_ms().saturating_sub(RETENTION_MS);
    let mut migrated = 0u32;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(s) = serde_json::from_str::<LegacySample>(line) {
            if s.t >= cutoff {
                a.add(
                    s.t,
                    s.cap.map(|v| v as f64),
                    s.temp,
                    s.pow,
                    s.volt,
                    s.curr,
                    s.chg,
                );
                migrated += 1;
            }
        }
    }
    (a, migrated > 0)
}

/// 按时间桶平均降采样。点数不超过 MAX_POINTS 时原样返回。
fn downsample(samples: Vec<Sample>) -> Vec<Sample> {
    if samples.len() <= MAX_POINTS {
        return samples;
    }
    let bucket = samples.len().div_ceil(MAX_POINTS);
    let mut out = Vec::with_capacity(MAX_POINTS);
    for chunk in samples.chunks(bucket) {
        out.push(average_bucket(chunk));
    }
    out
}

/// 对一个桶内的样本求平均; 可空字段忽略 None; chg/t 取桶内末值。
fn average_bucket(chunk: &[Sample]) -> Sample {
    let (mut cap, mut temp, mut pow, mut volt, mut curr) = (
        Acc::default(),
        Acc::default(),
        Acc::default(),
        Acc::default(),
        Acc::default(),
    );
    for s in chunk {
        cap.add(s.cap.map(|v| v as f64));
        temp.add(s.temp);
        pow.add(s.pow);
        volt.add(s.volt);
        curr.add(s.curr);
    }
    // chunk 来自 samples.chunks(bucket)，bucket>=1 且 samples 非空 → 桶必非空。
    let last = chunk.last().expect("downsample 不产生空桶");
    Sample {
        t: last.t,
        cap: cap.avg().map(|v| v.round() as i64),
        temp: temp.avg(),
        pow: pow.avg(),
        volt: volt.avg(),
        curr: curr.avg(),
        chg: last.chg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一个样本并入桶后，平均值与计数应正确反映出来。
    #[test]
    fn consolidates_average_within_bucket() {
        let mut arc = Archive::new(STEP_FINE_MS, 10);
        // 同一 30s 桶内的两个样本: t=0 与 t=10s。
        arc.add(
            0,
            Some(20.0),
            Some(30.0),
            Some(-5.0),
            Some(3.7),
            Some(-1000.0),
            false,
        );
        arc.add(
            10_000,
            Some(30.0),
            Some(32.0),
            Some(-7.0),
            Some(3.9),
            Some(-1200.0),
            false,
        );
        let s = arc.samples_since(0, 20_000);
        assert_eq!(s.len(), 1, "两样本落入同一桶应合并为一点");
        assert_eq!(s[0].cap, Some(25)); // (20+30)/2
        assert_eq!(s[0].temp, Some(31.0));
        assert_eq!(s[0].pow, Some(-6.0));
    }

    /// 跨桶的样本应产生独立的点，按时间升序。
    #[test]
    fn separate_buckets_stay_distinct() {
        let mut arc = Archive::new(STEP_FINE_MS, 10);
        arc.add(0, Some(10.0), None, None, None, None, false);
        arc.add(40_000, Some(20.0), None, None, None, None, false); // 下一个 30s 桶
        let s = arc.samples_since(0, 60_000);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].cap, Some(10));
        assert_eq!(s[1].cap, Some(20));
    }

    /// 充电状态按桶内多数表决。
    #[test]
    fn charging_is_majority_vote() {
        let mut arc = Archive::new(STEP_FINE_MS, 10);
        arc.add(0, None, None, None, None, None, true);
        arc.add(5_000, None, None, None, None, None, true);
        arc.add(10_000, None, None, None, None, None, false);
        let s = arc.samples_since(0, 20_000);
        assert_eq!(s[0].chg, true, "2/3 充电应判为充电");
    }

    /// 全 None 字段应得到 None 而非 0，空槽不产生样本。
    #[test]
    fn all_none_field_yields_none() {
        let mut arc = Archive::new(STEP_FINE_MS, 10);
        arc.add(0, Some(50.0), None, None, None, None, false);
        let s = arc.samples_since(0, 10_000);
        assert_eq!(s[0].cap, Some(50));
        assert_eq!(s[0].temp, None);
        assert_eq!(s[0].pow, None);
    }

    /// 环形归档: 超过容量的写入应覆盖最旧槽，而非无限增长。
    #[test]
    fn ring_wraps_and_overwrites() {
        let mut arc = Archive::new(STEP_FINE_MS, 3); // 仅 3 槽
        for i in 0..5u64 {
            arc.add(
                i * STEP_FINE_MS,
                Some(i as f64),
                None,
                None,
                None,
                None,
                false,
            );
        }
        // 只剩最近 3 个桶 (i=2,3,4)。
        let s = arc.samples_since(0, 5 * STEP_FINE_MS);
        assert_eq!(s.len(), 3);
        assert_eq!(
            s.iter().map(|x| x.cap.unwrap()).collect::<Vec<_>>(),
            vec![2, 3, 4]
        );
    }

    /// 二进制序列化往返应完全还原归档内容。
    #[test]
    fn binary_round_trip() {
        let mut a = Archives::new();
        a.add(
            0,
            Some(42.0),
            Some(25.5),
            Some(-3.3),
            Some(3.8),
            Some(-900.0),
            true,
        );
        a.add(
            STEP_FINE_MS,
            Some(43.0),
            None,
            Some(-4.4),
            None,
            None,
            false,
        );
        let bytes = a.to_bytes();
        assert_eq!(
            bytes.len(),
            MAGIC.len() + (ROWS_FINE + ROWS_COARSE) * ROW_BYTES
        );
        let b = Archives::from_bytes(&bytes).expect("应能还原");
        let orig = a.fine.samples_since(0, 10 * STEP_FINE_MS);
        let back = b.fine.samples_since(0, 10 * STEP_FINE_MS);
        assert_eq!(orig.len(), back.len());
        assert_eq!(orig[0].cap, back[0].cap);
        assert_eq!(orig[0].temp, back[0].temp);
        assert_eq!(orig[0].chg, back[0].chg);
        assert_eq!(orig[1].pow, back[1].pow);
    }

    /// 损坏/截断/错误魔数的数据应被拒绝。
    #[test]
    fn rejects_bad_bytes() {
        assert!(Archives::from_bytes(b"").is_none());
        assert!(Archives::from_bytes(b"XXXX").is_none());
        let mut bytes = Archives::new().to_bytes();
        bytes[0] = b'Z'; // 破坏魔数
        assert!(Archives::from_bytes(&bytes).is_none());
    }

    /// downsample 超过 MAX_POINTS 时压缩，否则原样返回。
    #[test]
    fn downsample_caps_point_count() {
        let small: Vec<Sample> = (0..10)
            .map(|i| Sample {
                t: i,
                cap: Some(i as i64),
                temp: None,
                pow: None,
                volt: None,
                curr: None,
                chg: false,
            })
            .collect();
        assert_eq!(downsample(small.clone()).len(), 10);

        let big: Vec<Sample> = (0..1000)
            .map(|i| Sample {
                t: i,
                cap: Some(i as i64),
                temp: None,
                pow: None,
                volt: None,
                curr: None,
                chg: false,
            })
            .collect();
        assert!(downsample(big).len() <= MAX_POINTS);
    }
}
