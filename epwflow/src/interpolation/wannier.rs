//! EPW 内置 Wannier90 的原生收敛与最终展宽；不再运行独立重复链。
use super::super::foundation::store;
use anyhow::{ensure, Context, Result};
use regex::Regex;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct WannierResult {
    pub num_iter: usize,
    pub converged: bool,
    pub max_spread_angstrom2: f64,
    pub wannier_digest: String,
}

pub fn parse_wannier_log(log: &Path) -> Result<WannierResult> {
    let txt = std::fs::read_to_string(log)?;
    ensure!(!Regex::new(r"(?i)convergence\s+(?:was\s+)?not\s+(?:reached|achieved)|not\s+converged|error in routine")?.is_match(&txt),
        "data_invalid: Wannier 明确未收敛或报错");
    ensure!(
        txt.contains("<<< Wannierisation convergence criteria satisfied >>>"),
        "data_invalid: 缺少原生 Wannier 局域化收敛证据"
    );
    let disentangled = Regex::new(r"Using band disentanglement\s*:\s*T")?.is_match(&txt);
    ensure!(
        !disentangled || txt.contains("<<< Disentanglement convergence criteria satisfied >>>"),
        "data_invalid: 缺少原生解纠缠收敛证据"
    );
    let final_state = txt
        .rsplit_once("Final State")
        .context("data_invalid: 缺少 Wannier 最终状态")?
        .1;
    let spread_re = Regex::new(r"(?m)^\s*WF centre and spread\s+(\d+)\s+\([^\n]*\)\s+(\S+)\s*$")?;
    let mut ids = std::collections::BTreeSet::new();
    let mut max_spread = 0f64;
    for c in spread_re.captures_iter(final_state) {
        let id: usize = c[1].parse()?;
        let spread: f64 = c[2].replace(['D', 'd'], "e").parse()?;
        ensure!(
            id == ids.len() + 1 && ids.insert(id) && spread.is_finite() && spread > 0.0,
            "data_invalid: Wannier 最终 spread 或 WF 编号无效"
        );
        max_spread = max_spread.max(spread);
    }
    ensure!(!ids.is_empty(), "data_invalid: 缺少完整最终 Wannier 展宽");
    let iter_re = Regex::new(r"(?m)^\s*Cycle:\s*(\d+)\s*$")?;
    let num_iter = iter_re
        .captures_iter(&txt)
        .last()
        .context("data_invalid: 缺 Wannier 迭代记录")?[1]
        .parse()?;
    Ok(WannierResult {
        num_iter,
        converged: true,
        max_spread_angstrom2: max_spread,
        wannier_digest: store::hash_file(log)?,
    })
}

pub fn check_native(log: &Path, nwann: usize) -> Result<WannierResult> {
    let result = parse_wannier_log(log)?;
    let txt = std::fs::read_to_string(log)?;
    let final_state = txt
        .rsplit_once("Final State")
        .context("data_invalid: 缺 Wannier 最终状态")?
        .1;
    ensure!(
        final_state
            .lines()
            .filter(|line| line.trim_start().starts_with("WF centre and spread"))
            .count()
            == nwann,
        "data_invalid: 最终 Wannier 函数数量不符"
    );
    Ok(result)
}
