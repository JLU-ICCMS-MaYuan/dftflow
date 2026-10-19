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
    #[serde(default)]
    pub artifact_root: Option<PathBuf>,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
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
            matches!(m.schema_version, 1 | 2),
            "unsupported_version: qe-data manifest schema_version 必须为 1 或 2"
        );
        if m.schema_version == 2 {
            m.verify()?;
        }
        Ok(m)
    }

    /// 原生交付逐文件重验；版本 1 仅保留历史读取，不凭 valid 标签进入下游。
    pub fn verify(&self) -> anyhow::Result<()> {
        use anyhow::{ensure, Context};
        ensure!(
            self.schema_version == 2 && self.validation.all_passed() && !self.artifacts.is_empty(),
            "data_invalid: 缺少版本 2 原生 QE 产物身份"
        );
        let root = self
            .artifact_root
            .as_ref()
            .context("data_invalid: QE 清单缺 artifact_root")?;
        let canonical_root = root.canonicalize()?;
        let mut roles = std::collections::BTreeSet::new();
        for a in &self.artifacts {
            ensure!(
                roles.insert(&a.role)
                    && !a.relative_path.is_absolute()
                    && a.relative_path
                        .components()
                        .all(|c| matches!(c, std::path::Component::Normal(_))),
                "data_invalid: QE 角色重复或路径非法"
            );
            let p = root.join(&a.relative_path);
            ensure!(
                p.canonicalize()?.starts_with(&canonical_root)
                    && !std::fs::symlink_metadata(&p)?.file_type().is_symlink()
                    && std::fs::metadata(&p)?.len() == a.byte_size
                    && a.byte_size > 0
                    && super::super::foundation::store::hash_file(&p)? == a.sha256,
                "input_conflict: QE 产物变化 {}",
                p.display()
            );
        }
        for required in ["dyn0", "phsave_control", "scf_snapshot_identity"] {
            ensure!(
                roles.contains(&required.to_string()),
                "data_invalid: QE 缺少必需角色 {required}"
            );
        }
        for q in &self.qpoints {
            for role in ["dyn", "patterns", "dvscf"] {
                ensure!(
                    roles.contains(&format!("{role}_q{}", q.global_index)),
                    "data_invalid: QE 缺少 q 角色"
                );
            }
        }
        let scf = self.scf.as_ref().context("data_invalid: QE 清单缺 SCF")?;
        let states = self
            .states
            .as_ref()
            .context("data_invalid: QE 清单缺 NSCF")?;
        ensure!(
            scf.total_energy_ry.is_finite()
                && !self.qpoints.is_empty()
                && self.qpoints.len() == self.phonon_files.len(),
            "data_invalid: QE 清单内容不完整"
        );
        let scf_xml = scf
            .snapshot_dir
            .join(
                states
                    .wavefunctions
                    .file_name()
                    .context("data_invalid: NSCF 保存树缺名称")?,
            )
            .join("data-file-schema.xml");
        let geometry = super::xml::Geometry::read(&scf_xml)?;
        let mut actual_states =
            super::nscf::parse_xml(&states.wavefunctions.join("data-file-schema.xml"))?;
        actual_states.k_grid = states.k_grid;
        ensure!(
            serde_json::to_value(&actual_states)? == serde_json::to_value(states)?,
            "data_invalid: NSCF 清单数值与实际 XML 不符"
        );
        super::nscf::validate_grid(states, states.k_grid, states.nbnd, &geometry)?;
        for (stage, dir) in [
            ("scf", scf.snapshot_dir.clone()),
            (
                "nscf",
                states
                    .wavefunctions
                    .parent()
                    .context("data_invalid: NSCF 保存树缺父目录")?
                    .to_path_buf(),
            ),
        ] {
            let actual = super::save::files(&dir)?
                .iter()
                .map(|f| dir.join(&f.relative_path))
                .collect::<std::collections::BTreeSet<_>>();
            let declared = self
                .artifacts
                .iter()
                .filter(|a| a.producer_stage == stage && a.role != "scf_snapshot_identity")
                .map(|a| root.join(&a.relative_path))
                .collect::<std::collections::BTreeSet<_>>();
            ensure!(
                actual == declared,
                "data_invalid: QE {stage} 保存树文件集合与清单不一致"
            );
        }
        for (index, q) in self.qpoints.iter().enumerate() {
            ensure!(
                q.global_index == index + 1
                    && (q.local_index == 1 || q.local_index == q.global_index),
                "data_invalid: QE 全局／局部 q 编号非法"
            );
            let matched = self
                .phonon_files
                .iter()
                .filter(|r| r.q_id == q.global_index)
                .collect::<Vec<_>>();
            ensure!(matched.len() == 1, "data_invalid: QE 物理 q 响应映射不唯一");
            let r = matched[0];
            ensure!(
                r.control_metadata
                    == root.join(
                        &self
                            .artifacts
                            .iter()
                            .find(|a| a.role == "phsave_control")
                            .unwrap()
                            .relative_path
                    ),
                "data_invalid: QE 全局 control 来源不符"
            );
            for (role, path) in [
                ("dyn", &r.dyn_file),
                ("patterns", &r.patterns),
                (
                    "dvscf",
                    r.dvscf.as_ref().context("data_invalid: QE 缺 dvscf")?,
                ),
            ] {
                let artifact = self
                    .artifacts
                    .iter()
                    .find(|a| a.role == format!("{role}_q{}", q.global_index))
                    .unwrap();
                ensure!(
                    root.join(&artifact.relative_path) == *path,
                    "data_invalid: QE 响应路径与角色身份不符"
                );
            }
            let mut normalized = q.clone();
            normalized.local_index = q.global_index;
            super::native_ph::verify(r, &normalized, &geometry)?;
        }
        super::native_ph::control(&self.phonon_files[0].control_metadata, &self.qpoints)?;
        Ok(())
    }
}

impl Validation {
    pub fn all_passed(&self) -> bool {
        self.status == "valid" && !self.checks.is_empty() && self.checks.iter().all(|c| c.passed)
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
