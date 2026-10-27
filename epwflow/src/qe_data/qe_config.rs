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
        let first = cell.elements.first().context("data_invalid: 结构没有元素")?;
        let mut pseudo_dir = config.pseudos[first].parent()
            .context("configuration: 赝势缺少父目录")?.to_path_buf();
        for el in &cell.elements {
            let path = &config.pseudos[el];
            ensure!(path.is_absolute(), "configuration: 赝势必须是绝对路径");
            while !path.starts_with(&pseudo_dir) {
                ensure!(pseudo_dir.pop(), "configuration: 赝势缺少共同目录");
            }
        }
        ensure!(pseudo_dir.to_str().context("赝势目录不是 UTF-8")?.len() < 256,
            "configuration: QE pseudo_dir 超过 255 字节");
        for el in &cell.elements {
            let name = config.pseudos[el].strip_prefix(&pseudo_dir)?.to_str()
                .context("赝势路径不是 UTF-8")?;
            ensure!(name.len() <= 80, "configuration: QE 赝势相对文件名超过 80 字节，请使用更近的共同目录");
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
        shared.pp_dir = Some(self.pseudo_dir.to_str().context("赝势目录不是 UTF-8")?.into());
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
                    "data_invalid: 常规电子路径点为空或非有限");
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
            ensure!(columns.len()==3,"configuration: stages.{}: ATOMIC_SPECIES 每行需要三列",task.join("."));
            let element=columns[0];
            ensure!(self.cell.elements.iter().any(|el|el==element)&&seen.insert(element),"configuration: stages.{}.ATOMIC_SPECIES: 重复或未知元素 {element}",task.join("."));
            let mass:f64=columns[1].parse()?;ensure!(mass.is_finite()&&mass>0.,"configuration: stages.{}.ATOMIC_SPECIES: 非法质量",task.join("."));
            let file=columns[2].trim_matches('\'');
            ensure!(self.pseudo_dir.join(file).canonicalize()?==self.config.pseudos[element].canonicalize()?,"input_conflict: stages.{}.ATOMIC_SPECIES 的 {element} 赝势 {file} 与 pseudos.{element}={} 不一致",task.join("."),self.config.pseudos[element].display());
        }
        ensure!(seen.len()==self.cell.elements.len(),"configuration: stages.{}.ATOMIC_SPECIES: 元素集合不完整",task.join("."));
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
        let input=qf_input::ph::render_inputph(&self.config.qe.prefix, outdir, &params) + &tail;
        self.config.apply_native(&[&self.config.phonon_stage(),"q","ph"],&input)
    }
}
