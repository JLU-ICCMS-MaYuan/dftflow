use anyhow::Result;
use super::super::foundation::config;

/// EPW/Wannier90 输入渲染（FR-002）：统一配置、明确能量参考。
pub struct EpwConfig<'a> {
    pub cfg: &'a config::Config,
}

impl<'a> EpwConfig<'a> {
    pub fn from_config(cfg: &'a config::Config) -> Result<Self> {
        // FR-002：能量参考必须显式声明，不得隐式默认。
        match cfg.wannier.energy_reference {
            config::EnergyReference::QeZero | config::EnergyReference::F2Fermi => {}
        }
        Ok(EpwConfig { cfg })
    }

    /// wannier90.win：投影、能带窗口、收敛参数。
    pub fn wannier90_input(&self) -> String {
        let w = &self.cfg.wannier;
        let proj = w.projections.join(", ");
        let mut s = String::new();
        s.push_str(&format!("num_wann = {}\n", w.num_wann));
        s.push_str(&format!("projections = {}\n", proj));
        if !w.exclude_bands.is_empty() {
            s.push_str(&format!("exclude_bands = {}\n", w.exclude_bands.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")));
        }
        let loc = &w.localization;
        s.push_str(&format!("num_iter = {}\n", loc.max_iterations));
        s.push_str(&format!("conv_tol = {}\n", loc.tolerance_angstrom2));
        s.push_str(&format!("energy_reference = {:?}\n", self.cfg.wannier.energy_reference));
        s
    }

    /// pw2wannier90.x 输入：衔接 QE 与 Wannier90。
    pub fn pw2wannier90_input(&self, outdir: &str) -> String {
        format!(
            "&inputpp\n  prefix = 'scf'\n  outdir = '{outdir}'\n  seedname = 'scf'\n  write_amn = .true.\n  write_mmn = .true.\n/\n"
        )
    }

    /// EPW 粗网格输入：读 wannier 数据、粗网格 k/q、写 ephmat。
    pub fn epw_coarse_input(&self, outdir: &str) -> String {
        let i = &self.cfg.interpolation;
        format!(
            "&inputepw\n  prefix = 'scf'\n  outdir = '{outdir}'\n  seedname = 'scf'\n  epwread = .false.\n  epwwrite = .true.\n  wannierize = .true.\n  nk1 = {} nk2 = {} nk3 = {}\n  nq1 = {} nq2 = {} nq3 = {}\n  elecselfen = .false.\n  phonselfen = .true.\n  ephwrite = .true.\n  degaussw = {}\n  fsthick = {}\n  energy_reference = {:?}\n/\n",
            i.k_grid[0], i.k_grid[1], i.k_grid[2],
            i.q_grid[0], i.q_grid[1], i.q_grid[2],
            i.degaussw_ev, i.fsthick_ev, self.cfg.wannier.energy_reference
        )
    }

    /// EPW 细网格输入：插值、谱函数、低频规则。
    pub fn epw_fine_input(&self, outdir: &str, smearing_mev: f64) -> String {
        let i = &self.cfg.interpolation;
        format!(
            "&inputepw\n  prefix = 'scf'\n  outdir = '{outdir}'\n  seedname = 'scf'\n  epwread = .true.\n  epwwrite = .false.\n  wannierize = .false.\n  nk1 = {} nk2 = {} nk3 = {}\n  nq1 = {} nq2 = {} nq3 = {}\n  elecselfen = .true.\n  phonselfen = .true.\n  ephwrite = .false.\n  degaussw = {}\n  fsthick = {}\n  vme \
             = .true.\n  int_smearing = {}\n  eps_acoustic = {}\n  energy_reference = {:?}\n/\n",
            i.k_grid[0] * 2, i.k_grid[1] * 2, i.k_grid[2] * 2,
            i.q_grid[0] * 2, i.q_grid[1] * 2, i.q_grid[2] * 2,
            i.degaussw_ev, i.fsthick_ev, smearing_mev / 1000.0, i.eps_acoustic_cm1, self.cfg.wannier.energy_reference
        )
    }
}
