//! 应用设置 — 持久化到配置目录的 settings.json。
//!
//! 设置与历史数据分开存：历史属于"数据"(app_data_dir)，设置属于"配置"(app_config_dir)。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// 关闭按钮(窗口 X)行为。
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CloseAction {
    /// 尚未选择 —— 前端应弹框询问。
    #[default]
    Ask,
    /// 隐藏到托盘，应用继续后台运行。
    Tray,
    /// 退出整个应用。
    Exit,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    /// 界面语言。
    pub language: String,
    /// 外观主题：system / light / dark。
    pub theme: String,
    /// 主题强调色：system / orange / blue。
    pub accent_color: String,
    /// 开机自启动。
    pub autostart: bool,
    /// 静默启动 —— 启动时不显示窗口，只显示托盘。
    pub silent_start: bool,
    /// 关闭按钮行为。
    pub close_action: CloseAction,

    /// 低电量提醒：放电中电量 ≤ remind_charge_at 时提醒接电源。
    pub remind_charge: bool,
    /// 低电量提醒阈值 (%)。
    pub remind_charge_at: i64,
    /// 高电量提醒：充电中电量 ≥ remind_unplug_at 时提醒拔电源。
    pub remind_unplug: bool,
    /// 高电量提醒阈值 (%)。
    pub remind_unplug_at: i64,
    /// 高温提醒：电池温度 ≥ remind_temp_high_at 时提醒。
    pub remind_temp_high: bool,
    /// 高温提醒阈值 (°C)。
    pub remind_temp_high_at: i64,
    /// 低温提醒：电池温度 ≤ remind_temp_low_at 时提醒。
    pub remind_temp_low: bool,
    /// 低温提醒阈值 (°C)。
    pub remind_temp_low_at: i64,
    /// 异常耗电提醒：放电功率持续 ≥ remind_drain_at 时提醒。
    pub remind_drain: bool,
    /// 异常耗电提醒阈值 (W)。
    pub remind_drain_at: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "zh-CN".to_string(),
            theme: "system".to_string(),
            accent_color: "system".to_string(),
            autostart: false,
            silent_start: false,
            close_action: CloseAction::Ask,
            remind_charge: true,
            remind_charge_at: 30,
            remind_unplug: true,
            remind_unplug_at: 80,
            remind_temp_high: true,
            remind_temp_high_at: 45,
            remind_temp_low: true,
            remind_temp_low_at: 5,
            remind_drain: true,
            remind_drain_at: 30,
        }
    }
}

impl Settings {
    /// 约束温度阈值到 UI 支持范围，并为 1°C 回差保留至少 2°C 的阈值间隔。
    /// 该层校验同时覆盖旧配置、手工修改配置以及绕过前端的命令调用。
    fn normalized(mut self) -> Self {
        self.remind_temp_high_at = self.remind_temp_high_at.clamp(20, 80);
        self.remind_temp_low_at = self.remind_temp_low_at.clamp(-10, 20);
        if self.remind_temp_low_at > self.remind_temp_high_at - 2 {
            self.remind_temp_low_at = self.remind_temp_high_at - 2;
        }
        self.remind_drain_at = self.remind_drain_at.clamp(5, 150);
        self
    }
}

/// 设置状态：路径 + 内存副本（Mutex 保护）。
pub struct SettingsStore {
    path: PathBuf,
    inner: Mutex<Settings>,
}

impl SettingsStore {
    /// 从磁盘载入设置；不存在或解析失败则用默认值。
    pub fn load(path: PathBuf) -> Self {
        let settings = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .unwrap_or_default()
            .normalized();
        SettingsStore {
            path,
            inner: Mutex::new(settings),
        }
    }

    /// 取当前设置的副本。
    pub fn get(&self) -> Settings {
        self.inner.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// 整体替换并写盘；持久化失败会返回错误。
    pub fn set(&self, settings: Settings) -> Result<(), String> {
        let settings = settings.normalized();
        // 持锁仅用于更新内存副本并序列化，写盘前释放。
        let json = {
            let mut guard = self.inner.lock().map_err(|_| "设置锁中毒".to_string())?;
            *guard = settings;
            serde_json::to_string_pretty(&*guard).map_err(|e| e.to_string())?
        }; // <- 锁在此释放
        std::fs::write(&self.path, json).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_settings_receive_new_reminder_defaults() {
        let settings = serde_json::from_str::<Settings>(r#"{"language":"en-US"}"#)
            .expect("旧设置应能反序列化");
        assert!(settings.remind_temp_high);
        assert_eq!(settings.remind_temp_high_at, 45);
        assert!(settings.remind_temp_low);
        assert_eq!(settings.remind_temp_low_at, 5);
        assert!(settings.remind_drain);
        assert_eq!(settings.remind_drain_at, 30);
        assert_eq!(settings.accent_color, "system");
    }

    #[test]
    fn normalization_prevents_overlapping_temperature_thresholds() {
        let settings = Settings {
            remind_temp_high_at: 20,
            remind_temp_low_at: 20,
            ..Settings::default()
        }
        .normalized();
        assert_eq!(settings.remind_temp_high_at, 20);
        assert_eq!(settings.remind_temp_low_at, 18);

        let settings = Settings {
            remind_temp_high_at: 200,
            remind_temp_low_at: -100,
            ..Settings::default()
        }
        .normalized();
        assert_eq!(settings.remind_temp_high_at, 80);
        assert_eq!(settings.remind_temp_low_at, -10);

        let settings = Settings {
            remind_drain_at: 999,
            ..Settings::default()
        }
        .normalized();
        assert_eq!(settings.remind_drain_at, 150);

        let settings = Settings {
            remind_drain_at: -1,
            ..Settings::default()
        }
        .normalized();
        assert_eq!(settings.remind_drain_at, 5);
    }
}
