use super::super::foundation::config;
use anyhow::{ensure, Context, Result};

/// EPW 5.9 输入适配。能量参考只用于数值换算，不写成未知 namelist 开关。
pub struct EpwConfig<'a> {
    pub cfg: &'a config::Config,
    fermi_ev: Option<f64>,
    species: Option<Vec<String>>,
}

/// 在生成细网格矩阵元之前，先在固定验证点完成 Wannier/粗网格检查。
#[derive(Clone, Copy)]
pub enum NativeStage {
    CoarseQuality,
    FineMatrices,
}

fn quoted(value: &str) -> Result<String> {
    ensure!(
        !value.contains(['\n', '\r', '\0']),
        "configuration: 原生输入字符串包含控制字符"
    );
    Ok(format!("'{}'", value.replace('\'', "''")))
}

impl<'a> EpwConfig<'a> {
    pub fn from_config(cfg: &'a config::Config) -> Result<Self> {
        Ok(Self {
            cfg,
            fermi_ev: None,
            species: None,
        })
    }

    /// 必须来自已绑定的 F2 电子态，不能使用材料猜测值或不同来源的零点。
    pub fn with_fermi(mut self, fermi_ev: f64) -> Result<Self> {
        ensure!(fermi_ev.is_finite(), "data_invalid: F2 Fermi 能非有限");
        self.fermi_ev = Some(fermi_ev);
        Ok(self)
    }

    /// 元素顺序来自共享结构解析器，不能按赝势映射的字母顺序填写 amass。
    pub fn with_species(mut self, species: &[String]) -> Result<Self> {
        let unique: std::collections::BTreeSet<_> = species.iter().collect();
        ensure!(
            unique.len() == species.len()
                && !species.is_empty()
                && species.iter().all(|el| self.cfg.pseudos.contains_key(el))
                && self.cfg.atomic_masses.keys().all(|el| unique.contains(el)),
            "data_invalid: EPW 原子质量缺少完整、有序的结构元素绑定"
        );
        self.species = Some(species.to_vec());
        Ok(self)
    }

    fn window_offset(&self) -> Result<f64> {
        match self.cfg.wannier.energy_reference {
            config::EnergyReference::QeZero => Ok(0.0),
            config::EnergyReference::F2Fermi => self
                .fermi_ev
                .context("data_invalid: 相对 Fermi 能窗缺少绑定的 F2 Fermi 能"),
        }
    }

    fn windows(&self) -> Result<Vec<(String, f64)>> {
        let w = &self.cfg.wannier;
        let offset = self.window_offset()?;
        let mut values = Vec::new();
        for (name, window) in [
            ("dis_win", w.outer_window_ev),
            ("dis_froz", w.frozen_window_ev),
        ] {
            if let Some([min, max]) = window {
                values.push((format!("{name}_min"), min + offset));
                values.push((format!("{name}_max"), max + offset));
            }
        }
        if let Some(max) = w.frozen_max_ev {
            ensure!(
                w.frozen_window_ev.is_none(),
                "configuration: 冻结能窗上界重复指定"
            );
            values.push(("dis_froz_max".into(), max + offset));
        }
        ensure!(
            values.iter().all(|(_, v)| v.is_finite()),
            "configuration: 能窗换算溢出"
        );
        Ok(values)
    }

