//! TOML 配置加载与递归深合并（复刻 `public/config_templates.py` 的 `_merge_dict_in_place`）。
//!
//! 合并规则：仅当 base[key] 与 override[key] **均为表** 时递归合并；否则 override 胜出；
//! override 中的 Null（对应 TOML 缺失/显式 null）**不覆盖** base。
//!
//! 嵌套配置段（`qe_phonon.scf`、`qe_phonon.ph` 等）用 [`SectionMap`]
//! （`BTreeMap<String, serde_json::Value>`）承载，以忠实对应 Python 端按 key 直接读取字典的行为。

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// 任意 QE 配置子段：key -> 值（数字/字符串/布尔/列表/嵌套表）。
pub type SectionMap = BTreeMap<String, Value>;

pub const LATTICE_MODE_IBRAV_ANGSTROM: &str = "ibrav_angstrom";
pub const LATTICE_MODE_IBRAV_CELLDM: &str = "ibrav_celldm";
pub const LATTICE_MODE_CELL_PARAMETERS: &str = "cell_parameters";
pub const VALID_LATTICE_MODES: &[&str] = &[
    LATTICE_MODE_IBRAV_ANGSTROM,
    LATTICE_MODE_IBRAV_CELLDM,
    LATTICE_MODE_CELL_PARAMETERS,
];

/// 不允许用户手动填写的 QE 晶格参数 key（由 lattice_mode 自动生成）。
pub const MANUAL_LATTICE_KEYS: &[&str] = &[
    "ibrav", "A", "B", "C", "cosAB", "cosAC", "cosBC", "celldm(1)", "celldm(2)", "celldm(3)",
    "celldm(4)", "celldm(5)", "celldm(6)",
];

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub pp_dir: Option<String>,
    #[serde(default)]
    pub qe: QeSection,
    #[serde(default)]
    pub slurm: SlurmSection,
    #[serde(default)]
    pub qe_phonon: QePhononSection,
    #[serde(default)]
    pub qe_scf: SectionMap,
    #[serde(default)]
    pub qe_nscf: SectionMap,
    #[serde(default)]
    pub qe_eband: SectionMap,
    #[serde(default)]
    pub qe_eledos: SectionMap,
    #[serde(default)]
    pub scflow: SectionMap,
    #[serde(default)]
    pub k_points: SectionMap,
    #[serde(default)]
    pub vasp: VaspSection,
    #[serde(default)]
    pub vasp_scf: SectionMap,
    #[serde(default)]
    pub vasp_scf_converge: SectionMap,
    #[serde(default)]
    pub vasp_eband: SectionMap,
    #[serde(default)]
    pub vasp_eledos: SectionMap,
    #[serde(default)]
    pub vasp_fermi: SectionMap,
    #[serde(default)]
    pub vasp_cohp_uniform: SectionMap,
    #[serde(default)]
    pub vasp_cohp_fatband: SectionMap,
    #[serde(default)]
    pub vasp_locpot: SectionMap,
    #[serde(default)]
    pub vasp_charge: SectionMap,
    #[serde(default)]
    pub vasp_elf: SectionMap,
    #[serde(default)]
    pub vasp_opt: SectionMap,
    #[serde(default)]
    pub opt_stage_templates: SectionMap,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct QeSection {
    #[serde(default)]
    pub executable_path: Option<String>,
    #[serde(default)]
    pub lattice_mode: Option<String>,
    #[serde(default)]
    pub pseudo_dir: Option<String>,
}

