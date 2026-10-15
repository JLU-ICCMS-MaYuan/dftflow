use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 产物记录：角色、相对路径、尺寸、哈希、生产阶段、输入指纹、格式版本。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub role: String,
    pub relative_path: PathBuf,
    pub byte_size: u64,
    pub sha256: String,
    pub producer_stage: String,
    pub input_digest: String,
    pub format_version: String,
}

/// 校验结论：状态、逐项检查、单位、阈值与证据；未通过不发布。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Validation {
    pub status: String,
    pub checks: Vec<Check>,
    pub units: std::collections::BTreeMap<String, String>,
    pub thresholds: std::collections::BTreeMap<String, f64>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

/// q 点映射：全局序号、局部序号、数字目录、两种坐标、基底与 dyn0 来源。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QPoint {
    pub global_index: usize,
    pub local_index: usize,
    pub directory: String,
    pub crystal: [f64; 3],
    pub cartesian: [f64; 3],
    pub alat: f64,
    pub reciprocal_basis: [[f64; 3]; 3],
    pub source_dyn0: PathBuf,
}

/// 电子态集合：完整均匀 k 点、权重、带数、本征能量、波函数布局与 Fermi。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElectronicStates {
    pub k_grid: [u32; 3],
    pub kpoints: Vec<[f64; 3]>,
    pub weights: Vec<f64>,
    pub nbnd: usize,
    pub eigenvalues: Vec<Vec<f64>>,
    pub wavefunctions: PathBuf,
    pub fermi: f64,
    pub layout: String,
}

/// PH 数据组：同一物理 q 的动力学矩阵、响应势、扰动模式与元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhResponse {
    pub q_id: usize,
    pub dyn_file: PathBuf,
    pub patterns: PathBuf,
    pub dvscf: Option<PathBuf>,
    pub control_metadata: PathBuf,
    pub format: String,
    pub perturbations: usize,
}

/// QE 数据清单：指纹、SCF、电子态、q 点与 PH 文件；完整校验后才发布。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QeManifest {
    pub schema_version: u32,
    pub run_id: String,
    pub fingerprints: Fingerprints,
    pub scf: Option<ScfRecord>,
    pub states: Option<ElectronicStates>,
    pub qpoints: Vec<QPoint>,
    pub phonon_files: Vec<PhResponse>,
    pub validation: Validation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fingerprints {
    pub structure_sha256: String,
    pub pseudo_sha256: std::collections::BTreeMap<String, String>,
    pub program_sha256: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScfRecord {
    pub total_energy_ry: f64,
    pub attempt_count: u32,
    pub success_count: u32,
    pub snapshot_dir: PathBuf,
}

impl QeManifest {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let m: Self = super::super::foundation::store::read(path)?;
        anyhow::ensure!(
            m.schema_version == 1,
            "unsupported_version: qe-data manifest schema_version 必须为 1"
        );
        Ok(m)
    }
}

impl Validation {
    pub fn all_passed(&self) -> bool {
        self.status == "valid" && self.checks.iter().all(|c| c.passed)
    }
    pub fn check(&mut self, name: &str, passed: bool, detail: &str) {
        self.checks.push(Check {
            name: name.to_string(),
            passed,
            detail: detail.to_string(),
        });
        if !passed {
            self.status = "invalid".to_string();
        }
    }
}
