use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use super::super::foundation::store;
use super::super::qe_data::manifest::Validation;

/// 六组合覆盖表单元格（3 PH × 2 求解，FR-004）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonCell {
    pub ph_mode: String,
    pub solver_mode: String,
    pub reference: String,
    pub input_hashes: BTreeMap<String, String>,
    pub metrics: BTreeMap<String, f64>,
    pub tolerances: BTreeMap<String, f64>,
    pub status: String,
    pub evidence: Vec<String>,
}

/// 交付结果包（results/manifest.json，FR-001/003）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultsBundle {
    pub schema_version: u32,
    pub run_id: String,
    pub cells: Vec<ComparisonCell>,
    pub tc: Option<f64>,
    pub gaps_mev: Vec<f64>,
    pub lambda_eff: f64,
    pub omega_log_k: f64,
    pub validation: Validation,
}

impl ResultsBundle {
    pub fn load(path: &Path) -> Result<Self> {
        let m: Self = store::read(path)?;
        if m.schema_version != 1 {
            anyhow::bail!("data_invalid: 交付清单 schema_version={} 不支持", m.schema_version);
        }
        Ok(m)
    }
}

/// 便捷校验条目。
pub fn cell_ok(ph: &str, solver: &str, metrics: BTreeMap<String, f64>) -> ComparisonCell {
    ComparisonCell {
        ph_mode: ph.to_string(),
        solver_mode: solver.to_string(),
        reference: "epwpy".to_string(),
        input_hashes: BTreeMap::new(),
        metrics,
        tolerances: BTreeMap::new(),
        status: "ok".to_string(),
        evidence: vec![],
    }
}
