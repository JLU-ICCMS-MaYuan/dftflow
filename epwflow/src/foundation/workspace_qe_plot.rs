//! QE 单侧图的派生缓存；不将图片纳入科学阶段的复用身份。
use super::{
    config::Config,
    layout,
    stages::{self, Step},
    store::Identity,
    workspace_plot,
};
use crate::qe_data::{nscf, qe_config::QeSettings};
use anyhow::{ensure, Context, Result};
use std::{fs, path::PathBuf};

struct Request {
    dir: PathBuf,
    outputs: Vec<PathBuf>,
    sources: Vec<Identity>,
    renderer: String,
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
        let outputs = files.iter().map(|p| dir.join(p)).collect();
        Ok(Self {
            dir,
            outputs,
            sources: identities,
            renderer: parameters
                .get("renderer_version")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "1".into()),
        })
    }
    fn location(&self) -> Result<(Config, Step, String)> {
        let root = crate::qe_data::direct::workspace(&self.dir)
            .context("configuration: 绘图目录不属于编号工作区")?;
        let cfg = crate::qe_data::direct::current(&root)?;
        let step = Step::ALL
            .into_iter()
            .find(|s| self.dir.starts_with(s.directory(&cfg)))
            .context("configuration: 绘图阶段未知")?;
        let label = format!(
            "绘图 · {}",
            super::readable_state::relative(&root, &self.dir)
        );
        Ok((cfg, step, label))
    }
    fn complete(&self) -> Result<bool> {
        let (cfg, step, label) = self.location()?;
        if super::readable_state::completed_files(&cfg, step, &label)?.is_none() {
            return Ok(false);
        }
        if super::readable_state::read(&cfg.work_dir, step)?.get(&label, "绘图版本")
            != Some(self.renderer.as_str())
        {
            return Ok(false);
        }
        let Some(old) = super::readable_state::checkpoint_config(&cfg, step, &label)? else {
            return Ok(false);
        };
        if serde_json::to_value(&old.comparison)? != serde_json::to_value(&cfg.comparison)? {
            return Ok(false);
        }
        for output in &self.outputs {
            if !output.is_file() {
                return Ok(false);
            }
            let modified = fs::metadata(output)?.modified()?;
            for source in &self.sources {
                if fs::metadata(&source.path)?.modified()? > modified {
                    return Ok(false);
                }
            }
        }
        Ok(true)
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
        let (cfg, step, label) = self.location()?;
        super::readable_state::complete_files(&cfg, step, &label, &self.outputs)?;
        super::readable_state::update(&cfg.work_dir, step, |doc| {
            doc.set(&label, "绘图版本", &self.renderer);
            Ok(())
        })?;
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
            &[
                "electronic-bands.png",
                "electronic-bands.csv",
                "electronic-bands.plot.json",
                "electronic-bands-valence.png",
                "electronic-bands-valence.plot.json",
                "electronic-display.json",
            ]
        }
        "qe_eledos" => {
            inputs.push(dir.join(format!("{}.tdos", cfg.qe.prefix)));
            inputs.extend(super::workspace_pdos::files(cfg)?);
            &[
                "electronic-dos.png",
                "electronic-dos.csv",
                "electronic-dos.plot.json",
                "electronic-dos-valence.png",
                "electronic-dos-valence.plot.json",
                "electronic-display.json",
                "electronic-pdos.csv",
                "electronic-pdos.json",
            ]
        }
        _ => anyhow::bail!("configuration: 未知 QE 绘图任务 {leaf}"),
    };
    Request::new(
        dir.join("plots"),
        inputs,
        files,
        serde_json::json!({"renderer_version":super::electronic_style::RENDERER_VERSION,"points_per_edge":cfg.comparison.clone().unwrap_or_default().points_per_edge}),
    )
}

pub fn electronic(cfg: &Config, leaf: &str) -> Result<()> {
    let pdos = if leaf == "qe_eledos" {
        super::workspace_pdos::run(cfg)
    } else {
        Ok(())
    };
    if pdos.as_ref().err().is_some_and(super::native_jobs::paused) {
        return pdos;
    }
    let request = electronic_request(cfg, leaf)?;
    request.run(|| {
        let fermi = nscf::parse_xml(&nscf_xml(cfg))?.fermi;
        match leaf {
            "qe_eband" => workspace_plot::qe_bands(cfg, &request.dir, fermi),
            "qe_eledos" => {
                workspace_plot::qe_dos(cfg, &request.dir, fermi)?;
                super::workspace_pdos::export(cfg, &request.dir, fermi)
            }
            _ => unreachable!(),
        }
    })?;
    let names = request
        .outputs
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    super::stage_results::publish(
        cfg,
        Step::Electronic,
        leaf,
        &request.dir,
        &names,
        serde_json::json!({"kind":leaf,"fermi_source":"NSCF","fermi_ev":nscf::parse_xml(&nscf_xml(cfg))?.fermi,"sigma_ev":cfg.native_value::<f64>(&[super::native_config::ELECTRONIC,"qe_eledos","dos","DOS","degauss"])?*13.605693122994}),
        request.sources,
    )?;
    pdos
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
    })?;
    let qe = QeSettings::from_config(cfg)?;
    super::stage_results::tree(
        cfg,
        Step::Phonon,
        "",
        &request.dir,
        serde_json::json!({"sources":request.sources,"kind":"qe_phonon","prefix":cfg.qe.prefix,"elements":qe.cell.elements,"counts":qe.cell.counts,"sigma_cm1":cfg.comparison.clone().unwrap_or_default().phonon_sigma_cm1}),
    )
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
    for step in stages::execution_steps(cfg, target) {
        if !matches!(step, Step::Electronic | Step::Phonon) || !stages::reusable(cfg, step)? {
            continue;
        }
        if step == Step::Electronic && super::workspace_pdos::files(cfg)?.is_empty() {
            return Ok(true);
        }
        let requests = if step == Step::Electronic {
            vec![
                electronic_request(cfg, "qe_eband")?,
                electronic_request(cfg, "qe_eledos")?,
            ]
        } else {
            vec![phonon_request(cfg)?]
        };
        if super::stage_results::pending(cfg, step)? {
            return Ok(true);
        }
        for request in requests {
            if !request.complete()? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
