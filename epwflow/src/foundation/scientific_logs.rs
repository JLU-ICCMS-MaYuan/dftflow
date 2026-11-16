//! 科学日志主名；只对五种历史命名提供有界只读兼容。
use std::path::{Path, PathBuf};

pub fn legacy_stem(stem: &str) -> Option<&'static str> {
    match stem {
        "elebanddata" => Some("bands"),
        "eletdos" => Some("dos"),
        "epw_wannier" | "epw_coarse" | "epw_fine" => Some("epw"),
        _ => None,
    }
}

/// 新名存在时必须核验新名，不能用旧日志掩盖新运行失败。
pub fn output(dir: &Path, stem: &str) -> PathBuf {
    let current = dir.join(format!("{stem}.out"));
    if current.symlink_metadata().is_ok() {
        return current;
    }
    legacy_stem(stem)
        .map(|old| dir.join(format!("{old}.out")))
        .filter(|old| old.exists())
        .unwrap_or(current)
}
