use anyhow::{ensure, Result};
use super::super::foundation::config;

/// 超导求解配置封装（FR-002）：iso/aniso、μ*、截止与迭代参数。
pub struct SolverConfig<'a> {
    pub cfg: &'a config::Config,
}

impl<'a> SolverConfig<'a> {
    pub fn from_config(cfg: &'a config::Config) -> Result<Self> {
        let sc = &cfg.superconductivity;
        ensure!(sc.mu_star >= 0.0, "configuration: mu_star 必须非负");
        ensure!(sc.frequency_cutoff_ev > 0.0, "configuration: 频率截止必须为正");
        ensure!(sc.mixing_beta > 0.0 && sc.mixing_beta <= 1.0, "configuration: mixing_beta 需在 (0,1]");
        ensure!(sc.convergence_relative > 0.0, "configuration: 收敛相对阈值必须为正");
        ensure!(sc.max_iterations > 0, "configuration: max_iterations 必须为正");
        ensure!(!sc.gap_temperatures_k.is_empty(), "configuration: 缺失 gap_temperatures_k");
        if let config::Mode::Aniso | config::Mode::Both = sc.mode {
            ensure!(sc.aniso.is_some(), "configuration: aniso 模式缺少 aniso 配置块");
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
