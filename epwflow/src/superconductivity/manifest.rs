use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use super::super::foundation::store;
use super::super::qe_data::manifest::{Validation, Check};

/// 单次尝试记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptRecord {
    pub temperature_k: f64,
    pub mode: String,
    pub converged: bool,
    pub gap_mev: f64,
}

/// F4 交付清单（FR-012）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuperconductivityManifest {
    pub schema_version: u32,
    pub run_id: String,
    pub interpolation_digest: String,
    pub mode: String,
    pub tc: Option<f64>,
    pub located: bool,
    pub converged: bool,
    pub gaps_mev: Vec<f64>,
    pub attempts: Vec<AttemptRecord>,
    pub validation: Validation,
}

impl SuperconductivityManifest {
    pub fn load(path: &Path) -> Result<Self> {
        let m: SuperconductivityManifest = store::read(path)?;
        if m.schema_version != 1 {
            anyhow::bail!("data_invalid: 超导清单 schema_version={} 不支持", m.schema_version);
        }
        Ok(m)
    }
}

/// 便捷校验条目。
pub fn check(name: &str, passed: bool, detail: &str) -> Check {
    Check { name: name.to_string(), passed, detail: detail.to_string() }
}

/// 占位以满足 BTreeMap 导入（避免未使用告警）。
#[allow(dead_code)]
pub fn _thresholds() -> BTreeMap<String, f64> {
    BTreeMap::new()
}
