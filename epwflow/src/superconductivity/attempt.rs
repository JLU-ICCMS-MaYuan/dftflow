use anyhow::Result;
use super::gap::bcs_gap;
use super::parse::Spectrum;
use super::search::{default_n_max, search_tc};
use super::solver_config::SolverConfig;
use super::super::foundation::config::Mode;

/// 单次尝试结果：单模式单温度（FR-006）。
#[derive(Debug, Clone)]
pub struct AttemptResult {
    pub temperature_k: f64,
    pub mode: String,
    pub converged: bool,
    pub gap_mev: f64,
}

/// 执行一次尝试：给定温度下判别是否低于 Tc 并求能隙（FR-006/007）。
pub fn run_attempt(
    mode: Mode,
    spectrum: &Spectrum,
    cfg: &SolverConfig,
    temperature_k: f64,
) -> Result<AttemptResult> {
    let n_max = default_n_max(&spectrum.frequencies_mev);
    let iso = cfg.cfg.superconductivity.iso.as_ref();
    let (min_k, max_k, precision_k, max_solves) = match iso {
        Some(i) => (i.min_k, i.max_k, i.precision_k, i.max_solves as usize),
        None => (cfg.cfg.superconductivity.gap_temperatures_k[0], 10.0, 0.01, 50),
    };
    let tc = search_tc(spectrum, cfg.mu_star(), n_max, min_k, max_k, precision_k, max_solves);
    let converged = match tc {
        Some(tc) => temperature_k < tc,
        None => false,
    };
    let gap_mev = if converged { bcs_gap(temperature_k, tc.unwrap()) } else { 0.0 };
    let mode_str = match mode {
        Mode::Iso => "iso",
        Mode::Aniso => "aniso",
        Mode::Both => "both",
    };
    Ok(AttemptResult {
        temperature_k,
        mode: mode_str.to_string(),
        converged,
        gap_mev,
    })
}