    /// 仅渲染 Wannier 参数片段；EPW 主路径调用其内置 Wannier90 库。
    pub fn wannier90_input(&self) -> Result<String> {
        let w = &self.cfg.wannier;
        let mut s = format!(
            "num_wann = {}\nnum_iter = {}\nconv_tol = {:.16e}\nconv_window = {}\n",
            w.num_wann,
            w.localization.max_iterations,
            w.localization.tolerance_angstrom2,
            w.localization.window_iterations
        );
        s.push_str("begin projections\n");
        for p in &w.projections {
            ensure!(
                !p.contains(['\n', '\r', '\0']),
                "configuration: 投影含控制字符"
            );
            s.push_str(p);
            s.push('\n');
        }
        s.push_str("end projections\n");
        if !w.exclude_bands.is_empty() {
            s.push_str(&format!(
                "exclude_bands = {}\n",
                w.exclude_bands
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        for (name, value) in self.windows()? {
            s.push_str(&format!("{name} = {value:.16e}\n"));
        }
        if let Some(d) = &w.disentanglement {
            s.push_str(&format!(
                "dis_num_iter = {}\ndis_conv_tol = {:.16e}\ndis_conv_window = {}\n",
                d.max_iterations, d.tolerance_angstrom2, d.window_iterations
            ));
        }
        Ok(s)
    }

    pub fn pw2wannier90_input(&self, outdir: &str) -> Result<String> {
        let prefix = quoted(&self.cfg.qe.prefix)?;
        Ok(format!("&inputpp\n prefix = {prefix}\n outdir = {}\n seedname = {prefix}\n write_amn = .true.\n write_mmn = .true.\n/\n", quoted(outdir)?))
    }

    pub fn epw_coarse_input(&self, outdir: &str) -> Result<String> {
        self.native_input(
            NativeStage::CoarseQuality,
            outdir,
            "./save",
            self.cfg.interpolation.phonon_smearings_mev[0],
        )
    }

    pub fn epw_fine_input(&self, outdir: &str, smearing_mev: f64) -> Result<String> {
        self.native_input(NativeStage::FineMatrices, outdir, "./save", smearing_mev)
    }

    /// coarse 使用 QE NSCF/PH 网格；fine 严格使用配置网格，不再乘二推测。
    /// FineMatrices 写完整 freq/egnv/ikmap/ephmat，并只计算谱、不解温度能隙。
    pub fn native_input(
        &self,
        stage: NativeStage,
        outdir: &str,
        dvscf_dir: &str,
        smearing_mev: f64,
    ) -> Result<String> {
        config::positive(smearing_mev, "interpolation.phonon_smearings_mev")?;
        ensure!(dvscf_dir.len() < 100, "configuration: EPW 5.9 dvscf_dir 长度必须小于 100 字节；请相对实际工作目录定位数据，避免 Fortran 静默截断");
        let i = &self.cfg.interpolation;
        let w = &self.cfg.wannier;
        let mut s = format!("DFTFlow EPW 5.9 workflow input\n&inputepw\n prefix = {}\n outdir = {}\n dvscf_dir = {}\n ep_coupling = .true.\n elph = .true.\n etf_mem = 0\n nbndsub = {}\n iverbosity = 2\n", quoted(&self.cfg.qe.prefix)?, quoted(outdir)?, quoted(dvscf_dir)?, w.num_wann);
        if !self.cfg.atomic_masses.is_empty() {
            let species = self
                .species
                .as_ref()
                .context("data_invalid: 显式原子质量必须绑定结构元素顺序")?;
            for (n, el) in species.iter().enumerate() {
                let mass =
                    qf_input::species::get_atomic_mass(el, self.cfg.atomic_masses.get(el).copied());
                s.push_str(&format!(" amass({}) = {mass:.16e}\n", n + 1));
            }
        }
        for (name, grid) in [
            ("nk", self.cfg.qe.nscf.k_grid),
            ("nq", self.cfg.phonon.q_grid),
        ] {
            for (axis, n) in grid.iter().enumerate() {
                s.push_str(&format!(" {name}{} = {n}\n", axis + 1));
            }
        }
        s.push_str(&format!(" fsthick = {:.16e}\n degaussw = {:.16e}\n degaussq = {:.16e}\n nqstep = {}\n nqsmear = 1\n eps_acoustic = {:.16e}\n elecselfen = .false.\n phonselfen = .false.\n a2f = .false.\n", i.fsthick_ev, i.degaussw_ev, smearing_mev, i.spectral_points, i.eps_acoustic_cm1));
        let ngaussw = match i.electron_smearing {
            config::Smearing::Gaussian => 0,
            config::Smearing::Mp => 1,
            config::Smearing::Mv => -1,
            config::Smearing::Fd => -99,
        };
        s.push_str(&format!(" ngaussw = {ngaussw}\n"));
        match i.fermi_source {
            config::FermiSource::F2 => s.push_str(&format!(
                " efermi_read = .true.\n fermi_energy = {:.16e}\n",
                self.fermi_ev
                    .context("data_invalid: 缺少绑定的 F2 Fermi 能")?
            )),
            config::FermiSource::FineFixedElectrons => s.push_str(" efermi_read = .false.\n"),
        }
        match stage {
            NativeStage::CoarseQuality => {
                s.push_str(" epwread = .false.\n epwwrite = .true.\n wannierize = .true.\n ephwrite = .false.\n eliashberg = .false.\n band_plot = .true.\n selecqread = .true.\n filkf = 'kpoints.dat'\n filqf = 'qpoints.dat'\n");
                s.push_str(&format!(" num_iter = {}\n", w.localization.max_iterations));
                for (j, p) in w.projections.iter().enumerate() {
                    s.push_str(&format!(" proj({}) = {}\n", j + 1, quoted(p)?));
                }
                if !w.exclude_bands.is_empty() {
                    s.push_str(&format!(
                        " bands_skipped = {}\n",
                        quoted(&format!(
                            "exclude_bands = {}",
                            w.exclude_bands
                                .iter()
                                .map(u32::to_string)
                                .collect::<Vec<_>>()
                                .join(",")
                        ))?
                    ));
                }
                for (name, value) in self.windows()? {
                    s.push_str(&format!(" {name} = {value:.16e}\n"));
                }
                let mut data = vec![
                    format!("conv_tol = {:.16e}", w.localization.tolerance_angstrom2),
                    format!("conv_window = {}", w.localization.window_iterations),
                ];
                if let Some(d) = &w.disentanglement {
                    data.extend([
                        format!("dis_num_iter = {}", d.max_iterations),
                        format!("dis_conv_tol = {:.16e}", d.tolerance_angstrom2),
                        format!("dis_conv_window = {}", d.window_iterations),
                    ]);
                }
                for (j, line) in data.iter().enumerate() {
                    s.push_str(&format!(" wdata({}) = {}\n", j + 1, quoted(line)?));
                }
            }
            NativeStage::FineMatrices => {
                s.push_str(" epwread = .true.\n epwwrite = .false.\n wannierize = .false.\n ephwrite = .true.\n eliashberg = .true.\n liso = .false.\n laniso = .false.\n mp_mesh_k = .true.\n");
                for (name, grid) in [("nkf", i.k_grid), ("nqf", i.q_grid)] {
                    for (axis, n) in grid.iter().enumerate() {
                        s.push_str(&format!(" {name}{} = {n}\n", axis + 1));
                    }
                }
            }
        }
        s.push_str("/\n");
        Ok(s)
    }

    /// 独立验证点按原配置写出，显式选全 q，避免 Fermi 窗口筛选留下假零频。
    pub fn write_quality_points(&self, dir: &std::path::Path) -> Result<()> {
        let quality = &self.cfg.interpolation.quality;
        for (name, points) in [
            ("kpoints.dat", &quality.k_points_crystal),
            ("qpoints.dat", &quality.q_points_crystal),
        ] {
            ensure!(
                !points.is_empty() && points.iter().flatten().all(|v| v.is_finite()),
                "configuration: 独立质量验证点缺失或非有限"
            );
            let mut text = format!("{} crystal\n", points.len());
            for [x, y, z] in points {
                text.push_str(&format!(
                    "{x:.17e} {y:.17e} {z:.17e} {:.17e}\n",
                    1.0 / points.len() as f64
                ));
            }
            std::fs::write(dir.join(name), text)?;
        }
        let count = quality.q_points_crystal.len();
        std::fs::write(
            dir.join("selecq.fmt"),
            format!(
                "{count}\n{count}\n{}\n",
                (1..=count)
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        )?;
        Ok(())
    }
}
