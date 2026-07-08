//! 历史趋势采集 — 定时采样电池快照，存入定长 RRD 风格环形归档，供前端画曲线。
//!
//! 设计 (round-robin / 多分辨率归档，借鉴 RRDtool 思想):
//! - 两级归档，文件大小固定 (~363KB)，不随时间增长、无需压实扫描:
//!   · fine   : 30s 步长 × 2880 槽 = 最近 24 小时
//!   · coarse : 5min 步长 × 2016 槽 = 最近 7 天 (写入时按平均合并)
//! - 写入即合并 (consolidate-on-write): 每个时间桶对落入的样本累加 sum/count，
//!   查询时算平均；app 关闭期间不采样，未写入的槽视为 None → 前端断线。
//! - 存储格式为定长小端二进制 (history.rdb)，写入用临时文件 + rename 原子替换。
//! - 旧版 history.jsonl 存在时自动迁移到归档并改名为 .bak。
//! - query 选合适分辨率的归档，再降采样到约 HISTORY_MAX_POINTS 个点，避免前端绘制过密。

use crate::{battery, power};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// 保留时长: 7 天 (毫秒)。
const RETENTION_MS: u64 = 7 * 24 * 60 * 60 * 1000;
/// 查询降采样目标点数。
const HISTORY_MAX_POINTS: usize = 240;

/// fine 归档: 30s 步长，保留 24 小时。
const HISTORY_FINE_STEP_MS: u64 = 30_000;
const ROWS_FINE: usize = 2880;
/// coarse 归档: 5min 步长，保留 7 天。
const HISTORY_COARSE_STEP_MS: u64 = 300_000;
const ROWS_COARSE: usize = 2016;

/// 每槽二进制大小: bucket_start_ms(8)+sample_count(4)+charging_count(4)
/// + 5×(sum f64(8)+count u32(4)) = 76 字节。
const ROW_BYTES: usize = 8 + 4 + 4 + 5 * (8 + 4);
const ARCHIVES_BYTES: usize = (ROWS_FINE + ROWS_COARSE) * ROW_BYTES;
/// 文件魔数，用于校验。
const MAGIC: &[u8; 4] = b"BSR1";
const INPUT_MAGIC: &[u8; 4] = b"BSI1";
const TOTAL_INPUT_SOURCE_ID: &str = "total";

/// 落盘节流: 每 N 次采样落盘一次 (10 × 30s ≈ 5 分钟)，降低写放大。
/// 内存归档始终是查询来源，崩溃至多丢失最近 ~5 分钟的细粒度点。
const FLUSH_EVERY: u32 = 10;

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 单个历史采样点 (查询输出)。
#[derive(Serialize, Deserialize, Clone)]
pub struct Sample {
    pub timestamp_ms: u64,        // unix 毫秒
    pub capacity: Option<i64>,    // 电量 %
    pub temperature: Option<f64>, // 温度 °C
    pub power_w: Option<f64>,     // 功率 W (带符号: 放电为负, 充电为正)
    pub voltage: Option<f64>,     // 电压 V
    pub current_ma: Option<f64>,  // 电流 mA (带符号)
    pub charging: bool,           // 是否充电中 (桶内多数表决)
}

/// 可空字段的均值累加器: 忽略 None，记录 sum 与样本数。
#[derive(Clone, Copy, Default)]
struct AverageAccumulator {
    sum: f64,
    count: u32,
}

impl AverageAccumulator {
    #[inline]
    fn add(&mut self, value: Option<f64>) {
        if let Some(value) = value {
            self.sum += value;
            self.count += 1;
        }
    }
    #[inline]
    fn average(&self) -> Option<f64> {
        (self.count > 0).then(|| self.sum / self.count as f64)
    }
}

/// 环形归档中的一个时间桶 (consolidate-on-write 累加器)。
#[derive(Clone, Copy, Default)]
struct Slot {
    bucket_start_ms: u64, // 桶对齐起始时间 (毫秒); 0 表示空槽
    sample_count: u32,    // 落入该桶的样本总数
    charging_count: u32,  // 其中处于充电状态的样本数
    capacity: AverageAccumulator,
    temperature: AverageAccumulator,
    power: AverageAccumulator,
    voltage: AverageAccumulator,
    current: AverageAccumulator,
}

