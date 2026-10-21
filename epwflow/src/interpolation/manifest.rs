use super::super::foundation::store;
pub use super::super::qe_data::manifest::{Artifact, Check, Validation};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// 粗网格数据：EPW 粗网格插值所需全部角色（FR-005/007）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoarseData {
    pub kpoints: usize,
    pub qpoints: usize,
    pub nwann: usize,
    pub ephmat_role: String,
    pub artifacts: Vec<Artifact>,
}

/// 细网格数据：k/q 网格、能量、频率、权重、kqmap、ephmat 分片（FR-007）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FineData {
    pub kpoints: usize,
    pub qpoints: usize,
    pub energies: Vec<f64>,
    pub frequencies: Vec<f64>,
    pub weights: Vec<f64>,
    pub kqmap: usize,
    pub ephmat_shards: usize,
    pub layout: String,
    pub artifacts: Vec<Artifact>,
}

/// 谱函数：每个展宽分列（FR-008）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spectrum {
    pub smearing_id: String,
    pub energy_mev: Vec<f64>,
    pub a2f: Vec<f64>,
    pub lambda_integral: f64,
    pub lambda_sum: f64,
    pub cutoff_ev: f64,
}

/// F3 交付清单：f2_digest、wannier_digest、粗/细/谱与模式就绪（FR-012）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterpolationManifest {
    pub schema_version: u32,
    pub run_id: String,
    pub f2_digest: String,
    pub wannier_digest: String,
    pub coarse: Option<CoarseData>,
    pub fine: Option<FineData>,
    pub spectra: Vec<Spectrum>,
    pub mode_readiness: BTreeMap<String, bool>,
    pub validation: Validation,
}

impl InterpolationManifest {
    pub fn load(path: &Path) -> Result<Self> {
        let m: InterpolationManifest = store::read(path)?;
        if m.schema_version != 2 {
            bail!(
                "data_invalid: 插值清单 schema_version={} 不支持",
                m.schema_version
            );
        }
        Ok(m)
    }
}

/// 便捷：构造一条校验条目。
pub fn check(name: &str, passed: bool, detail: &str) -> Check {
    Check {
        name: name.to_string(),
        passed,
        detail: detail.to_string(),
    }
}
