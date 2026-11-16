//! epwflow 严格配置到共享 qeflow 输入库的适配层。
use super::super::foundation::config;
use anyhow::{ensure, Context, Result};
use qf_config::SectionMap;
use qf_crystal::Structure;
use qf_input::pw::PwParams;
use serde_json::json;
use std::path::PathBuf;

/// 结构解析、原子质量与 PW/PH 格式由共享库维护；此处只映射 EPW 参数。
pub struct QeSettings<'a> {
    pub config: &'a config::Config,
    pub cell: Structure,
    pseudo_dir: PathBuf,
}

impl<'a> QeSettings<'a> {
    pub fn from_config(config: &'a config::Config) -> Result<Self> {
        let cell =
            qf_crystal::parse_poscar(config.structure.to_str().context("The structural path is not UTF-8")?)?;
        ensure!(
            cell.elements.len() == cell.counts.len(),
            "data_invalid: The element does not match the quantity"
        );
        for el in &cell.elements {
            ensure!(
                config.pseudos.contains_key(el),
                "configuration: Missing {el} The Visible Shape"
            );
            ensure!(
                qf_input::species::ATOMIC_MASSES
                    .iter()
                    .any(|(e, _)| e == el),
                "configuration: {el} No supported atom mass"
            );
        }
        let first = cell.elements.first().context("data_invalid: Structure without elements")?;
        let mut pseudo_dir = config.pseudos[first].parent()
            .context("configuration: Splendid missing parent directory")?.to_path_buf();
        for el in &cell.elements {
            let path = &config.pseudos[el];
            ensure!(path.is_absolute(), "configuration: It has to be an absolute path");
            while !path.starts_with(&pseudo_dir) {
                ensure!(pseudo_dir.pop(), "configuration: Lack of common catalogues for impulsiveness");
            }
        }
        ensure!(pseudo_dir.to_str().context("No, it's not UTF-8")?.len() < 256,
            "configuration: QE pseudo_dir More than 255 Bytes");
        for el in &cell.elements {
            let name = config.pseudos[el].strip_prefix(&pseudo_dir)?.to_str()
                .context("It's not a power path UTF-8")?;
            ensure!(name.len() <= 80, "configuration: QE Spoil relative file name exceeds 80 bytes, use a more recent common directory");
        }
        Ok(Self { config, cell, pseudo_dir })
    }