impl Slot {
    /// 借用当前槽生成查询输出样本; 空槽返回 None。
    fn as_sample(&self) -> Option<Sample> {
        if self.sample_count == 0 {
            return None;
        }
        Some(Sample {
            timestamp_ms: self.bucket_start_ms,
            capacity: self.capacity.average().map(|v| v.round() as i64),
            temperature: self.temperature.average(),
            power_w: self.power.average(),
            voltage: self.voltage.average(),
            current_ma: self.current.average(),
            charging: self.charging_count * 2 > self.sample_count, // 桶内多数表决
        })
    }

    fn write_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.bucket_start_ms.to_le_bytes());
        out.extend_from_slice(&self.sample_count.to_le_bytes());
        out.extend_from_slice(&self.charging_count.to_le_bytes());
        for accumulator in [
            &self.capacity,
            &self.temperature,
            &self.power,
            &self.voltage,
            &self.current,
        ] {
            out.extend_from_slice(&accumulator.sum.to_le_bytes());
            out.extend_from_slice(&accumulator.count.to_le_bytes());
        }
    }

    /// 从恰好 ROW_BYTES 长的切片解析一个槽。
    fn read_bytes(buf: &[u8]) -> Slot {
        let u64a = |o: usize| u64::from_le_bytes(buf[o..o + 8].try_into().unwrap());
        let u32a = |o: usize| u32::from_le_bytes(buf[o..o + 4].try_into().unwrap());
        let f64a = |o: usize| f64::from_le_bytes(buf[o..o + 8].try_into().unwrap());
        let mut slot = Slot {
            bucket_start_ms: u64a(0),
            sample_count: u32a(8),
            charging_count: u32a(12),
            ..Slot::default()
        };
        let mut offset = 16;
        for accumulator in [
            &mut slot.capacity,
            &mut slot.temperature,
            &mut slot.power,
            &mut slot.voltage,
            &mut slot.current,
        ] {
            accumulator.sum = f64a(offset);
            accumulator.count = u32a(offset + 8);
            offset += 12;
        }
        slot
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
        timestamp_ms: u64,
        capacity: Option<f64>,
        temperature: Option<f64>,
        power_w: Option<f64>,
        voltage: Option<f64>,
        current_ma: Option<f64>,
        charging: bool,
    ) {
        let bucket_start_ms = timestamp_ms - (timestamp_ms % self.step);
        let slot_index = (bucket_start_ms / self.step) as usize % self.slots.len();
        let slot = &mut self.slots[slot_index];
        if slot.bucket_start_ms != bucket_start_ms {
            // 该索引上残留的是已滚出的旧桶，重置后归入新桶。
            *slot = Slot {
                bucket_start_ms,
                ..Slot::default()
            };
        }
        slot.sample_count += 1;
        if charging {
            slot.charging_count += 1;
        }
        slot.capacity.add(capacity);
        slot.temperature.add(temperature);
        slot.power.add(power_w);
        slot.voltage.add(voltage);
        slot.current.add(current_ma);
    }

    /// 时间 >= cutoff 且未超出保留期的样本，按时间升序。
    fn samples_since(&self, cutoff: u64, now: u64) -> Vec<Sample> {
        let oldest = now.saturating_sub(RETENTION_MS);
        let mut out: Vec<Sample> = self
            .slots
            .iter()
            .filter(|slot| slot.bucket_start_ms >= cutoff && slot.bucket_start_ms >= oldest)
            .filter_map(Slot::as_sample)
            .collect();
        out.sort_by_key(|sample| sample.timestamp_ms);
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
            fine: Archive::new(HISTORY_FINE_STEP_MS, ROWS_FINE),
            coarse: Archive::new(HISTORY_COARSE_STEP_MS, ROWS_COARSE),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        timestamp_ms: u64,
        capacity: Option<f64>,
        temperature: Option<f64>,
        power_w: Option<f64>,
        voltage: Option<f64>,
        current_ma: Option<f64>,
        charging: bool,
    ) {
        self.fine.add(
            timestamp_ms,
            capacity,
            temperature,
            power_w,
            voltage,
            current_ma,
            charging,
        );
        self.coarse.add(
            timestamp_ms,
            capacity,
            temperature,
            power_w,
            voltage,
            current_ma,
            charging,
        );
    }

    /// 序列化为定长二进制: MAGIC + fine 槽 + coarse 槽。
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(MAGIC.len() + ARCHIVES_BYTES);
        out.extend_from_slice(MAGIC);
        self.write_archive_bytes(&mut out);
        out
    }

    fn write_archive_bytes(&self, out: &mut Vec<u8>) {
        for slot in &self.fine.slots {
            slot.write_bytes(out);
        }
        for slot in &self.coarse.slots {
            slot.write_bytes(out);
        }
    }

    /// 从二进制还原; 魔数或长度不符则返回 None (当作无历史)。
    fn from_bytes(buf: &[u8]) -> Option<Archives> {
        let expected = MAGIC.len() + ARCHIVES_BYTES;
        if buf.len() != expected || &buf[..MAGIC.len()] != MAGIC {
            return None;
        }
        Archives::from_archive_bytes(&buf[MAGIC.len()..])
    }

    fn from_archive_bytes(buf: &[u8]) -> Option<Archives> {
        if buf.len() != ARCHIVES_BYTES {
            return None;
        }
        let mut a = Archives::new();
        let mut o = 0;
        for slot in a.fine.slots.iter_mut().chain(a.coarse.slots.iter_mut()) {
            *slot = Slot::read_bytes(&buf[o..o + ROW_BYTES]);
            o += ROW_BYTES;
        }
        Some(a)
    }
}

