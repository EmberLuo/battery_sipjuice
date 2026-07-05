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
    /// 开机自启动。
    pub autostart: bool,
    /// 静默启动 —— 启动时不显示窗口，只显示托盘。
    pub silent_start: bool,
    /// 关闭按钮行为。
    pub close_action: CloseAction,
    /// 超级省电模式的用户期望状态；实际是否生效以 sysfs 实时状态为准。
    pub super_power_saver: bool,

    /// 低电量提醒：放电中电量 ≤ remind_charge_at 时提醒接电源。
    pub remind_charge: bool,
    /// 低电量提醒阈值 (%)。
    pub remind_charge_at: i64,
    /// 高电量提醒：充电中电量 ≥ remind_unplug_at 时提醒拔电源。
    pub remind_unplug: bool,
    /// 高电量提醒阈值 (%)。
    pub remind_unplug_at: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "zh-CN".to_string(),
            theme: "system".to_string(),
            autostart: false,
            silent_start: false,
            close_action: CloseAction::Ask,
            super_power_saver: false,
            remind_charge: true,
            remind_charge_at: 30,
            remind_unplug: true,
            remind_unplug_at: 80,
        }
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
            .unwrap_or_default();
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
        // 持锁仅用于更新内存副本并序列化，写盘前释放。
        let json = {
            let mut guard = self.inner.lock().map_err(|_| "设置锁中毒".to_string())?;
            *guard = settings;
            serde_json::to_string_pretty(&*guard).map_err(|e| e.to_string())?
        }; // <- 锁在此释放
        std::fs::write(&self.path, json).map_err(|e| e.to_string())
    }
}
