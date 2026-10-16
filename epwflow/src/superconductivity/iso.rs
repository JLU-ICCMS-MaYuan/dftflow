use anyhow::Result;
use super::gap::bcs_gap;
use super::parse::Spectrum;
use super::search::search_tc;
use super::solver_config::SolverConfig;

/// 各向同性求解结果（FR-004）。
#[derive(Debug, Clone)]
pub struct IsoResult {
    pub tc: Option<f64>,
    pub located: bool,
    pub lambda_eff: f64,
    pub omega_log_k: f64,
    pub gap_at_t: Option<f64>,
}

/// Allen-Dynes/McMillan 线性化判据下的临界温度（各向同性，FR-004）。
pub fn crit_temp_iso(
    spectrum: &Spectrum,
    mu_star: f64,
    n_max: usize,
    min_k: f64,
    max_k: f64,
    precision_k: f64,
    max_scores: usize,
) -> Option<f64> {
    search_tc(spectrum, mu_star, n_max, min_k, max_k, precision_k, max_scores)
}

/// 各向同性求解：Tc 搜索 + 低温能隙（FR-004/007）。
pub fn solve(spectrum: &Spectrum, cfg: &SolverConfig) -> Result<IsoResult> {
    let n_max = super::search::default_n_max(&spectrum.frequencies_mev);
    let iso = cfg.cfg.superconductivity.iso.as_ref();
    let (min_k, max_k, precision_k, max_solves) = match iso {
        Some(i) => (i.min_k, i.max_k, i.precision_k, i.max_solves as usize),
        None => (cfg.cfg.superconductivity.gap_temperatures_k[0], 10.0, 0.01, 50),
    };
    let tc = crit_temp_iso(spectrum, cfg.mu_star(), n_max, min_k, max_k, precision_k, max_solves);
    let located = tc.is_some();
    let gap_at_t = match tc {
        Some(tc) => Some(bcs_gap((tc * 0.5).max(min_k), tc)),
        None => None,
    };
    Ok(IsoResult {
        tc,
        located,
        lambda_eff: spectrum.lambda_eff,
        omega_log_k: spectrum.omega_log_k,
        gap_at_t,
    })
}
