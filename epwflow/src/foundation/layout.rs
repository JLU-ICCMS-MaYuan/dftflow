//! 编号工作区的唯一目录映射；旧工作区不改变原有物理路径。
use super::config::{Config, PhMode};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub schema_version: u32,
    pub phonon: PhMode,
}

pub fn enabled(root: &Path) -> bool {
    root.join("epw-state/layout.json").is_file()
}

pub fn initialize(cfg: &Config) -> Result<()> {
    let file = cfg.work_dir.join("epw-state/layout.json");
    if file.exists() {
        let old: Layout = super::store::read(&file)?;
        ensure!(
            old.schema_version == 1,
            "unsupported_version: 工作区布局版本"
        );
    }
    super::store::write(
        &file,
        &Layout {
            schema_version: 1,
            phonon: cfg.phonon.mode,
        },
    )
}

pub fn phonon_name(mode: PhMode) -> &'static str {
    match mode {
        PhMode::Nosplit => "4.qe-phonon-nosplit",
        PhMode::Coords => "4.qe-phonon-coords",
        PhMode::Index => "4.qe-phonon-index",
    }
}

pub fn path(root: &Path, relative: &str) -> PathBuf {
    let file = root.join("epw-state/layout.json");
    if !file.is_file() {
        return root.join(relative);
    }
    // 调用方先校验布局；损坏布局不能静默退回旧路径。
    let layout: Layout = super::store::read(&file).expect("工作区布局必须在入口验证");
    let (head, tail) = relative.split_once('/').unwrap_or((relative, ""));
    let ph = phonon_name(layout.phonon);
    let base = match head {
        "scf" => root.join("1.qe-scf"),
        "nscf" => root.join("2.qe-nscf"),
        "qe-phonon" | "ph_nosplit" => root.join(ph),
        "ph_init" => root.join(ph).join("init"),
        "ph" => root.join(ph).join("q"),
        "coarse" => root.join("6.epw-coarse"),
        "results" => root.join("6.epw-coarse/results"),
        "fine" => root.join("7.epw-fine"),
        "wannier" => root.join("epw-state/coarse"),
        "qe-data" if tail == "save" || tail.starts_with("save/") => {
            return root.join(ph).join(tail);
        }
        "qe-data" => root.join("epw-state/qe-data"),
        "interpolation" if tail == "comparison" || tail.starts_with("comparison/") => {
            return root.join("6.epw-coarse").join(tail);
        }
        "interpolation" => root.join("epw-state/interpolation"),
        "superconductivity" => root.join("8.epw-superconductivity"),
        "native-attempts" => root.join("epw-state/native-attempts"),
        "quality-sources" | "comparison-native" => root.join("6.epw-coarse").join(head),
        "quality-reference.json" | "quality-binding.json" | "quality-report.json" => {
            root.join("6.epw-coarse").join(head)
        }
        _ => root.join(head),
    };
    if tail.is_empty() {
        base
    } else {
        base.join(tail)
    }
}
