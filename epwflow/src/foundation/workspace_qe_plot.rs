//! QE 单侧图的派生缓存；不将图片纳入科学阶段的复用身份。
use super::{
    config::Config,
    layout,
    stages::{self, Step},
    store::{self, Identity},
    workspace_plot,
};
use crate::qe_data::{nscf, qe_config::QeSettings};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Serialize, Deserialize)]
struct Receipt {
    key: String,
    outputs: Vec<Identity>,
}

struct Request {
    dir: PathBuf,
    key: String,
    outputs: Vec<PathBuf>,
}
impl Request {
    fn new(
        dir: PathBuf,
        inputs: Vec<PathBuf>,
        files: &[&str],
        parameters: serde_json::Value,
    ) -> Result<Self> {
        let identities = inputs
            .iter()
            .map(|p| Identity::capture(p))
            .collect::<Result<Vec<_>>>()?;
        let key = store::hash_bytes(&serde_json::to_vec(&(1, identities, parameters))?);
        let outputs = files.iter().map(|p| dir.join(p)).collect();
        Ok(Self { dir, key, outputs })
    }
    fn complete(&self) -> Result<bool> {
        let file = self.dir.join("completion.json");
        if !file.is_file() {
            return Ok(false);
        }
        let r: Receipt = store::read(&file)?;
        if r.key != self.key
            || r.outputs.iter().map(|i| &i.path).collect::<Vec<_>>()
                != self.outputs.iter().collect::<Vec<_>>()
        {
            return Ok(false);
        }
        let mut available = true;
        for id in r.outputs {
            if !store::identity_available(&id)? {
                available = false;
            }
        }
        Ok(available)
    }
    fn run(&self, render: impl FnOnce() -> Result<()>) -> Result<()> {
        if self.complete()? {
            return Ok(());
        }
        fs::create_dir_all(&self.dir)?;
        render().with_context(|| {
            format!(
                "plot_failed: QE 图像未完成 {}；科学结果已保留，可恢复补图",
                self.dir.display()
            )
        })?;
        let outputs = self
            .outputs
            .iter()
            .map(|p| Identity::capture(p))
            .collect::<Result<Vec<_>>>()?;
        store::write(
            &self.dir.join("completion.json"),
            &Receipt {
                key: self.key.clone(),
                outputs,
            },
        )?;
        println!("QE plots: {}", self.dir.display());
        Ok(())
    }
}

fn nscf_xml(cfg: &Config) -> PathBuf {
    layout::path(&cfg.work_dir, "nscf/out")
        .join(format!("{}.save/data-file-schema.xml", cfg.qe.prefix))
}
fn electronic_request(cfg: &Config, leaf: &str) -> Result<Request> {
    let dir = Step::Electronic.directory(cfg).join(leaf);
    let mut inputs = vec![nscf_xml(cfg)];
    let files: &[&str] = match leaf {
        "qe_eband" => {
            inputs.extend([
                dir.join(format!("out/{}.save/data-file-schema.xml", cfg.qe.prefix)),
                dir.join("path-points.json"),
                cfg.kpath.clone().context("required: 电子绘图需要 KPATH")?,
            ]);
            &["electronic-bands.png", "electronic-bands.csv"]
        }
        "qe_eledos" => {
            inputs.push(dir.join(format!("{}.tdos", cfg.qe.prefix)));
            &["electronic-dos.png", "electronic-dos.csv"]
        }
        _ => anyhow::bail!("configuration: 未知 QE 绘图任务 {leaf}"),
    };
    Request::new(
        dir.join("plots"),
        inputs,
        files,
        serde_json::json!(cfg.comparison.clone().unwrap_or_default().points_per_edge),
    )
}

pub fn electronic(cfg: &Config, leaf: &str) -> Result<()> {
    let request = electronic_request(cfg, leaf)?;
    request.run(|| {
        let fermi = nscf::parse_xml(&nscf_xml(cfg))?.fermi;
        match leaf {
            "qe_eband" => workspace_plot::qe_bands(cfg, &request.dir, fermi),
            "qe_eledos" => workspace_plot::qe_dos(cfg, &request.dir, fermi),
            _ => unreachable!(),
        }
    })
}