#[derive(Clone)]
struct InputReading {
    source_id: String,
    power_w: Option<f64>,
    voltage: Option<f64>,
    current_ma: Option<f64>,
}

#[derive(Default)]
struct InputArchives {
    sources: HashMap<String, Archives>,
}

impl InputArchives {
    fn add_readings(&mut self, timestamp_ms: u64, readings: &[InputReading]) {
        for reading in readings {
            self.sources
                .entry(reading.source_id.clone())
                .or_insert_with(Archives::new)
                .add(
                    timestamp_ms,
                    None,
                    None,
                    reading.power_w,
                    reading.voltage,
                    reading.current_ma,
                    true,
                );
        }
    }

    fn query(&self, range_ms: u64, source_id: Option<&str>) -> Vec<Sample> {
        let source_id = source_id
            .filter(|id| !id.trim().is_empty())
            .unwrap_or(TOTAL_INPUT_SOURCE_ID);
        self.sources
            .get(source_id)
            .map(|archives| query_archives(archives, range_ms))
            .unwrap_or_default()
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut source_ids = self.sources.keys().collect::<Vec<_>>();
        source_ids.sort();

        let mut out = Vec::with_capacity(
            INPUT_MAGIC.len()
                + 4
                + source_ids
                    .iter()
                    .map(|id| 4 + id.len() + ARCHIVES_BYTES)
                    .sum::<usize>(),
        );
        out.extend_from_slice(INPUT_MAGIC);
        out.extend_from_slice(&(source_ids.len() as u32).to_le_bytes());
        for source_id in source_ids {
            let id_bytes = source_id.as_bytes();
            out.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(id_bytes);
            self.sources[source_id].write_archive_bytes(&mut out);
        }
        out
    }

    fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < INPUT_MAGIC.len() + 4 || &buf[..INPUT_MAGIC.len()] != INPUT_MAGIC {
            return None;
        }

        let mut offset = INPUT_MAGIC.len();
        let source_count = read_u32(buf, &mut offset)? as usize;
        let mut sources = HashMap::with_capacity(source_count);
        for _ in 0..source_count {
            let id_len = read_u32(buf, &mut offset)? as usize;
            let id_end = offset.checked_add(id_len)?;
            if id_end > buf.len() {
                return None;
            }
            let source_id = std::str::from_utf8(&buf[offset..id_end]).ok()?.to_string();
            offset = id_end;

            let archive_end = offset.checked_add(ARCHIVES_BYTES)?;
            if archive_end > buf.len() {
                return None;
            }
            let archives = Archives::from_archive_bytes(&buf[offset..archive_end])?;
            offset = archive_end;
            sources.insert(source_id, archives);
        }

