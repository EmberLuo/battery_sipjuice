//! CPU/GPU power limits — 强力省电模式使用 cpufreq/devfreq 上限约束。

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

const CPUFREQ_BASE: &str = "/sys/devices/system/cpu/cpufreq";
const DEVFREQ_BASE: &str = "/sys/class/devfreq";
const SUPER_LIMIT_PERCENT: i64 = 45;
const PRIVILEGED_FLAG: &str = "--battery-sipjuice-cpu-power";
const ACTION_ENABLE: &str = "enable";
const ACTION_DISABLE: &str = "disable";

#[derive(Serialize, Clone)]
pub struct CpuPolicy {
    pub name: String,
    pub affected_cpus: String,
    pub current_freq: Option<i64>,
    pub max_freq: Option<i64>,
    pub hardware_max_freq: Option<i64>,
    pub target_freq: Option<i64>,
}

#[derive(Serialize, Clone)]
pub struct CpuPowerState {
    pub supported: bool,
    pub active: bool,
    pub policies: Vec<CpuPolicy>,
    pub gpus: Vec<GpuPolicy>,
    pub message: String,
}

#[derive(Serialize, Clone)]
pub struct GpuPolicy {
    pub name: String,
    pub current_freq: Option<i64>,
    pub max_freq: Option<i64>,
    pub hardware_max_freq: Option<i64>,
    pub target_freq: Option<i64>,
}

pub fn state() -> CpuPowerState {
    let policies = collect_policies();
    let gpus = collect_gpus();
    if policies.is_empty() && gpus.is_empty() {
        return CpuPowerState {
            supported: false,
            active: false,
            policies,
            gpus,
            message: "未检测到可调节的 CPU/GPU 频率策略".to_string(),
        };
    }

    let cpu_active = policies.iter().all(|p| match (p.max_freq, p.target_freq) {
        (Some(max), Some(target)) => max <= target,
        _ => false,
    });
    let gpu_active = gpus.iter().all(|g| match (g.max_freq, g.target_freq) {
        (Some(max), Some(target)) => max <= target,
        _ => false,
    });
    let active = cpu_active && gpu_active;
    CpuPowerState {
        supported: true,
        active,
        policies,
        gpus,
        message: if active {
            "超级省电模式已限制 CPU/GPU 频率上限".to_string()
        } else {
            "CPU/GPU 频率上限处于默认或更高性能状态".to_string()
        },
    }
}

pub fn set_super_saver(enabled: bool) -> Result<CpuPowerState, String> {
    if collect_policy_dirs().is_empty() && collect_gpu_dirs().is_empty() {
        return Err("未检测到可调节的 CPU/GPU 频率策略".to_string());
    }

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let action = if enabled {
        ACTION_ENABLE
    } else {
        ACTION_DISABLE
    };
    let output = Command::new("pkexec")
        .arg(exe)
        .arg(PRIVILEGED_FLAG)
        .arg(action)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "系统未安装 pkexec，无法请求管理员权限".to_string()
            } else {
                e.to_string()
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "管理员授权已取消或执行失败".to_string()
        } else {
            stderr
        });
    }

    Ok(state())
}

pub fn handle_privileged_cli<I>(args: I) -> Result<bool, String>
where
    I: IntoIterator<Item = String>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) != Some(PRIVILEGED_FLAG) {
        return Ok(false);
    }

    match args.get(2).map(String::as_str) {
        Some(ACTION_ENABLE) => apply_limits(true),
        Some(ACTION_DISABLE) => apply_limits(false),
        _ => Err("未知的 CPU 省电模式操作".to_string()),
    }?;
    Ok(true)
}

