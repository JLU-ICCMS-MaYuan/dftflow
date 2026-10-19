use super::super::foundation::config;
use anyhow::{ensure, Result};

/// 超导求解配置封装（FR-002）：iso/aniso、μ*、截止与迭代参数。
pub struct SolverConfig<'a> {
    pub cfg: &'a config::Config,
}

impl<'a> SolverConfig<'a> {
    pub fn from_config(cfg: &'a config::Config) -> Result<Self> {
        cfg.validate()?;
        let sc = &cfg.superconductivity;
        ensure!(
            sc.gap_temperatures_k.iter().all(|t| *t <= 999.99),
            "configuration: EPW 5.9 温度文件格式只支持到 999.99 K"
        );
        for tc in [sc.iso.as_ref(), sc.aniso.as_ref()].into_iter().flatten() {
            ensure!(
                tc.max_k <= 999.99,
                "configuration: EPW 5.9 搜索温度文件格式只支持到 999.99 K"
            );
        }
        Ok(SolverConfig { cfg })
    }

    pub fn mode(&self) -> config::Mode {
        self.cfg.superconductivity.mode
    }
    pub fn mu_star(&self) -> f64 {
        self.cfg.superconductivity.mu_star
    }
    pub fn frequency_cutoff_ev(&self) -> f64 {
        self.cfg.superconductivity.frequency_cutoff_ev
    }
    pub fn max_iterations(&self) -> usize {
        self.cfg.superconductivity.max_iterations as usize
    }
    pub fn convergence_relative(&self) -> f64 {
        self.cfg.superconductivity.convergence_relative
    }
    pub fn mixing_beta(&self) -> f64 {
        self.cfg.superconductivity.mixing_beta
    }
    pub fn mixing_history(&self) -> usize {
        self.cfg.superconductivity.mixing_history as usize
    }
    pub fn initial_gaps_mev(&self) -> Vec<f64> {
        self.cfg.superconductivity.initial_gaps_mev.to_vec()
    }
    pub fn gap_temperatures_k(&self) -> Vec<f64> {
        self.cfg.superconductivity.gap_temperatures_k.clone()
    }
    pub fn spectrum_index(&self) -> usize {
        self.cfg.superconductivity.spectrum_index as usize
    }
}
