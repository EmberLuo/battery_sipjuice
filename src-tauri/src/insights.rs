//! 充电洞察 — 独立于短期 RRD 的会话与健康快照存储。
//!
//! RRD 适合固定窗口曲线；这里保存低频、长期且有业务含义的记录。文件使用带版本号的
//! JSON，并通过临时文件 + rename 原子替换。进行中的会话也会周期性落盘，以便重启续接。

use crate::battery::{BatteryInfo, CapacityValue};
use crate::power::PowerSource;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

const DATA_VERSION: u32 = 1;
const FLUSH_EVERY: u32 = 10;
const MAX_SAMPLE_GAP_MS: u64 = 2 * 60_000;
const MAX_RESUME_GAP_MS: u64 = 10 * 60_000;
const END_DEBOUNCE_SAMPLES: u8 = 2;
const MIN_SESSION_MS: u64 = 30_000;
const HEALTH_INTERVAL_MS: u64 = 24 * 60 * 60_000;
const MAX_SESSIONS: usize = 1_000;
const MAX_HEALTH_SNAPSHOTS: usize = 5_000;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct StoredCapacity {
    pub value: f64,
    pub unit: String,
}

impl From<&CapacityValue> for StoredCapacity {
    fn from(value: &CapacityValue) -> Self {
        Self {
            value: value.value,
            unit: value.unit.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChargeSession {
    pub id: String,
    pub battery_id: String,
    pub battery_device: String,
    pub battery_name: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub start_capacity: Option<i64>,
    pub end_capacity: Option<i64>,
    pub charged_percent: Option<i64>,
    pub duration_ms: u64,
    pub charging_ms: u64,
    pub battery_energy_wh: Option<f64>,
    pub input_energy_wh: Option<f64>,
    pub charged_mah: Option<f64>,
    pub average_battery_power_w: Option<f64>,
    pub average_input_power_w: Option<f64>,
    pub peak_input_power_w: Option<f64>,
    pub average_temperature_c: Option<f64>,
    pub peak_temperature_c: Option<f64>,
    pub source_names: Vec<String>,
    pub source_kinds: Vec<String>,
    pub usb_types: Vec<String>,
    pub sample_count: u32,
    pub powered_sample_count: u32,
    pub health_percent_end: Option<f64>,
    pub cycle_count_end: Option<i64>,
    pub complete: bool,
    pub end_reason: String,
}

#[derive(Serialize, Clone)]
pub struct SessionView {
    #[serde(flatten)]
    pub session: ChargeSession,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HealthSnapshot {
    pub battery_id: String,
    pub battery_device: String,
    pub recorded_at_ms: u64,
    pub full_capacity: Option<StoredCapacity>,
    pub design_capacity: Option<StoredCapacity>,
    pub health_percent: Option<f64>,
    pub state_of_health: Option<i64>,
    pub cycle_count: Option<i64>,
}

#[derive(Serialize)]
pub struct InsightsView {
    pub sessions: Vec<SessionView>,
    pub health: Vec<HealthSnapshot>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ActiveSession {
    id: String,
    battery_id: String,
    battery_device: String,
    battery_name: String,
    start_ms: u64,
    last_ms: u64,
    last_active_ms: u64,
    start_capacity: Option<i64>,
    end_capacity: Option<i64>,
    end_health_percent: Option<f64>,
    end_cycle_count: Option<i64>,
    sample_count: u32,
    powered_sample_count: u32,
    charging_ms: u64,
    battery_energy_wh: f64,
    input_energy_wh: f64,
    charged_mah: f64,
    #[serde(default)]
    battery_power_ms: u64,
    #[serde(default)]
    input_power_ms: u64,
    battery_energy_samples: u32,
    input_energy_samples: u32,
    current_samples: u32,
    peak_input_power_w: Option<f64>,
    temperature_sum: f64,
    temperature_count: u32,
    peak_temperature_c: Option<f64>,
    source_names: HashSet<String>,
    source_kinds: HashSet<String>,
    usb_types: HashSet<String>,
    last_battery_power_w: Option<f64>,
    last_input_power_w: Option<f64>,
    last_current_ma: Option<f64>,
    last_was_charging: bool,
    inactive_samples: u8,
}

impl ActiveSession {
    fn new(battery: &BatteryInfo, sources: &[PowerSource], now: u64) -> Self {
        let battery_id = battery_identity(battery);
        let mut active = Self {
            id: format!("{}-{now}", battery.device),
            battery_id,
            battery_device: battery.device.clone(),
            battery_name: battery_name(battery),
            start_ms: now,
            last_ms: now,
            last_active_ms: now,
            start_capacity: battery.capacity,
            end_capacity: battery.capacity,
            end_health_percent: battery.health_percent,
            end_cycle_count: battery.cycle_count,
            sample_count: 0,
            powered_sample_count: 0,
            charging_ms: 0,
            battery_energy_wh: 0.0,
            input_energy_wh: 0.0,
            charged_mah: 0.0,
            battery_power_ms: 0,
            input_power_ms: 0,
            battery_energy_samples: 0,
            input_energy_samples: 0,
            current_samples: 0,
            peak_input_power_w: None,
            temperature_sum: 0.0,
            temperature_count: 0,
            peak_temperature_c: None,
            source_names: HashSet::new(),
            source_kinds: HashSet::new(),
            usb_types: HashSet::new(),
            last_battery_power_w: None,
            last_input_power_w: None,
            last_current_ma: None,
            last_was_charging: false,
            inactive_samples: 0,
        };
        active.observe(battery, sources, now);
        active
    }

    fn observe(&mut self, battery: &BatteryInfo, sources: &[PowerSource], now: u64) {
        let charging = battery.status.as_deref() == Some("Charging");
        let dt_ms = now.saturating_sub(self.last_ms).min(MAX_SAMPLE_GAP_MS);
        let battery_power = charging.then(|| battery.power_now.map(f64::abs)).flatten();
        let current_ma = charging
            .then(|| battery.current_now.map(f64::abs))
            .flatten();
        let input_power = total_input_power(sources);

        if charging && self.last_was_charging && dt_ms > 0 {
            let hours = dt_ms as f64 / 3_600_000.0;
            if let (Some(previous), Some(current)) = (self.last_battery_power_w, battery_power) {
                self.battery_energy_wh += (previous + current) * 0.5 * hours;
                self.battery_energy_samples += 1;
                self.battery_power_ms += dt_ms;
            }
            if let (Some(previous), Some(current)) = (self.last_input_power_w, input_power) {
                self.input_energy_wh += (previous + current) * 0.5 * hours;
                self.input_energy_samples += 1;
                self.input_power_ms += dt_ms;
            }
            if let (Some(previous), Some(current)) = (self.last_current_ma, current_ma) {
                self.charged_mah += (previous + current) * 0.5 * hours;
                self.current_samples += 1;
            }
            self.charging_ms += dt_ms;
        }

        self.last_ms = now;
        self.last_active_ms = now;
        self.end_capacity = battery.capacity.or(self.end_capacity);
        self.end_health_percent = battery.health_percent.or(self.end_health_percent);
        self.end_cycle_count = battery.cycle_count.or(self.end_cycle_count);
        self.sample_count += 1;
        if battery_power.is_some() || input_power.is_some() {
            self.powered_sample_count += 1;
        }
        if charging {
            if let Some(power) = input_power {
                self.peak_input_power_w = Some(self.peak_input_power_w.unwrap_or(power).max(power));
            }
        }
        if charging {
            if let Some(temp) = battery.temperature.filter(|value| value.is_finite()) {
                self.temperature_sum += temp;
                self.temperature_count += 1;
                self.peak_temperature_c = Some(self.peak_temperature_c.unwrap_or(temp).max(temp));
            }
        }
        collect_source_metadata(self, sources);
        self.last_battery_power_w = battery_power;
        self.last_input_power_w = input_power;
        self.last_current_ma = current_ma;
        self.last_was_charging = charging;
        self.inactive_samples = 0;
    }

    fn is_recordable(&self) -> bool {
        let duration_ms = self.last_active_ms.saturating_sub(self.start_ms);
        duration_ms >= MIN_SESSION_MS && self.sample_count >= 2
    }

    fn build_session(
        &self,
        complete: bool,
        reason: &str,
        include_aggregates: bool,
    ) -> ChargeSession {
        let duration_ms = self.last_active_ms.saturating_sub(self.start_ms);
        let charged_percent = self
            .start_capacity
            .zip(self.end_capacity)
            .map(|(start, end)| end - start);
        ChargeSession {
            id: self.id.clone(),
            battery_id: self.battery_id.clone(),
            battery_device: self.battery_device.clone(),
            battery_name: self.battery_name.clone(),
            start_ms: self.start_ms,
            end_ms: self.last_active_ms,
            start_capacity: self.start_capacity,
            end_capacity: self.end_capacity,
            charged_percent,
            duration_ms,
            charging_ms: self.charging_ms,
            battery_energy_wh: (include_aggregates && self.battery_energy_samples > 0)
                .then_some(self.battery_energy_wh),
            input_energy_wh: (include_aggregates && self.input_energy_samples > 0)
                .then_some(self.input_energy_wh),
            charged_mah: (include_aggregates && self.current_samples > 0)
                .then_some(self.charged_mah),
            average_battery_power_w: include_aggregates
                .then(|| {
                    average_power(
                        self.battery_energy_wh,
                        self.battery_power_ms,
                        self.battery_energy_samples,
                    )
                })
                .flatten(),
            average_input_power_w: include_aggregates
                .then(|| {
                    average_power(
                        self.input_energy_wh,
                        self.input_power_ms,
                        self.input_energy_samples,
                    )
                })
                .flatten(),
            peak_input_power_w: self.peak_input_power_w,
            average_temperature_c: (include_aggregates && self.temperature_count > 0)
                .then(|| self.temperature_sum / self.temperature_count as f64),
            peak_temperature_c: self.peak_temperature_c,
            source_names: sorted(self.source_names.clone()),
            source_kinds: sorted(self.source_kinds.clone()),
            usb_types: sorted(self.usb_types.clone()),
            sample_count: self.sample_count,
            powered_sample_count: self.powered_sample_count,
            health_percent_end: self.end_health_percent,
            cycle_count_end: self.end_cycle_count,
            complete,
            end_reason: reason.to_string(),
        }
    }

    fn finish(self, complete: bool, reason: &str) -> Option<ChargeSession> {
        self.is_recordable()
            .then(|| self.build_session(complete, reason, true))
    }

    fn preview(&self) -> ChargeSession {
        self.build_session(false, "active", self.is_recordable())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct InsightsData {
    version: u32,
    sessions: Vec<ChargeSession>,
    active: HashMap<String, ActiveSession>,
    health: Vec<HealthSnapshot>,
}

impl Default for InsightsData {
    fn default() -> Self {
        Self {
            version: DATA_VERSION,
            sessions: Vec::new(),
            active: HashMap::new(),
            health: Vec::new(),
        }
    }
}

impl InsightsData {
    fn tick(&mut self, batteries: &[BatteryInfo], sources: &[PowerSource], now: u64) -> bool {
        let stale = self
            .active
            .iter()
            .filter(|(_, active)| now.saturating_sub(active.last_ms) > MAX_RESUME_GAP_MS)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        let mut urgent = false;
        for id in stale {
            if let Some(active) = self.active.remove(&id) {
                urgent |= self.push_finished(active, false, "sampling_gap");
            }
        }

        let mut seen = HashSet::new();
        for battery in batteries {
            let id = battery_identity(battery);
            seen.insert(id.clone());
            let charging = battery.status.as_deref() == Some("Charging");
            let externally_powered = sources.iter().any(|source| source.online == Some(true));
            if let Some(mut active) = self.active.remove(&id) {
                if charging || externally_powered {
                    active.observe(battery, sources, now);
                    self.active.insert(id.clone(), active);
                } else {
                    active.inactive_samples = active.inactive_samples.saturating_add(1);
                    if active.inactive_samples >= END_DEBOUNCE_SAMPLES {
                        urgent |= self.push_finished(active, true, "unplugged");
                        self.record_health(battery, now, true);
                    } else {
                        self.active.insert(id.clone(), active);
                    }
                }
            } else if charging {
                self.active
                    .insert(id.clone(), ActiveSession::new(battery, sources, now));
                // 会话刚开始时立即落盘，避免在下一次周期保存前退出而丢失整段会话。
                urgent = true;
            }
            self.record_health(battery, now, false);
        }

        let missing = self
            .active
            .keys()
            .filter(|id| !seen.contains(*id))
            .cloned()
            .collect::<Vec<_>>();
        for id in missing {
            if let Some(mut active) = self.active.remove(&id) {
                active.inactive_samples = active.inactive_samples.saturating_add(1);
                if active.inactive_samples >= END_DEBOUNCE_SAMPLES {
                    urgent |= self.push_finished(active, false, "battery_missing");
                } else {
                    self.active.insert(id, active);
                }
            }
        }
        urgent
    }

    fn push_finished(&mut self, active: ActiveSession, complete: bool, reason: &str) -> bool {
        let Some(session) = active.finish(complete, reason) else {
            return false;
        };
        self.sessions.push(session);
        self.sessions.sort_by_key(|session| session.start_ms);
        if self.sessions.len() > MAX_SESSIONS {
            self.sessions.drain(..self.sessions.len() - MAX_SESSIONS);
        }
        true
    }

    fn record_health(&mut self, battery: &BatteryInfo, now: u64, session_finished: bool) {
        if battery.full_capacity.is_none()
            && battery.health_percent.is_none()
            && battery.state_of_health.is_none()
            && battery.cycle_count.is_none()
        {
            return;
        }
        let id = battery_identity(battery);
        let previous = self
            .health
            .iter()
            .rev()
            .find(|snapshot| snapshot.battery_id == id);
        let due = previous
            .map(|snapshot| now.saturating_sub(snapshot.recorded_at_ms) >= HEALTH_INTERVAL_MS)
            .unwrap_or(true);
        let changed = previous.is_some_and(|snapshot| {
            snapshot.cycle_count != battery.cycle_count
                || snapshot.state_of_health != battery.state_of_health
                || option_f64_changed(snapshot.health_percent, battery.health_percent, 0.1)
                || capacity_changed(
                    snapshot.full_capacity.as_ref(),
                    battery.full_capacity.as_ref(),
                )
        });
        if !(due || session_finished && changed) {
            return;
        }
        self.health.push(HealthSnapshot {
            battery_id: id,
            battery_device: battery.device.clone(),
            recorded_at_ms: now,
            full_capacity: battery.full_capacity.as_ref().map(StoredCapacity::from),
            design_capacity: battery.design_capacity.as_ref().map(StoredCapacity::from),
            health_percent: battery.health_percent,
            state_of_health: battery.state_of_health,
            cycle_count: battery.cycle_count,
        });
        if self.health.len() > MAX_HEALTH_SNAPSHOTS {
            self.health
                .drain(..self.health.len() - MAX_HEALTH_SNAPSHOTS);
        }
    }
}

pub struct InsightsStore {
    path: PathBuf,
    inner: Mutex<InsightsData>,
    writes: AtomicU32,
}

impl InsightsStore {
    pub fn load(path: PathBuf) -> Self {
        let data = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<InsightsData>(&text).ok())
            .filter(|data| data.version == DATA_VERSION)
            .unwrap_or_default();
        Self {
            path,
            inner: Mutex::new(data),
            writes: AtomicU32::new(0),
        }
    }

    pub fn tick(&self, batteries: &[BatteryInfo], sources: &[PowerSource], now: u64) {
        let urgent = self
            .inner
            .lock()
            .map(|mut data| data.tick(batteries, sources, now))
            .unwrap_or(false);
        let periodic = self
            .writes
            .fetch_add(1, Ordering::Relaxed)
            .is_multiple_of(FLUSH_EVERY);
        if urgent || periodic {
            if let Err(error) = self.flush() {
                eprintln!("insights: 保存失败: {error}");
            }
        }
    }

    pub fn view(&self, battery_id: Option<&str>) -> InsightsView {
        let Ok(data) = self.inner.lock() else {
            return InsightsView {
                sessions: Vec::new(),
                health: Vec::new(),
            };
        };
        let matches = |id: &str| {
            battery_id
                .filter(|value| !value.is_empty())
                .is_none_or(|value| value == id)
        };
        let mut sessions = data
            .sessions
            .iter()
            .filter(|session| matches(&session.battery_id))
            .cloned()
            .map(|session| SessionView {
                session,
                active: false,
            })
            .chain(
                data.active
                    .values()
                    .filter(|active| matches(&active.battery_id))
                    .map(|active| {
                        let session = active.preview();
                        SessionView {
                            session,
                            active: true,
                        }
                    }),
            )
            .collect::<Vec<_>>();
        sessions.sort_by_key(|session| Reverse(session.session.start_ms));
        sessions.truncate(200);

        let mut health = data
            .health
            .iter()
            .filter(|snapshot| matches(&snapshot.battery_id))
            .cloned()
            .collect::<Vec<_>>();
        health.sort_by_key(|snapshot| snapshot.recorded_at_ms);

        InsightsView { sessions, health }
    }

    pub fn flush(&self) -> Result<(), String> {
        let bytes = {
            let data = self
                .inner
                .lock()
                .map_err(|_| "洞察数据锁中毒".to_string())?;
            serde_json::to_vec_pretty(&*data).map_err(|error| error.to_string())?
        };
        atomic_write(&self.path, &bytes)
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes).map_err(|error| error.to_string())?;
    fs::rename(&tmp, path).map_err(|error| {
        let _ = fs::remove_file(&tmp);
        error.to_string()
    })
}

pub fn battery_identity(battery: &BatteryInfo) -> String {
    battery
        .serial_number
        .as_deref()
        .filter(|serial| !serial.trim().is_empty())
        .map(|serial| format!("serial:{}", serial.trim()))
        .unwrap_or_else(|| format!("device:{}", battery.device))
}

fn battery_name(battery: &BatteryInfo) -> String {
    battery
        .model
        .as_deref()
        .or(battery.manufacturer.as_deref())
        .unwrap_or(&battery.device)
        .to_string()
}

fn total_input_power(sources: &[PowerSource]) -> Option<f64> {
    let values = sources
        .iter()
        .filter(|source| source.online == Some(true))
        .filter_map(|source| source.power_now.map(f64::abs))
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.iter().sum())
}

fn collect_source_metadata(active: &mut ActiveSession, sources: &[PowerSource]) {
    for source in sources.iter().filter(|source| source.online == Some(true)) {
        active.source_names.insert(source.name.clone());
        active.source_kinds.insert(source.kind.clone());
        if let Some(usb_type) = &source.usb_type {
            active.usb_types.insert(usb_type.clone());
        }
    }
}

fn sorted(values: HashSet<String>) -> Vec<String> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort();
    values
}

fn average_power(energy_wh: f64, duration_ms: u64, samples: u32) -> Option<f64> {
    (samples > 0 && duration_ms > 0).then(|| energy_wh / (duration_ms as f64 / 3_600_000.0))
}

fn capacity_changed(previous: Option<&StoredCapacity>, current: Option<&CapacityValue>) -> bool {
    match (previous, current) {
        (Some(previous), Some(current)) => {
            previous.unit != current.unit || (previous.value - current.value).abs() >= 0.5
        }
        (None, None) => false,
        _ => true,
    }
}

fn option_f64_changed(previous: Option<f64>, current: Option<f64>, threshold: f64) -> bool {
    match (previous, current) {
        (Some(previous), Some(current)) => (previous - current).abs() >= threshold,
        (None, None) => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn battery(status: &str, capacity: i64) -> BatteryInfo {
        BatteryInfo {
            device: "BAT0".to_string(),
            model: Some("Test Battery".to_string()),
            manufacturer: Some("Test".to_string()),
            serial_number: Some("SERIAL-1".to_string()),
            technology: Some("Li-ion".to_string()),
            present: Some(true),
            capacity: Some(capacity),
            status: Some(status.to_string()),
            health_status: None,
            full_capacity: Some(CapacityValue {
                value: 45.0,
                unit: "Wh".to_string(),
                source_kind: "energy".to_string(),
            }),
            design_capacity: Some(CapacityValue {
                value: 50.0,
                unit: "Wh".to_string(),
                source_kind: "energy".to_string(),
            }),
            health_percent: Some(90.0),
            cycle_count: Some(100),
            state_of_health: Some(90),
            voltage_now: Some(12.0),
            voltage_ocv: None,
            voltage_max: None,
            current_now: Some(1_000.0),
            power_now: Some(10.0),
            temperature: Some(32.0),
            internal_resistance: None,
        }
    }

    fn source(online: bool) -> PowerSource {
        PowerSource {
            name: "usb0".to_string(),
            kind: "USB_C".to_string(),
            online: Some(online),
            voltage_now: Some(20.0),
            current_now: Some(1_000.0),
            current_max: Some(3_000.0),
            power_now: Some(20.0),
            usb_type: Some("PD".to_string()),
        }
    }

    #[test]
    fn charging_session_is_integrated_and_finalized_after_unplug_debounce() {
        let mut data = InsightsData::default();
        let t0 = 1_000_000;
        data.tick(&[battery("Charging", 20)], &[source(true)], t0);
        data.tick(&[battery("Charging", 30)], &[source(true)], t0 + 30_000);
        data.tick(&[battery("Charging", 40)], &[source(true)], t0 + 60_000);
        assert_eq!(data.active.len(), 1);

        data.tick(&[battery("Discharging", 40)], &[source(false)], t0 + 90_000);
        assert!(data.sessions.is_empty(), "第一次离线采样只进入结束防抖");
        data.tick(
            &[battery("Discharging", 40)],
            &[source(false)],
            t0 + 120_000,
        );

        assert!(data.active.is_empty());
        assert_eq!(data.sessions.len(), 1);
        let session = &data.sessions[0];
        assert_eq!(session.start_capacity, Some(20));
        assert_eq!(session.end_capacity, Some(40));
        assert_eq!(session.charged_percent, Some(20));
        assert_eq!(session.duration_ms, 60_000);
        assert!((session.battery_energy_wh.unwrap() - 1.0 / 6.0).abs() < 0.001);
        assert!((session.input_energy_wh.unwrap() - 1.0 / 3.0).abs() < 0.001);
        assert!((session.charged_mah.unwrap() - 16.666).abs() < 0.01);
        assert_eq!(session.usb_types, vec!["PD"]);
        assert!(session.complete);
    }

    #[test]
    fn powered_but_temporarily_not_charging_stays_in_the_same_session() {
        let mut data = InsightsData::default();
        let t0 = 2_000_000;
        data.tick(&[battery("Charging", 70)], &[source(true)], t0);
        data.tick(&[battery("Not charging", 80)], &[source(true)], t0 + 30_000);
        data.tick(&[battery("Charging", 80)], &[source(true)], t0 + 60_000);
        assert_eq!(data.active.len(), 1);
        assert!(data.sessions.is_empty());
    }

    #[test]
    fn starting_session_requests_immediate_flush() {
        let mut data = InsightsData::default();
        assert!(data.tick(&[battery("Charging", 20)], &[source(true)], 2_250_000));
        assert_eq!(data.active.len(), 1);
    }

    #[test]
    fn active_preview_matches_finish_once_session_is_recordable() {
        let t0 = 2_300_000;
        let mut active = ActiveSession::new(&battery("Charging", 20), &[source(true)], t0);
        active.observe(
            &battery("Charging", 30),
            &[source(true)],
            t0 + MIN_SESSION_MS,
        );

        let preview = active.preview();
        let finished = active.finish(false, "active").unwrap();

        assert_eq!(
            serde_json::to_value(preview).unwrap(),
            serde_json::to_value(finished).unwrap()
        );
    }

    #[test]
    fn early_active_preview_keeps_aggregate_metrics_hidden() {
        let active = ActiveSession::new(&battery("Charging", 20), &[source(true)], 2_400_000);

        let preview = active.preview();

        assert_eq!(preview.sample_count, 1);
        assert_eq!(preview.battery_energy_wh, None);
        assert_eq!(preview.input_energy_wh, None);
        assert_eq!(preview.average_temperature_c, None);
        assert_eq!(preview.peak_temperature_c, Some(32.0));
    }

    #[test]
    fn session_temperature_only_includes_actual_charging_samples() {
        let mut data = InsightsData::default();
        let t0 = 2_500_000;
        let mut charging = battery("Charging", 70);
        charging.temperature = Some(32.0);
        data.tick(&[charging], &[source(true)], t0);

        let mut idle = battery("Not charging", 80);
        idle.temperature = Some(70.0);
        data.tick(&[idle], &[source(true)], t0 + 30_000);

        let mut charging_again = battery("Charging", 85);
        charging_again.temperature = Some(34.0);
        data.tick(&[charging_again], &[source(true)], t0 + 60_000);
        data.tick(&[battery("Discharging", 85)], &[source(false)], t0 + 90_000);
        data.tick(
            &[battery("Discharging", 85)],
            &[source(false)],
            t0 + 120_000,
        );

        let session = &data.sessions[0];
        assert_eq!(session.average_temperature_c, Some(33.0));
        assert_eq!(session.peak_temperature_c, Some(34.0));
    }

    #[test]
    fn stale_active_session_is_closed_as_incomplete() {
        let mut data = InsightsData::default();
        let t0 = 3_000_000;
        data.tick(&[battery("Charging", 20)], &[source(true)], t0);
        data.tick(&[battery("Charging", 30)], &[source(true)], t0 + 30_000);
        data.tick(
            &[battery("Charging", 40)],
            &[source(true)],
            t0 + MAX_RESUME_GAP_MS + 60_000,
        );
        assert_eq!(data.sessions.len(), 1);
        assert!(!data.sessions[0].complete);
        assert_eq!(data.sessions[0].end_reason, "sampling_gap");
        assert_eq!(data.active.len(), 1, "当前仍充电时应开启新会话");
    }

    #[test]
    fn health_snapshots_are_daily_and_separated_by_serial_number() {
        let mut data = InsightsData::default();
        let t0 = 4_000_000;
        data.tick(&[battery("Discharging", 50)], &[], t0);
        data.tick(
            &[battery("Discharging", 49)],
            &[],
            t0 + HEALTH_INTERVAL_MS - 1,
        );
        assert_eq!(data.health.len(), 1);
        data.tick(&[battery("Discharging", 48)], &[], t0 + HEALTH_INTERVAL_MS);
        assert_eq!(data.health.len(), 2);

        let mut replacement = battery("Discharging", 80);
        replacement.serial_number = Some("SERIAL-2".to_string());
        data.tick(&[replacement], &[], t0 + HEALTH_INTERVAL_MS + 30_000);
        assert_eq!(data.health.len(), 3);
        assert_ne!(data.health[0].battery_id, data.health[2].battery_id);
    }

    #[test]
    fn versioned_data_round_trips() {
        let mut data = InsightsData::default();
        data.tick(&[battery("Discharging", 50)], &[], 6_000_000);
        let json = serde_json::to_string(&data).unwrap();
        let restored: InsightsData = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.version, DATA_VERSION);
        assert_eq!(restored.health.len(), 1);
    }
}