fn apply_limits(enabled: bool) -> Result<(), String> {
    let policies = collect_policy_dirs();
    let gpus = collect_gpu_dirs();
    if policies.is_empty() && gpus.is_empty() {
        return Err("未检测到可调节的 CPU/GPU 频率策略".to_string());
    }

    let mut errors = Vec::new();
    for policy in policies {
        let target = if enabled {
            target_freq(&policy)
        } else {
            read_i64(&policy, "cpuinfo_max_freq")
        };
        let Some(target) = target else {
            errors.push(format!("{} 缺少目标频率", policy.display()));
            continue;
        };
        if let Err(err) = std::fs::write(policy.join("scaling_max_freq"), target.to_string()) {
            errors.push(format!("{}: {}", policy.display(), err));
        }
    }
    for gpu in gpus {
        let target = if enabled {
            target_gpu_freq(&gpu)
        } else {
            gpu_hardware_max_freq(&gpu)
        };
        let Some(target) = target else {
            errors.push(format!("{} 缺少目标频率", gpu.display()));
            continue;
        };
        if let Err(err) = std::fs::write(gpu.join("max_freq"), target.to_string()) {
            errors.push(format!("{}: {}", gpu.display(), err));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn collect_policies() -> Vec<CpuPolicy> {
    collect_policy_dirs()
        .into_iter()
        .filter_map(|dir| {
            let name = dir.file_name()?.to_string_lossy().into_owned();
            Some(CpuPolicy {
                name,
                affected_cpus: read_raw(&dir, "affected_cpus").unwrap_or_else(|| "?".to_string()),
                current_freq: read_i64(&dir, "scaling_cur_freq"),
                max_freq: read_i64(&dir, "scaling_max_freq"),
                hardware_max_freq: read_i64(&dir, "cpuinfo_max_freq"),
                target_freq: target_freq(&dir),
            })
        })
        .collect()
}

fn collect_policy_dirs() -> Vec<PathBuf> {
    let mut dirs = std::fs::read_dir(CPUFREQ_BASE)
        .ok()
        .into_iter()
        .flat_map(|it| it.filter_map(Result::ok))
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("policy"))
                && p.join("scaling_max_freq").exists()
        })
        .collect::<Vec<_>>();
    dirs.sort_by_key(|p| p.file_name().map(|n| n.to_os_string()));
    dirs
}

fn collect_gpus() -> Vec<GpuPolicy> {
    collect_gpu_dirs()
        .into_iter()
        .filter_map(|dir| {
            let name = dir.file_name()?.to_string_lossy().into_owned();
            Some(GpuPolicy {
                name,
                current_freq: read_i64(&dir, "cur_freq"),
                max_freq: read_i64(&dir, "max_freq"),
                hardware_max_freq: gpu_hardware_max_freq(&dir),
                target_freq: target_gpu_freq(&dir),
            })
        })
        .collect()
}

fn collect_gpu_dirs() -> Vec<PathBuf> {
    let mut dirs = std::fs::read_dir(DEVFREQ_BASE)
        .ok()
        .into_iter()
        .flat_map(|it| it.filter_map(Result::ok))
        .map(|e| e.path())
        .filter(|p| {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            p.join("max_freq").exists() && p.join("cur_freq").exists() && name.contains("gpu")
        })
        .collect::<Vec<_>>();
    dirs.sort_by_key(|p| p.file_name().map(|n| n.to_os_string()));
    dirs
}

fn target_freq(policy: &Path) -> Option<i64> {
    let max = read_i64(policy, "cpuinfo_max_freq")?;
    let min = read_i64(policy, "cpuinfo_min_freq").unwrap_or(0);
    let target = (max * SUPER_LIMIT_PERCENT / 100).max(min);
    let freqs = read_raw(policy, "scaling_available_frequencies")
        .map(|s| {
            s.split_whitespace()
                .filter_map(|v| v.parse::<i64>().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if freqs.is_empty() {
        return Some(target);
    }
    freqs
        .iter()
        .copied()
        .filter(|f| *f <= target)
        .max()
        .or_else(|| freqs.iter().copied().min())
}

fn target_gpu_freq(gpu: &Path) -> Option<i64> {
    let max = gpu_hardware_max_freq(gpu)?;
    let min = read_i64(gpu, "min_freq").unwrap_or(0);
    let target = (max * SUPER_LIMIT_PERCENT / 100).max(min);
    let freqs = available_freqs(gpu);

    if freqs.is_empty() {
        return Some(target);
    }
    freqs
        .iter()
        .copied()
        .filter(|f| *f <= target)
        .max()
        .or_else(|| freqs.iter().copied().min())
}

fn gpu_hardware_max_freq(gpu: &Path) -> Option<i64> {
    let freqs = available_freqs(gpu);
    freqs
        .into_iter()
        .max()
        .or_else(|| read_i64(gpu, "max_freq"))
}

fn available_freqs(dir: &Path) -> Vec<i64> {
    read_raw(dir, "available_frequencies")
        .map(|s| {
            s.split_whitespace()
                .filter_map(|v| v.parse::<i64>().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn read_raw(dir: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(dir.join(name))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn read_i64(dir: &Path, name: &str) -> Option<i64> {
    read_raw(dir, name)?.parse().ok()
}