impl QeSection {
    pub fn executable_path(&self) -> &str {
        self.executable_path.as_deref().unwrap_or("")
    }
    pub fn lattice_mode(&self) -> &str {
        self.lattice_mode.as_deref().unwrap_or(LATTICE_MODE_IBRAV_ANGSTROM)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SlurmSection {
    #[serde(default)]
    pub header: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct VaspSection {
    #[serde(default)]
    pub executable_path: Option<String>,
    #[serde(default)]
    pub potcar_dir: Option<String>,
    #[serde(default)]
    pub chgcar_path: Option<String>,
    #[serde(default)]
    pub wavecar_path: Option<String>,
    #[serde(default)]
    pub lobster_executable_path: Option<String>,
}

impl VaspSection {
    pub fn executable_path(&self) -> &str {
        self.executable_path.as_deref().unwrap_or("")
    }
    pub fn potcar_dir(&self) -> &str {
        self.potcar_dir.as_deref().unwrap_or("")
    }
    pub fn lobster_executable_path(&self) -> &str {
        self.lobster_executable_path.as_deref().unwrap_or("")
    }
    pub fn chgcar_path(&self) -> &str {
        self.chgcar_path.as_deref().unwrap_or("")
    }
    pub fn wavecar_path(&self) -> &str {
        self.wavecar_path.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct QePhononSection {
    /// 为 EPW 生成逐 q 响应势与 XML 动力学矩阵，并归档到 save。
    #[serde(default)]
    pub retain_epw_data: bool,
    #[serde(default)]
    pub work_dir: Option<String>,
    /// `[qe_phonon].prefix`（复刻 `input_deck/prefix.resolve_prefix` 的优先来源）。
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub scf: SectionMap,
    #[serde(default)]
    pub ph: SectionMap,
    #[serde(default)]
    pub q2r: SectionMap,
    #[serde(default)]
    pub phband: SectionMap,
    #[serde(default)]
    pub phdos: SectionMap,
}

impl Config {
    /// 解析 lattice_mode 并校验合法性（复刻 `resolve_lattice_mode`）。
    pub fn lattice_mode(&self) -> Result<&str> {
        let mode = self.qe.lattice_mode();
        if !VALID_LATTICE_MODES.contains(&mode) {
            anyhow::bail!(
                "[qe].lattice_mode = {mode:?} 不合法，允许值: {}",
                VALID_LATTICE_MODES.join(", ")
            );
        }
        Ok(mode)
    }

    /// 校验用户未在 input.toml 中手动指定 QE 晶格参数（复刻 `validate_no_manual_lattice_params`）。
    pub fn validate_no_manual_lattice_params(&self) -> Result<()> {
        let mut conflicts = Vec::new();
        for (name, section) in [
            ("qe_scf", &self.qe_scf),
            ("qe_nscf", &self.qe_nscf),
            ("qe_eband", &self.qe_eband),
            ("qe_eledos", &self.qe_eledos),
            ("qe_phonon.scf", &self.qe_phonon.scf),
            ("qe_phonon.ph", &self.qe_phonon.ph),
            ("qe_phonon.q2r", &self.qe_phonon.q2r),
            ("qe_phonon.phband", &self.qe_phonon.phband),
            ("qe_phonon.phdos", &self.qe_phonon.phdos),
        ] {
            for (k, v) in section {
                let child_path = format!("{name}.{k}");
                if MANUAL_LATTICE_KEYS.contains(&k.as_str()) {
                    conflicts.push(child_path.clone());
                }
                walk_manual(v, &child_path, &mut conflicts);
            }
        }
        if !conflicts.is_empty() {
            anyhow::bail!(
                "不允许在 input.toml 中手动指定 QE 晶格参数；请删除这些 key，由 lattice_mode 自动生成: {}",
                conflicts.join(", ")
            );
        }
        Ok(())
    }
}

fn walk_manual(value: &Value, path: &str, conflicts: &mut Vec<String>) {
    if let Value::Object(map) = value {
        for (k, child) in map {
            let child_path = if path.is_empty() {
                k.clone()
            } else {
                format!("{path}.{k}")
            };
            if MANUAL_LATTICE_KEYS.contains(&k.as_str()) {
                conflicts.push(child_path.clone());
            }
            walk_manual(child, &child_path, conflicts);
        }
    }
}

/// 内置最小默认值，用户配置在其上递归覆盖。
const DEFAULTS_TOML: &str = r#"
[qe]
lattice_mode = "ibrav_angstrom"

[qe_phonon.scf]
calculation = "scf"
restart_mode = "from_scratch"
outdir = "./tmp"
"#;

/// 加载配置：读取 `path`（input.toml），在内置默认值上递归深合并，并解析 `pp_dir` 路径。
/// 若 `path` 为 None，仅返回默认值。
pub fn load(path: Option<&str>, config_file: Option<&str>) -> Result<Config> {
    let mut base = defaults_json()?;
    if let Some(p) = path {
        let text = std::fs::read_to_string(p).with_context(|| format!("read config {p}"))?;
        let user_toml: toml::Value =
            toml::from_str(&text).with_context(|| format!("parse TOML {p}"))?;
        let user_json = serde_json::to_value(user_toml).context("convert user TOML to JSON")?;
        merge_recursive(&mut base, &user_json);
    }
    hoist_root_vasp_keys(&mut base);
    let mut config: Config =
        serde_json::from_value(base).context("deserialize merged config")?;
    config.pp_dir = resolve_pp_dir_path(config.pp_dir, config_file);
    config.validate_no_manual_lattice_params()?;
    Ok(config)
}

/// 根级 VASP 键（复刻 Python 直接读 `config["potcar_dir"]` 等根键的行为）：
/// 用户配置把这些键写在 TOML 顶层时，合并后提升进 `[vasp]` 段（根键优先，
/// 覆盖 `[vasp]` 内同名键，保持与 Python `self.config.get("potcar_dir", "")` 语义一致）。
const ROOT_VASP_KEYS: [&str; 4] = [
    "potcar_dir",
    "chgcar_path",
    "wavecar_path",
    "lobster_executable_path",
];

fn hoist_root_vasp_keys(base: &mut Value) {
    let Some(root) = base.as_object_mut() else {
        return;
    };
    let hoisted: Vec<(String, Value)> = ROOT_VASP_KEYS
        .iter()
        .filter_map(|k| {
            root.get(*k)
                .filter(|v| !v.is_null())
                .map(|v| (k.to_string(), v.clone()))
        })
        .collect();
    if hoisted.is_empty() {
        return;
    }
    let vasp = root
        .entry("vasp".to_string())
        .or_insert_with(|| Value::Object(Default::default()));
    if let Some(vo) = vasp.as_object_mut() {
        for (k, v) in hoisted {
            vo.insert(k, v);
        }
    }
}

fn defaults_json() -> Result<Value> {
    let toml_val: toml::Value = toml::from_str(DEFAULTS_TOML).context("parse DEFAULTS_TOML")?;
    serde_json::to_value(toml_val).context("convert defaults to JSON")
}

/// 递归深合并 `overrides` 到 `base`（复刻 `_merge_dict_in_place`）。
pub fn merge_recursive(base: &mut Value, overrides: &Value) {
    match (base, overrides) {
        (Value::Object(b), Value::Object(o)) => {
            for (k, v) in o {
                if v.is_null() {
                    continue; // None 不覆盖
                }
                match b.get_mut(k) {
                    Some(bv) => merge_recursive(bv, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (base_slot, v) => {
            *base_slot = v.clone();
        }
    }
}

/// 解析 `pp_dir`：绝对路径直接用；相对路径依次尝试当前目录、配置文件所在目录及其各级父目录（复刻 `resolve_pp_dir_path`）。
pub fn resolve_pp_dir_path(pp_dir: Option<String>, config_file: Option<&str>) -> Option<String> {
    let pp = pp_dir?;
    let expanded = expand_user(&pp);
    if Path::new(&expanded).is_absolute() {
        return Some(expanded);
    }
    let mut candidates: Vec<String> = Vec::new();
    candidates.push(std::fs::canonicalize(".").map(|p| p.join(&expanded).to_string_lossy().into_owned()).unwrap_or(expanded.clone()));
    if let Some(cfg) = config_file {
        let cfg_dir = Path::new(cfg).parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let mut cur = cfg_dir.clone();
        loop {
            candidates.push(cur.join(&expanded).to_string_lossy().into_owned());
            let parent = cur.parent().map(|p| p.to_path_buf());
            match parent {
                Some(p) if p != cur => cur = p,
                _ => break,
            }
        }
    }
    for c in &candidates {
        if Path::new(c).is_dir() {
            return Some(c.clone());
        }
    }
    candidates.into_iter().next()
}

/// 展开 `~` 为用户主目录（复刻 `os.path.expanduser` 的最小子集）。
fn expand_user(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    } else if path == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return home;
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_recursive_nested_and_null_skip() {
        let mut base: Value = serde_json::json!({"qe": {"kmesh": 0.04, "ecutwfc": 60}, "a": 1});
        let over: Value =
            serde_json::json!({"qe": {"ecutwfc": 80, "pseudo": "x"}, "a": null});
        merge_recursive(&mut base, &over);
        assert_eq!(base["qe"]["kmesh"], serde_json::json!(0.04)); // untouched
        assert_eq!(base["qe"]["ecutwfc"], serde_json::json!(80)); // overridden
        assert_eq!(base["qe"]["pseudo"], serde_json::json!("x")); // added
        assert_eq!(base["a"], serde_json::json!(1)); // null did not overwrite
    }

    #[test]
    fn hoist_root_vasp_keys_into_vasp_section() {
        // 根键存在时提升进 [vasp]，且根键优先覆盖 [vasp] 同名键
        let mut base: Value = serde_json::json!({
            "potcar_dir": "potcar_lib",
            "wavecar_path": "vasp_scf/WAVECAR",
            "vasp": {"executable_path": "mpirun -np 4 vasp_std", "potcar_dir": "/cluster/pot"}
        });
        hoist_root_vasp_keys(&mut base);
        assert_eq!(base["vasp"]["potcar_dir"], serde_json::json!("potcar_lib"));
        assert_eq!(base["vasp"]["wavecar_path"], serde_json::json!("vasp_scf/WAVECAR"));
        assert_eq!(
            base["vasp"]["executable_path"],
            serde_json::json!("mpirun -np 4 vasp_std")
        );
        // chgcar_path / lobster_executable_path 根级缺省时不注入
        assert!(base["vasp"].get("chgcar_path").is_none());
        assert!(base["vasp"].get("lobster_executable_path").is_none());
    }

    #[test]
    fn hoist_creates_vasp_section_when_absent() {
        let mut base: Value = serde_json::json!({"potcar_dir": "potcar_lib"});
        hoist_root_vasp_keys(&mut base);
        assert_eq!(base["vasp"]["potcar_dir"], serde_json::json!("potcar_lib"));
        // 根级键为 null 时不提升
        let mut base2: Value = serde_json::json!({"potcar_dir": null});
        hoist_root_vasp_keys(&mut base2);
        assert!(base2.get("vasp").is_none());
    }
}

/// 固定声子工作目录及配置相对路径；其他用户字段保留。
pub fn phonon_snapshot_text(path: &str, workspace: &str) -> Result<String> {
    let mut value: toml::Value = toml::from_str(&std::fs::read_to_string(path)?)?;
    let cfg = load(Some(path), Some(path))?;
    let root = value.as_table_mut().context("配置必须为 TOML 表")?;
    if let Some(pp) = cfg.pp_dir { root.insert("pp_dir".into(), toml::Value::String(pp)); }
    let ph = root.entry("qe_phonon").or_insert_with(|| toml::Value::Table(Default::default())).as_table_mut().context("qe_phonon 必须为表")?;
    ph.insert("work_dir".into(), toml::Value::String(workspace.into()));
    Ok(toml::to_string_pretty(&value)?)
}