fn phonon_inputs(cfg: &Config) -> Vec<(PathBuf, PathBuf)> {
    let dir = Step::Phonon.directory(cfg);
    [
        (format!("{}.phband.freq", cfg.qe.prefix), "phband"),
        (format!("{}_band.labelinfo.dat", cfg.qe.prefix), "phband"),
        (format!("{}.phdos.dat", cfg.qe.prefix), "phdos"),
    ]
    .into_iter()
    .map(|(name, sub)| (dir.join(&name), PathBuf::from(sub).join(name)))
    .collect()
}
fn phonon_request(cfg: &Config) -> Result<Request> {
    let mut inputs: Vec<_> = phonon_inputs(cfg).into_iter().map(|(p, _)| p).collect();
    inputs.push(cfg.structure.clone());
    Request::new(
        Step::Phonon.directory(cfg).join("plots"),
        inputs,
        &[
            "phband/phonon_band.png",
            "phband/phonon_band.csv",
            "phband/phonon_band_labels.csv",
            "phdos/phonon_dos.png",
            "phdos/phonon_dos_atoms.csv",
            "phdos/phonon_dos_elements.csv",
            "phonon_combined.jpg",
        ],
        serde_json::json!(cfg.comparison.clone().unwrap_or_default().phonon_sigma_cm1),
    )
}

pub fn phonon(cfg: &Config) -> Result<()> {
    let request = phonon_request(cfg)?;
    request.run(|| {
        let qe = QeSettings::from_config(cfg)?;
        let text = fs::read_to_string(
            Step::Phonon
                .directory(cfg)
                .join(format!("{}.phdos.dat", cfg.qe.prefix)),
        )?;
        ensure!(
            !regex::Regex::new(r"(?i)\b(?:nan|inf|infinity)\b")?.is_match(&text),
            "data_invalid: 声子 DOS 非有限"
        );
        let rows = qf_io::read_numeric_rows(&text);
        let nat: usize = qe.cell.counts.iter().sum();
        ensure!(
            !rows.is_empty()
                && rows
                    .iter()
                    .all(|r| r.len() == nat + 2 && r.iter().all(|v| v.is_finite())),
            "data_invalid: 元素声子 DOS 需要完整的 {nat} 个原子投影列，不能用 TDOS 或部分投影代替"
        );
        ensure!(
            rows.windows(2).all(|r| r[0][0] < r[1][0]),
            "data_invalid: 声子 DOS 频率轴不递增"
        );
        for (source, relative) in phonon_inputs(cfg) {
            let target = request.dir.join(relative);
            fs::create_dir_all(target.parent().unwrap())?;
            fs::copy(source, target)?;
        }
        let work = request.dir.to_str().context("绘图路径非 UTF-8")?;
        let (band, labels) = qf_phonon::build_phonon_band_tables(
            work,
            &cfg.qe.prefix,
            &qe.cell.elements,
            &qe.cell.counts,
        )?;
        let (atoms, elements, _) = qf_phonon::build_phonon_dos_tables(
            work,
            &cfg.qe.prefix,
            &qe.cell.elements,
            &qe.cell.counts,
        )?;
        for (relative, rows) in [
            ("phband/phonon_band.csv", band),
            ("phband/phonon_band_labels.csv", labels),
            ("phdos/phonon_dos_atoms.csv", atoms),
            ("phdos/phonon_dos_elements.csv", elements),
        ] {
            qf_phonon::write_csv(
                request
                    .dir
                    .join(relative)
                    .to_str()
                    .context("CSV 路径非 UTF-8")?,
                &rows,
            )?;
        }
        let sigma = cfg.comparison.clone().unwrap_or_default().phonon_sigma_cm1;
        qf_phonon::plot::plot_phonon_band(work, &cfg.qe.prefix)?;
        qf_phonon::plot::plot_phonon_dos(
            work,
            &cfg.qe.prefix,
            &qe.cell.elements,
            &qe.cell.counts,
            sigma,
        )?;
        qf_phonon::plot::plot_phonon_combined(
            work,
            &cfg.qe.prefix,
            &qe.cell.elements,
            &qe.cell.counts,
            sigma,
        )?;
        Ok(())
    })
}

pub fn refresh(cfg: &Config, step: Step) -> Result<()> {
    match step {
        Step::Electronic => {
            electronic(cfg, "qe_eband")?;
            electronic(cfg, "qe_eledos")
        }
        Step::Phonon => phonon(cfg),
        _ => Ok(()),
    }
}

pub fn pending(cfg: &Config, target: Step) -> Result<bool> {
    for step in target.closure() {
        if !matches!(step, Step::Electronic | Step::Phonon) || !stages::reusable(cfg, step)? {
            continue;
        }
        let requests = if step == Step::Electronic {
            vec![
                electronic_request(cfg, "qe_eband")?,
                electronic_request(cfg, "qe_eledos")?,
            ]
        } else {
            vec![phonon_request(cfg)?]
        };
        for request in requests {
            if !request.complete()? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