        (offset == buf.len()).then_some(InputArchives { sources })
    }
}

fn read_u32(buf: &[u8], offset: &mut usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    if end > buf.len() {
        return None;
    }
    let value = u32::from_le_bytes(buf[*offset..end].try_into().ok()?);
    *offset = end;
    Some(value)
}

fn input_readings(sources: &[power::PowerSource]) -> Vec<InputReading> {
    let online_sources = sources
        .iter()
        .filter(|source| source.online == Some(true))
        .collect::<Vec<_>>();
    if online_sources.is_empty() {
        return Vec::new();
    }

    let mut readings = Vec::with_capacity(online_sources.len() + 1);
    readings.push(InputReading {
        source_id: TOTAL_INPUT_SOURCE_ID.to_string(),
        power_w: sum_options(
            online_sources
                .iter()
                .filter_map(|source| source.power_now.map(|power| power.abs())),
        ),
        voltage: average_options(
            online_sources
                .iter()
                .filter_map(|source| source.voltage_now),
        ),
        current_ma: sum_options(
            online_sources
                .iter()
                .filter_map(|source| source.current_now.map(|current| current.abs())),
        ),
    });

    readings.extend(online_sources.into_iter().map(|source| InputReading {
        source_id: source.name.clone(),
        power_w: source.power_now.map(|power| power.abs()),
        voltage: source.voltage_now,
        current_ma: source.current_now.map(|current| current.abs()),
    }));
    readings
}

fn sum_options(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut count = 0u32;
    let sum = values.inspect(|_| count += 1).sum::<f64>();
    (count > 0).then_some(sum)
}

fn average_options(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut count = 0u32;
    let sum = values.inspect(|_| count += 1).sum::<f64>();
    (count > 0).then(|| sum / count as f64)
}

pub struct HistoryStore {
    path: PathBuf,
    archives: Mutex<Archives>,
    input_path: PathBuf,
    input_archives: Mutex<InputArchives>,
    writes: AtomicU32,
    input_writes: AtomicU32,
}

