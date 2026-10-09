//! scflow 配置解析（对齐 `scflow/config.py: load_scflow_config` 的 `[scflow]` 段语义）。

use crate::Integration;
use anyhow::{bail, Result};
use qf_config::SectionMap;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ScflowConfig {
    pub work_dir: PathBuf,
    pub a2f_file: String,
    pub mu: f64,
    pub integration: Integration,
    pub resolution: usize,
    pub smoothing_thz: f64,
    pub gaussian_sigma: f64,
    pub eliashberg_dim: usize,
}

impl Integration {
    pub fn from_str(s: &str) -> Integration {
        if s == "simps" {
            Integration::Simps
        } else {
            Integration::Sum
        }
    }
}

impl ScflowConfig {
    /// 从 `[scflow]` 配置段构造，路径相对配置文件所在目录（`base`）解析。
    pub fn from_section(base: &Path, section: &SectionMap) -> Result<Self> {
        let work_dir_raw = section
            .get("work_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let work_dir = resolve_path(base, work_dir_raw);

        let a2f_file = section
            .get("a2f_file")
            .and_then(|v| v.as_str())
            .unwrap_or("a2F.dat")
            .to_string();

        let mu = get_f64(section, "mu", 0.1);
        let integration = section
            .get("integration")
            .and_then(|v| v.as_str())
            .map(Integration::from_str)
            .unwrap_or(Integration::Sum);
        let resolution = get_i64(section, "resolution", 1000) as usize;
        let smoothing_thz = get_f64(section, "smoothing_thz", 3.0);
        let gaussian_sigma = get_f64(section, "gaussian_sigma", 5.0);
        let eliashberg_dim = get_i64(section, "eliashberg_dim", 24) as usize;

        Ok(ScflowConfig {
            work_dir,
            a2f_file,
            mu,
            integration,
            resolution,
            smoothing_thz,
            gaussian_sigma,
            eliashberg_dim,
        })
    }
}

fn resolve_path(base: &Path, raw: &str) -> PathBuf {
    let p = Path::new(raw);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

fn get_f64(section: &SectionMap, key: &str, default: f64) -> f64 {
    match section.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(default),
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(default),
        _ => default,
    }
}

fn get_i64(section: &SectionMap, key: &str, default: i64) -> i64 {
    match section.get(key) {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(default),
        Some(Value::String(s)) => s.parse::<i64>().unwrap_or(default),
        _ => default,
    }
}

/// 从 `qf_config::Config` 直接构造（便于 CLI 复用同一份 input.toml）。
pub fn from_config(base: &Path, cfg: &qf_config::Config) -> Result<ScflowConfig> {
    if cfg.scflow.is_empty() {
        bail!("input.toml 缺少 [scflow] 配置段（需要 work_dir / a2f_file / mu 等）");
    }
    ScflowConfig::from_section(base, &cfg.scflow)
}
