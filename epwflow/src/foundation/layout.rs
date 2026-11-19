//! 编号工作区的唯一目录映射；旧工作区不改变原有物理路径。
use super::config::{Config, PhMode};
use anyhow::Result;
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
        || super::readable_state::read(root, super::stages::Step::Scf)
            .is_ok_and(|doc| doc.get("Workspace", "Layout") == Some("Eight numbered stages"))
}
pub fn initialize(cfg: &Config) -> Result<()> {
    super::readable_state::initialize(cfg)
}

pub fn phonon_name(mode: PhMode) -> &'static str {
    match mode {
        PhMode::Nosplit => "4.qe-phonon-nosplit",
        PhMode::Coords => "4.qe-phonon-coords",
        PhMode::Index => "4.qe-phonon-index",
    }
}

pub fn path(root: &Path, relative: &str) -> PathBuf {
    if !enabled(root) {
        return root.join(relative);
    }
    if relative == "scf/snapshot" || relative.starts_with("scf/snapshot/") {
        return root.join("1.qe-scf/out").join(relative.strip_prefix("scf/snapshot").unwrap().trim_start_matches('/'));
    }
    let mode = super::readable_state::mode_in(root).expect("workspace phonon mode initialized at entry");
    let (head, tail) = relative.split_once('/').unwrap_or((relative, ""));
    let ph = format!("4.qe-phonon-{mode}");
    let base = match head {
        "scf" => root.join("1.qe-scf"),
        "nscf" => root.join("2.qe-nscf"),
        "qe-phonon" | "ph_nosplit" => root.join(&ph),
        "ph_init" => root.join(&ph).join("init"),
        "ph" => root.join(&ph).join("q"),
        "coarse" => root.join("6.epw-coarse"),
        "results" => root.join("results"),
        "fine" => root.join("7.epw-fine"),
        "wannier" => root.join("6.epw-coarse"),
        "qe-data" if tail == "save" || tail.starts_with("save/") => {
            return root.join(&ph).join(tail);
        }
        "qe-data" => root.join(&ph),
        "interpolation" if tail == "comparison" || tail.starts_with("comparison/") => {
            return root.join("6.epw-coarse/comparison-native").join(tail.strip_prefix("comparison").unwrap().trim_start_matches('/'));
        }
        "interpolation" => root.join("7.epw-fine"),
        "superconductivity" => root.join("8.epw-superconductivity"),
        "comparison-native" if tail == "electron" || tail.starts_with("electron/") => {
            return root
                .join("5.epw-wannier")
                .join(tail.strip_prefix("electron").unwrap().trim_start_matches('/'));
        }
        "comparison-native" if tail == "phonon" || tail.starts_with("phonon/") => {
            return root.join("6.epw-coarse/comparison-native")
                .join(tail.strip_prefix("phonon").unwrap().trim_start_matches('/'));
        }
        "comparison-native" => root.join("6.epw-coarse").join(head),
        _ => root.join(head),
    };
    if tail.is_empty() {
        base
    } else {
        base.join(tail)
    }
}