impl HistoryStore {
    /// 从磁盘载入归档; 不存在则尝试迁移同目录旧版 history.jsonl, 否则空归档。
    pub fn load(path: PathBuf) -> Self {
        let jsonl = path.with_extension("jsonl");
        let input_path = path.with_file_name("input_history.rdb");
        let (archives, migrated) = match fs::read(&path)
            .ok()
            .and_then(|buf| Archives::from_bytes(&buf))
        {
            Some(a) => (a, false),
            None => migrate_legacy_jsonl(&jsonl),
        };
        let input_archives = fs::read(&input_path)
            .ok()
            .and_then(|buf| InputArchives::from_bytes(&buf))
            .unwrap_or_default();
        let store = HistoryStore {
            path,
            archives: Mutex::new(archives),
            input_path,
            input_archives: Mutex::new(input_archives),
            writes: AtomicU32::new(0),
            input_writes: AtomicU32::new(0),
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
        let timestamp_ms = now_ms();
        let mut wrote_battery = false;

        if let Some(battery) = battery::collect() {
            let charging = battery.status.as_deref() == Some("Charging");
            // 功率带符号: 放电(Discharging)为负，便于曲线区分方向。
            let power_w = battery
                .power_now
                .map(|power| if charging { power.abs() } else { -power.abs() });

            if let Ok(mut archives) = self.archives.lock() {
                archives.add(
                    timestamp_ms,
                    battery.capacity.map(|value| value as f64),
                    battery.temperature,
                    power_w,
                    battery.voltage_now,
                    battery.current_now,
                    charging,
                );
                wrote_battery = true;
            }
        }

        let readings = input_readings(&power::collect());
        let wrote_input = if readings.is_empty() {
            false
        } else if let Ok(mut archives) = self.input_archives.lock() {
            archives.add_readings(timestamp_ms, &readings);
            true
        } else {
            false
        };

        // 节流落盘: 内存归档是查询来源，磁盘只需周期性持久化。
        if wrote_battery
            && self
                .writes
                .fetch_add(1, Ordering::Relaxed)
                .is_multiple_of(FLUSH_EVERY)
        {
            self.flush();
        }
        if wrote_input
            && self
                .input_writes
                .fetch_add(1, Ordering::Relaxed)
                .is_multiple_of(FLUSH_EVERY)
        {
            self.flush_input();
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

    fn flush_input(&self) -> bool {
        let bytes = {
            let Ok(a) = self.input_archives.lock() else {
                return false;
            };
            a.to_bytes()
        };
        let tmp = self.input_path.with_extension("rdb.tmp");
        if let Err(e) = fs::write(&tmp, &bytes) {
            eprintln!("history: 输入历史写临时文件失败: {e}");
            return false;
        }
        if let Err(e) = fs::rename(&tmp, &self.input_path) {
            eprintln!("history: 输入历史 rename 失败: {e}");
            let _ = fs::remove_file(&tmp);
            return false;
        }
        true
    }

    /// 查询最近 range_ms 内的样本，自动选分辨率并降采样到约 HISTORY_MAX_POINTS 点。
    pub fn query(
        &self,
        range_ms: u64,
        source_kind: &str,
        input_source_id: Option<&str>,
    ) -> Vec<Sample> {
        if source_kind == "input" {
            return self
                .input_archives
                .lock()
                .map(|archives| archives.query(range_ms, input_source_id))
                .unwrap_or_default();
        }
        self.archives
            .lock()
            .map(|archives| query_archives(&archives, range_ms))
            .unwrap_or_default()
    }
}

fn query_archives(archives: &Archives, range_ms: u64) -> Vec<Sample> {
    let now = now_ms();
    let cutoff = now.saturating_sub(range_ms);
    // 范围超出 fine 归档覆盖 (24h) 时用 coarse, 否则用 fine。
    let src = if range_ms > HISTORY_FINE_STEP_MS * ROWS_FINE as u64 {
        &archives.coarse
    } else {
        &archives.fine
    };
    let samples = src.samples_since(cutoff, now);
    downsample(samples)
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
        #[serde(rename = "t")]
        timestamp_ms: u64,
        #[serde(rename = "cap")]
        capacity: Option<i64>,
        #[serde(rename = "temp")]
        temperature: Option<f64>,
        #[serde(rename = "pow")]
        power_w: Option<f64>,
        #[serde(rename = "volt")]
        voltage: Option<f64>,
        #[serde(rename = "curr")]
        current_ma: Option<f64>,
        #[serde(rename = "chg")]
        charging: bool,
    }
    let cutoff = now_ms().saturating_sub(RETENTION_MS);
    let mut migrated = 0u32;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(sample) = serde_json::from_str::<LegacySample>(line) {
            if sample.timestamp_ms >= cutoff {
                a.add(
                    sample.timestamp_ms,
                    sample.capacity.map(|value| value as f64),
                    sample.temperature,
                    sample.power_w,
                    sample.voltage,
                    sample.current_ma,
                    sample.charging,
                );
                migrated += 1;
            }
        }
    }
    (a, migrated > 0)
}

/// 按时间桶平均降采样。点数不超过 HISTORY_MAX_POINTS 时原样返回。
fn downsample(samples: Vec<Sample>) -> Vec<Sample> {
    if samples.len() <= HISTORY_MAX_POINTS {
        return samples;
    }
    let bucket = samples.len().div_ceil(HISTORY_MAX_POINTS);
    let mut out = Vec::with_capacity(HISTORY_MAX_POINTS);
    for chunk in samples.chunks(bucket) {
        out.push(average_bucket(chunk));
    }
    out
}

/// 对一个桶内的样本求平均; 可空字段忽略 None; charging/timestamp_ms 取桶内末值。
fn average_bucket(chunk: &[Sample]) -> Sample {
    let (mut capacity, mut temperature, mut power, mut voltage, mut current) = (
        AverageAccumulator::default(),
        AverageAccumulator::default(),
        AverageAccumulator::default(),
        AverageAccumulator::default(),
        AverageAccumulator::default(),
    );
    for sample in chunk {
        capacity.add(sample.capacity.map(|value| value as f64));
        temperature.add(sample.temperature);
        power.add(sample.power_w);
        voltage.add(sample.voltage);
        current.add(sample.current_ma);
    }
    // chunk 来自 samples.chunks(bucket)，bucket>=1 且 samples 非空 → 桶必非空。
    let last = chunk.last().expect("downsample 不产生空桶");
    Sample {
        timestamp_ms: last.timestamp_ms,
        capacity: capacity.average().map(|value| value.round() as i64),
        temperature: temperature.average(),
        power_w: power.average(),
        voltage: voltage.average(),
        current_ma: current.average(),
        charging: last.charging,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一个样本并入桶后，平均值与计数应正确反映出来。
    #[test]
    fn consolidates_average_within_bucket() {
        let mut arc = Archive::new(HISTORY_FINE_STEP_MS, 10);
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
        assert_eq!(s[0].capacity, Some(25)); // (20+30)/2
        assert_eq!(s[0].temperature, Some(31.0));
        assert_eq!(s[0].power_w, Some(-6.0));
    }

    /// 跨桶的样本应产生独立的点，按时间升序。
    #[test]
    fn separate_buckets_stay_distinct() {
        let mut arc = Archive::new(HISTORY_FINE_STEP_MS, 10);
        arc.add(0, Some(10.0), None, None, None, None, false);
        arc.add(40_000, Some(20.0), None, None, None, None, false); // 下一个 30s 桶
        let s = arc.samples_since(0, 60_000);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].capacity, Some(10));
        assert_eq!(s[1].capacity, Some(20));
    }

