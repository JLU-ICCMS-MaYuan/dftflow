use super::super::foundation::config;
use anyhow::{ensure, Context, Result};

/// EPW 5.9 输入适配。能量参考只用于数值换算，不写成未知 namelist 开关。
pub struct EpwConfig<'a> {
    pub cfg: &'a config::Config,
    fermi_ev: Option<f64>,
    species: Option<Vec<String>>,
    bands: Option<Vec<Vec<f64>>>,
}

/// 在生成细网格矩阵元之前，先在固定验证点完成 Wannier/粗网格检查。
#[derive(Clone, Copy)]
pub enum NativeStage {
    WannierOnly,
    ReuseWannier,
    CoarseQuality,
    /// 只重读既有 Wannier 规范，在全部独立点重新插值，不重新选窗。
    RestartQuality,
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
            bands: None,
        })
    }

    /// 必须来自已绑定的 F2 电子态，不能使用材料猜测值或不同来源的零点。
    pub fn with_fermi(mut self, fermi_ev: f64) -> Result<Self> {
        ensure!(fermi_ev.is_finite(), "data_invalid: F2 Fermi 能非有限");
        self.fermi_ev = Some(fermi_ev);
        Ok(self)
    }

    /// 由已核验的 F2 清单提供完整 NSCF 本征值，单位为 eV。
    pub fn with_bands(mut self, bands: &[Vec<f64>]) -> Result<Self> {
        ensure!(qf_wannier::validate_bands(bands)? == self.cfg.qe.nscf.nbnd as usize,
            "data_invalid: 选窗本征值与配置 NSCF 带数不符");
        self.bands = Some(bands.to_vec());
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

    pub fn window_selection(&self) -> Result<qf_wannier::Selection> {
        let w = &self.cfg.wannier;
        ensure!(w.frozen_max_ev.is_none() || w.frozen_window_ev.is_none(), "configuration: 冻结能窗上界重复指定");
        let bounds = qf_wannier::Bounds {
            dis_win_min: w.outer_window_ev.map(|v| v[0]),
            dis_win_max: w.outer_window_ev.map(|v| v[1]),
            dis_froz_min: w.frozen_window_ev.map(|v| v[0]),
            dis_froz_max: w.frozen_window_ev.map(|v| v[1]).or(w.frozen_max_ev),
        };
        let reference = match w.energy_reference {
            config::EnergyReference::QeZero => qf_wannier::Reference::Absolute,
            config::EnergyReference::F2Fermi => qf_wannier::Reference::Fermi,
        };
        let select = match w.selection_mode {
            qf_wannier::SelectionMode::EnergyOnly => qf_wannier::select_energy_only,
            qf_wannier::SelectionMode::FixedPlusProjectability => qf_wannier::select_default,
        };
        let selection = select(
            self.bands.as_deref().context("data_invalid: 选窗缺少绑定的完整 F2 本征值")?,
            self.fermi_ev.context("data_invalid: 选窗缺少绑定的 F2 Fermi 能")?,
            w.num_wann as usize,
            &w.exclude_bands.iter().map(|&v| v as usize).collect::<Vec<_>>(),
            &bounds, reference,
        )?;
        if w.selection_mode == qf_wannier::SelectionMode::EnergyOnly {
            let q = &self.cfg.interpolation.quality;
            let fermi = selection.fermi_ev;
            let lower = fermi + q.energy_window_ev[0].min(-self.cfg.interpolation.fsthick_ev);
            let upper = fermi + q.energy_window_ev[1].max(self.cfg.interpolation.fsthick_ev);
            ensure!(selection.parameters["dis_froz_min"] <= lower
                && upper <= selection.parameters["dis_froz_max"],
                "data_invalid: 纯能量冻结窗未包含质量／EPC 目标窗 [{lower},{upper}] eV");
        }
        Ok(selection)
    }

    pub fn require_window_support(&self) -> Result<()> {
        if self.cfg.wannier.selection_mode == qf_wannier::SelectionMode::FixedPlusProjectability {
            qf_wannier::require_projectability_keywords(&self.cfg.programs.epw.path)?;
        }
        Ok(())
    }

    /// 核对内嵌 W90 实际窗口，不能用生成过 namelist／wdata 代替原生采用证据。
    pub fn check_native_windows(&self, log: &str) -> Result<()> {
        if let Some(expected) = self.cfg.wannier.guiding_centres {
            let pattern = regex::Regex::new(r"Use guiding centre to control phases\s*:\s*([TF])\b")?;
            let actual = pattern.captures_iter(log).last()
                .context("data_invalid: 原生 Wannier 日志缺少 guiding_centres 采用证据")?;
            ensure!((&actual[1] == "T") == expected,
                "data_invalid: 原生 guiding_centres 与当前配置不一致");
        }
        if self.cfg.wannier.selection_mode != qf_wannier::SelectionMode::EnergyOnly {
            return Ok(());
        }
        let outer = super::comparison::parse_outer_window(log)?;
        let inner = super::comparison::parse_frozen_window(log)?
            .context("data_invalid: 纯能量 Wannier 日志缺少 Inner 能窗")?;
        let selection = self.window_selection()?;
        for (name, actual) in [
            ("dis_win_min", outer[0]), ("dis_win_max", outer[1]),
            ("dis_froz_min", inner[0]), ("dis_froz_max", inner[1]),
        ] {
            let expected = selection.parameters[name];
            // 部署 W90 的 disentangle 使用 f10.5；仅允许半个显示间隔与浮点余量。
            let tolerance = 0.000005 + 8.0 * f64::EPSILON * expected.abs().max(1.0);
            ensure!((actual - expected).abs() <= tolerance,
                "data_invalid: 原生 {name}={actual} eV 与选窗记录 {expected} eV 不一致");
        }
        Ok(())
    }

    fn windows(&self) -> Result<Vec<(String, f64)>> {
        Ok(self.window_selection()?.parameters.into_iter()
            .filter(|(name, _)| !name.starts_with("dis_proj_")).collect())
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
        if let Some(enabled) = w.guiding_centres {
            s.push_str(&format!("guiding_centres = {enabled}\n"));
        }
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
        if w.selection_mode == qf_wannier::SelectionMode::FixedPlusProjectability {
            s.push_str(&format!("dis_proj_min = {}\ndis_proj_max = {}\n", qf_wannier::DIS_PROJ_MIN, qf_wannier::DIS_PROJ_MAX));
            s.push_str(qf_wannier::PROJECTABILITY_SWITCH);
            s.push('\n');
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
        let wannier_only = matches!(stage, NativeStage::WannierOnly);
        let elph = if wannier_only { ".false." } else { ".true." };
        let mut s = format!("DFTFlow EPW 5.9 workflow input\n&inputepw\n prefix = {}\n outdir = {}\n dvscf_dir = {}\n ep_coupling = .true.\n elph = {elph}\n etf_mem = 0\n nbndsub = {}\n iverbosity = 2\n", quoted(&self.cfg.qe.prefix)?, quoted(outdir)?, quoted(dvscf_dir)?, w.num_wann);
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
            NativeStage::WannierOnly | NativeStage::CoarseQuality => {
                s.push_str(" epwread = .false.\n epwwrite = .true.\n wannierize = .true.\n ephwrite = .false.\n eliashberg = .false.\n");
                if wannier_only {
                    // readin 会无条件打开非空 filkf/filqf；拟合不消费插值点。
                    s.push_str(" band_plot = .false.\n");
                } else {
                    s.push_str(" band_plot = .true.\n selecqread = .true.\n filkf = 'kpoints.dat'\n filqf = 'qpoints.dat'\n");
                }
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
                let mut data = Vec::new();
                if let Some(enabled) = w.guiding_centres {
                    data.push(format!("guiding_centres = {enabled}"));
                }
                for (name, value) in self.windows()? {
                    if w.selection_mode == qf_wannier::SelectionMode::EnergyOnly && name == "dis_win_min" {
                        // 部署的 EPW 5.9 会清空 namelist 下界；由受控 wdata 传给内嵌 W90。
                        data.push(format!("{name} = {value:.16e}"));
                    } else {
                        s.push_str(&format!(" {name} = {value:.16e}\n"));
                    }
                }
                if w.selection_mode == qf_wannier::SelectionMode::FixedPlusProjectability {
                    data.extend([
                        qf_wannier::PROJECTABILITY_SWITCH.to_string(),
                        format!("dis_proj_min = {}", qf_wannier::DIS_PROJ_MIN),
                        format!("dis_proj_max = {}", qf_wannier::DIS_PROJ_MAX),
                    ]);
                }
                data.extend([
                    format!("conv_tol = {:.16e}", w.localization.tolerance_angstrom2),
                    format!("conv_window = {}", w.localization.window_iterations),
                ]);
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
            NativeStage::ReuseWannier => {
                s.push_str(" epwread = .false.\n epwwrite = .true.\n wannierize = .false.\n ephwrite = .false.\n eliashberg = .false.\n band_plot = .true.\n selecqread = .true.\n filkf = 'kpoints.dat'\n filqf = 'qpoints.dat'\n");
            }
            NativeStage::RestartQuality => {
                s.push_str(" epwread = .true.\n epwwrite = .false.\n epbread = .false.\n wannierize = .false.\n ephwrite = .false.\n eliashberg = .false.\n band_plot = .true.\n selecqread = .true.\n filkf = 'kpoints.dat'\n filqf = 'qpoints.dat'\n");
            }
            NativeStage::FineMatrices => {
                s.push_str(" epwread = .true.\n epwwrite = .false.\n wannierize = .false.\n ephwrite = .true.\n eliashberg = .true.\n liso = .false.\n laniso = .false.\n mp_mesh_k = .true.\n");
                // EPW 缺省使用 300 K；显式绑定首个固定能隙温度以便复核。
                s.push_str(&format!(" nstemp = 1\n temps = {:.16e}\n", self.cfg.superconductivity.gap_temperatures_k[0]));
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
        Self::write_points(dir, &quality.k_points_crystal, &quality.q_points_crystal)
    }

    pub(super) fn write_points(dir: &std::path::Path, k: &[[f64; 3]], q: &[[f64; 3]]) -> Result<()> {
        for (name, points) in [
            ("kpoints.dat", k),
            ("qpoints.dat", q),
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
        let count = q.len();
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
