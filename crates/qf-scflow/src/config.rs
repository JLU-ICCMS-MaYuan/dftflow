//! scflow 配置解析（对齐 `scflow/config.py: load_scflow_config` 的 `[scflow]` 段语义）。

use anyhow::{bail, Result};
use qf_config::SectionMap;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// 积分方法：梯形累加（Sum）或 Simpson（Simps），对齐 Python `scflow` 的 `integration` 选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integration {
    Sum,
    Simps,
}

#[derive(Debug, Clone)]
pub struct ScflowConfig {
    pub work_dir: PathBuf,
    /// 结果输出目录；`None` 时为 `work_dir/results`（对齐 Python config）。
    pub results_dir: PathBuf,
    pub a2f_file: String,
    pub structure_path: Option<PathBuf>,
    pub mu: f64,
    pub integration: Integration,
    pub resolution: usize,
    pub smoothing_thz: f64,
    pub gaussian_sigma: f64,
    pub structure_tolerance: f64,
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

    /// `"sum" | "simps"` 字符串。
    pub fn integration_str(&self) -> &'static str {
        match self {
            Integration::Sum => "sum",
            Integration::Simps => "simps",
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
        let work_dir = std::fs::canonicalize(&work_dir).unwrap_or(work_dir);

        let a2f_file = section
            .get("a2f_file")
            .and_then(|v| v.as_str())
            .unwrap_or("a2F.dat")
            .to_string();
        let structure_path = section
            .get("structure_path")
            .and_then(|v| v.as_str())
            .map(|s| resolve_path(base, s));

        let mu = get_f64(section, "mu", 0.1);
        let integration = section
            .get("integration")
            .and_then(|v| v.as_str())
            .map(Integration::from_str)
            .unwrap_or(Integration::Sum);
        let resolution = get_i64(section, "resolution", 1000) as usize;
        let smoothing_thz = get_f64(section, "smoothing_thz", 3.0);
        let gaussian_sigma = get_f64(section, "gaussian_sigma", 5.0);
        let structure_tolerance = get_f64(section, "structure_tolerance", 0.2);

        // results_dir：缺省 work_dir/results（Python `load_scflow_config`）。
        let results_dir = match section.get("results_dir").and_then(|v| v.as_str()) {
            Some(raw) => resolve_path(base, raw),
            None => work_dir.join("results"),
        };
        let results_dir = std::fs::canonicalize(&results_dir).unwrap_or(results_dir);

        Ok(ScflowConfig {
            work_dir,
            results_dir,
            a2f_file,
            structure_path,
            mu,
            integration,
            resolution,
            smoothing_thz,
            gaussian_sigma,
            structure_tolerance,
            eliashberg_dim: 24,
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