    /// 充电状态按桶内多数表决。
    #[test]
    fn charging_is_majority_vote() {
        let mut arc = Archive::new(HISTORY_FINE_STEP_MS, 10);
        arc.add(0, None, None, None, None, None, true);
        arc.add(5_000, None, None, None, None, None, true);
        arc.add(10_000, None, None, None, None, None, false);
        let s = arc.samples_since(0, 20_000);
        assert!(s[0].charging, "2/3 充电应判为充电");
    }

    /// 全 None 字段应得到 None 而非 0，空槽不产生样本。
    #[test]
    fn all_none_field_yields_none() {
        let mut arc = Archive::new(HISTORY_FINE_STEP_MS, 10);
        arc.add(0, Some(50.0), None, None, None, None, false);
        let s = arc.samples_since(0, 10_000);
        assert_eq!(s[0].capacity, Some(50));
        assert_eq!(s[0].temperature, None);
        assert_eq!(s[0].power_w, None);
    }

    /// 环形归档: 超过容量的写入应覆盖最旧槽，而非无限增长。
    #[test]
    fn ring_wraps_and_overwrites() {
        let mut arc = Archive::new(HISTORY_FINE_STEP_MS, 3); // 仅 3 槽
        for i in 0..5u64 {
            arc.add(
                i * HISTORY_FINE_STEP_MS,
                Some(i as f64),
                None,
                None,
                None,
                None,
                false,
            );
        }
        // 只剩最近 3 个桶 (i=2,3,4)。
        let s = arc.samples_since(0, 5 * HISTORY_FINE_STEP_MS);
        assert_eq!(s.len(), 3);
        assert_eq!(
            s.iter().map(|x| x.capacity.unwrap()).collect::<Vec<_>>(),
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
            HISTORY_FINE_STEP_MS,
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
        let orig = a.fine.samples_since(0, 10 * HISTORY_FINE_STEP_MS);
        let back = b.fine.samples_since(0, 10 * HISTORY_FINE_STEP_MS);
        assert_eq!(orig.len(), back.len());
        assert_eq!(orig[0].capacity, back[0].capacity);
        assert_eq!(orig[0].temperature, back[0].temperature);
        assert_eq!(orig[0].charging, back[0].charging);
        assert_eq!(orig[1].power_w, back[1].power_w);
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

    /// downsample 超过 HISTORY_MAX_POINTS 时压缩，否则原样返回。
    #[test]
    fn downsample_caps_point_count() {
        let small: Vec<Sample> = (0..10)
            .map(|i| Sample {
                timestamp_ms: i,
                capacity: Some(i as i64),
                temperature: None,
                power_w: None,
                voltage: None,
                current_ma: None,
                charging: false,
            })
            .collect();
        assert_eq!(downsample(small.clone()).len(), 10);

        let big: Vec<Sample> = (0..1000)
            .map(|i| Sample {
                timestamp_ms: i,
                capacity: Some(i as i64),
                temperature: None,
                power_w: None,
                voltage: None,
                current_ma: None,
                charging: false,
            })
            .collect();
        assert!(downsample(big).len() <= HISTORY_MAX_POINTS);
    }

    fn power_source(
        name: &str,
        online: bool,
        power_w: Option<f64>,
        voltage_now: Option<f64>,
        current_now: Option<f64>,
    ) -> power::PowerSource {
        power::PowerSource {
            name: name.to_string(),
            kind: "USB_C".to_string(),
            online: Some(online),
            voltage_now,
            current_now,
            current_max: None,
            power_now: power_w,
            usb_type: None,
        }
    }

    fn assert_close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("应有数值");
        assert!(
            (actual - expected).abs() < 0.000_001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn input_readings_sum_online_sources_only() {
        let readings = input_readings(&[
            power_source("usb0", true, Some(5.0), Some(20.0), Some(250.0)),
            power_source("wls0", true, Some(3.0), None, Some(100.0)),
            power_source("offline", false, Some(99.0), Some(9.0), Some(9.0)),
        ]);

        assert_eq!(readings.len(), 3);
        let total = readings
            .iter()
            .find(|reading| reading.source_id == TOTAL_INPUT_SOURCE_ID)
            .expect("应有总输入");
        assert_close(total.power_w, 8.0);
        assert_close(total.voltage, 20.0);
        assert_close(total.current_ma, 350.0);
        assert!(readings.iter().any(|reading| reading.source_id == "usb0"));
        assert!(readings.iter().any(|reading| reading.source_id == "wls0"));
        assert!(!readings
            .iter()
            .any(|reading| reading.source_id == "offline"));
    }

    #[test]
    fn input_archives_query_single_source() {
        let mut archives = InputArchives::default();
        let base = now_ms().saturating_sub(2 * HISTORY_FINE_STEP_MS);
        archives.add_readings(
            base,
            &input_readings(&[power_source(
                "usb0",
                true,
                Some(4.0),
                Some(10.0),
                Some(400.0),
            )]),
        );
        archives.add_readings(
            base + HISTORY_FINE_STEP_MS,
            &input_readings(&[power_source(
                "usb0",
                true,
                Some(6.0),
                Some(10.0),
                Some(600.0),
            )]),
        );

        let samples = archives.query(10 * HISTORY_FINE_STEP_MS, Some("usb0"));
        assert!(samples.len() >= 2);
        assert_close(samples[0].power_w, 4.0);
        assert_close(samples[1].current_ma, 600.0);
        assert!(archives
            .query(10 * HISTORY_FINE_STEP_MS, Some("missing"))
            .is_empty());
    }

    #[test]
    fn input_archives_binary_round_trip() {
        let mut archives = InputArchives::default();
        let base = now_ms().saturating_sub(HISTORY_FINE_STEP_MS);
        archives.add_readings(
            base,
            &input_readings(&[power_source(
                "usb0",
                true,
                Some(7.0),
                Some(20.0),
                Some(350.0),
            )]),
        );

        let bytes = archives.to_bytes();
        let back = InputArchives::from_bytes(&bytes).expect("输入历史应能还原");
        let samples = back.query(10 * HISTORY_FINE_STEP_MS, Some("usb0"));
        assert_eq!(samples.len(), 1);
        assert_close(samples[0].power_w, 7.0);
        assert!(InputArchives::from_bytes(b"BSR1").is_none());
    }
}