    fn pw_input(&self, outdir: &str, nscf: bool, reference: Option<&[[f64; 3]]>) -> Result<String> {
        use crate::foundation::native_config::{SCF,NSCF,ELECTRONIC};
        let task: &[&str] = if reference.is_some() {&[ELECTRONIC,"qe_eband","pw"]} else if nscf {&[NSCF,"pw"]} else {&[SCF,"pw"]};
        let qe = &self.config.qe;
        let mut shared = qf_config::Config::default();
        shared.qe.lattice_mode = Some(qf_config::LATTICE_MODE_CELL_PARAMETERS.into());
        // QE atom_pfile 限制为 80 字节；目录与短相对名共同定位原显式文件。
        shared.pp_dir = Some(self.pseudo_dir.to_str().context("No, it's not UTF-8")?.into());
        let sections: serde_json::Value=self.config.native_value(task)?;
        let mut params=PwParams {
            control: serde_json::from_value(sections["CONTROL"].clone())?,
            system: serde_json::from_value(sections["SYSTEM"].clone())?,
            electrons: serde_json::from_value(sections["ELECTRONS"].clone())?,
        };
        params.control.insert("outdir".into(),json!(outdir));
        let points = if nscf {
            if let Some(points) = reference {
                ensure!(!points.is_empty() && points.iter().flatten().all(|v| v.is_finite()),
                    "data_invalid: General electronic path points are empty or not finite");
                let mut text = format!("K_POINTS crystal\n{}\n", points.len());
                for p in points { text.push_str(&format!("{:.17e} {:.17e} {:.17e} {:.17e}\n",
                    p[0], p[1], p[2], 1.0 / points.len() as f64)); }
                text
            } else { qf_input::pw::uniform_kpoints_text(qe.nscf.k_grid.map(i64::from), true)? }
        } else {
            format!("K_POINTS {}\n",self.config.native_value::<String>(&[SCF,"pw","K_POINTS"])?)
        };
        let mut card_path=task.to_vec();card_path.push("ATOMIC_SPECIES");
        let native_species:String=self.config.native_value(&card_path)?;
        let mut seen=std::collections::BTreeSet::new();
        for row in native_species.lines().filter(|s|!s.trim().is_empty()) {
            let columns:Vec<_>=row.split_whitespace().collect();
            ensure!(columns.len()==3,"configuration: stages.{}: ATOMIC_SPECIES Three rows per row",task.join("."));
            let element=columns[0];
            ensure!(self.cell.elements.iter().any(|el|el==element)&&seen.insert(element),"configuration: stages.{}.ATOMIC_SPECIES: Repeat or Unknown Element {element}",task.join("."));
            let mass:f64=columns[1].parse()?;ensure!(mass.is_finite()&&mass>0.,"configuration: stages.{}.ATOMIC_SPECIES: Illegal quality",task.join("."));
            let file=columns[2].trim_matches('\'');
            ensure!(self.pseudo_dir.join(file).canonicalize()?==self.config.pseudos[element].canonicalize()?,"input_conflict: stages.{}.ATOMIC_SPECIES It's {element} Powerful {file} with pseudos.{element}={} Inconsistencies",task.join("."),self.config.pseudos[element].display());
        }
        ensure!(seen.len()==self.cell.elements.len(),"configuration: stages.{}.ATOMIC_SPECIES: The collection of elements is incomplete",task.join("."));
        let species=format!("ATOMIC_SPECIES\n{}\n",native_species);
        let input=qf_input::pw::write_pw_input_with_species(
            &shared, &self.cell, &params, &qe.prefix, &points, 17, &species,
        )?;
        self.config.apply_native(task,&input)
    }

    pub fn scf_input(&self, outdir: &str) -> Result<String> {
        self.pw_input(outdir, false, None)
    }
    pub fn nscf_input(&self, outdir: &str) -> Result<String> {
        self.pw_input(outdir, true, None)
    }

    /// 常规电子能带路径；完整均匀 NSCF 保存树由 nscf_input 单独生成。
    pub fn path_input(&self, outdir: &str, points: &[[f64; 3]]) -> Result<String> {
        Ok(self.pw_input(outdir, true, Some(points))?.replace("'nscf'", "'bands'"))
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
            (
                "fildyn",
                json!(format!("{}.dyn.xml", self.config.qe.prefix)),
            ),
            ("fildvscf", json!("dvscf")),
            ("nq1", json!(p.q_grid[0])),
            ("nq2", json!(p.q_grid[1])),
            ("nq3", json!(p.q_grid[2])),
        ] {
            params.insert(key.into(), value);
        }
        params
    }

    pub fn phonon_init_input(&self, outdir: &str) -> Result<String> {
        self.config.apply_native(&[&self.config.phonon_stage(),"ph"],&qf_input::ph::render_inputph(&self.config.qe.prefix, outdir, &self.ph_params(true)))
    }

    pub fn phonon_nosplit_input(&self, outdir: &str) -> Result<String> {
        self.config.apply_native(&[&self.config.phonon_stage(),"ph"],&qf_input::ph::render_inputph(&self.config.qe.prefix, outdir, &self.ph_params(false)))
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
                ensure!(q_index > 0, "data_invalid: q Serial number has to be from 1 Start");
                params.insert("start_q".into(), json!(q_index));
                params.insert("last_q".into(), json!(q_index));
            }
            config::PhMode::Coords => {
                let q = q.context("data_invalid: coords Mode requires visible q Coordinates")?;
                ensure!(
                    q.cartesian.iter().all(|x| x.is_finite()),
                    "data_invalid: q Coordinates are not finite"
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
        let input=qf_input::ph::render_inputph(&self.config.qe.prefix, outdir, &params) + &tail;
        self.config.apply_native(&[&self.config.phonon_stage(),"q","ph"],&input)
    }
}
