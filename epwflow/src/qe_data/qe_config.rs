//! epwflow 严格配置到共享 qeflow 输入库的适配层。
use super::super::foundation::config;
use anyhow::{ensure, Context, Result};
use qf_config::SectionMap;
use qf_crystal::Structure;
use qf_input::pw::PwParams;
use serde_json::json;

/// 结构解析、原子质量与 PW/PH 格式由共享库维护；此处只映射 EPW 参数。
pub struct QeSettings<'a> {
    pub config: &'a config::Config,
    pub cell: Structure,
}

impl<'a> QeSettings<'a> {
    pub fn from_config(config: &'a config::Config) -> Result<Self> {
        let cell =
            qf_crystal::parse_poscar(config.structure.to_str().context("结构路径不是 UTF-8")?)?;
        ensure!(
            cell.elements.len() == cell.counts.len(),
            "data_invalid: 元素与数量不匹配"
        );
        for el in &cell.elements {
            ensure!(
                config.pseudos.contains_key(el),
                "configuration: 缺少 {el} 的显式赝势"
            );
            ensure!(
                qf_input::species::ATOMIC_MASSES
                    .iter()
                    .any(|(e, _)| e == el),
                "configuration: {el} 没有受支持的原子质量"
            );
        }
        Ok(Self { config, cell })
    }

    fn pw_input(&self, outdir: &str, nscf: bool) -> Result<String> {
        let qe = &self.config.qe;
        let mut shared = qf_config::Config::default();
        shared.qe.lattice_mode = Some(qf_config::LATTICE_MODE_CELL_PARAMETERS.into());
        // 赝势使用已解析的绝对路径，保持显式文件身份，不在目录内自动猜选。
        shared.pp_dir = Some("/".into());
        let mut species = String::from("ATOMIC_SPECIES\n");
        for el in &self.cell.elements {
            let path = &self.config.pseudos[el];
            let name = path.to_str().context("赝势路径不是 UTF-8")?;
            ensure!(
                path.is_absolute() && !name.chars().any(char::is_whitespace),
                "configuration: QE 赝势卡片需要不含空白的绝对路径；请先固定到受控赝势目录"
            );
            species.push_str(&format!(
                "  {el} {:.4} {name}\n",
                qf_input::species::get_atomic_mass(el, None)
            ));
        }
        let mut params = PwParams::default();
        params.control.insert(
            "calculation".into(),
            json!(if nscf { "nscf" } else { "scf" }),
        );
        params.control.insert("outdir".into(), json!(outdir));
        params.system.insert("ecutwfc".into(), json!(qe.ecutwfc_ry));
        params.system.insert("ecutrho".into(), json!(qe.ecutrho_ry));
        params
            .system
            .insert("occupations".into(), json!(qe.occupations));
        if let Some(v) = qe.degauss_ry {
            params.system.insert("degauss".into(), json!(v));
        }
        if let Some(v) = &qe.smearing {
            params.system.insert("smearing".into(), json!(v));
        }
        let points = if nscf {
            params.system.insert("nbnd".into(), json!(qe.nscf.nbnd));
            // EPW 5.9 allocates symmetry-sized arrays before restoring crystal
            // symmetries. A nosym NSCF save can therefore cause an out-of-bounds
            // write in its coarse-grid stage. Keep the crystal metadata and
            // supply the full explicit mesh below; validate actual output
            // coverage rather than relying on nosym to establish completeness.
            params.system.insert("nosym".into(), json!(false));
            params.system.insert("noinv".into(), json!(false));
            params
                .electrons
                .insert("conv_thr".into(), json!(qe.nscf.conv_thr_ry));
            qf_input::pw::uniform_kpoints_text(qe.nscf.k_grid.map(i64::from), true)?
        } else {
            params
                .electrons
                .insert("conv_thr".into(), json!(qe.scf.conv_thr_ry));
            let [a, b, c] = qe.scf.k_grid;
            let [x, y, z] = qe.scf.k_shift;
            format!("K_POINTS automatic\n  {a} {b} {c} {x} {y} {z}\n")
        };
        qf_input::pw::write_pw_input_with_species(
            &shared, &self.cell, &params, &qe.prefix, &points, 17, &species,
        )
    }

    pub fn scf_input(&self, outdir: &str) -> Result<String> {
        self.pw_input(outdir, false)
    }
    pub fn nscf_input(&self, outdir: &str) -> Result<String> {
        self.pw_input(outdir, true)
    }

    fn ph_params(&self, init: bool) -> SectionMap {
        let p = &self.config.phonon;
        let mut params = SectionMap::new();
        for (key, value) in [
            ("tr2_ph", json!(p.tr2_ph)),
            ("ldisp", json!(true)),
            ("only_init", json!(init)),
            ("lqdir", json!(true)),
            ("recover", json!(false)),
            ("only_wfc", json!(false)),
            ("fildyn", json!(format!("{}.dyn", self.config.qe.prefix))),
            ("fildvscf", json!("dvscf")),
            ("nq1", json!(p.q_grid[0])),
            ("nq2", json!(p.q_grid[1])),
            ("nq3", json!(p.q_grid[2])),
        ] {
            params.insert(key.into(), value);
        }
        params
    }

    pub fn phonon_init_input(&self, outdir: &str) -> String {
        qf_input::ph::render_inputph(&self.config.qe.prefix, outdir, &self.ph_params(true))
    }

    pub fn phonon_nosplit_input(&self, outdir: &str) -> String {
        qf_input::ph::render_inputph(&self.config.qe.prefix, outdir, &self.ph_params(false))
    }

    pub fn phonon_q_input(
        &self,
        outdir: &str,
        mode: &config::PhMode,
        q: Option<&super::manifest::QPoint>,
        q_index: usize,
    ) -> Result<String> {
        let mut params = self.ph_params(false);
        let mut tail = String::new();
        match mode {
            config::PhMode::Nosplit => (),
            config::PhMode::Index => {
                ensure!(q_index > 0, "data_invalid: q 序号必须从 1 起始");
                params.insert("start_q".into(), json!(q_index));
                params.insert("last_q".into(), json!(q_index));
            }
            config::PhMode::Coords => {
                let q = q.context("data_invalid: coords 模式需要显式 q 坐标")?;
                ensure!(
                    q.cartesian.iter().all(|x| x.is_finite()),
                    "data_invalid: q 坐标非有限"
                );
                params.insert("ldisp".into(), json!(false));
                for key in ["nq1", "nq2", "nq3"] {
                    params.remove(key);
                }
                tail = format!(
                    "{:.17e} {:.17e} {:.17e}\n",
                    q.cartesian[0], q.cartesian[1], q.cartesian[2]
                );
            }
        }
        Ok(qf_input::ph::render_inputph(&self.config.qe.prefix, outdir, &params) + &tail)
    }
}
