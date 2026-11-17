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
            .context("configuration: Drawing directories do not belong to numbered workspaces")?;
        let cfg = crate::qe_data::direct::current(&root)?;
        let step = Step::ALL
            .into_iter()
            .find(|s| self.dir.starts_with(s.directory(&cfg)) || self.dir.starts_with(super::stage_results::root(&cfg).join(s.directory(&cfg).file_name().unwrap())))
            .context("configuration: The drawing phase is unknown")?;
        let label = format!(
            "Plot · {}",
            super::readable_state::relative(&root, &self.dir)
        );
        Ok((cfg, step, label))
    }
    fn complete(&self) -> Result<bool> {
        let (cfg, step, label) = self.location()?;
        if super::readable_state::completed_files(&cfg, step, &label)?.is_none() {
            return Ok(false);
        }
        if super::readable_state::read(&cfg.work_dir, step)?.get(&label, "Plot version")
            != Some(self.renderer.as_str())
        {
            return Ok(false);
        }
        let Some(old) = super::readable_state::checkpoint_config(&cfg, step, &label)? else {
            return Ok(false);
        };
        if serde_json::to_value(&old.comparison)? != serde_json::to_value(&cfg.comparison)? || old.plot != cfg.plot {
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
        let (cfg,step,label)=self.location()?;
        if let Some(reason)=super::readable_state::read(&cfg.work_dir,step)?.get(&label,"Failure") {
            anyhow::bail!("plot_failed: {reason}; automatic retry is disabled; request plotting explicitly");
        }

        if self.complete()? {
            return Ok(());
        }
        let rendered=fs::create_dir_all(&self.dir).map_err(anyhow::Error::from).and_then(|_|render());
        if let Err(error)=rendered {
            super::readable_state::update(&cfg.work_dir,step,|d| {
                d.set(&label,"Status","failed"); d.set(&label,"Failure",format!("{error:#}")); Ok(())
            })?;
            anyhow::bail!("plot_failed: {error:#}; scientific results retained; automatic retry is disabled");
        }
        let (cfg, step, label) = self.location()?;
        for name in ["electronic-bands.csv","electronic-dos.csv","electronic-bands.json","electronic-dos.json","electronic-pdos.csv","electronic-pdos.json","electronic-display.json"] {
            let path=self.dir.join(name); if path.is_file(){fs::remove_file(path)?;}
        }
        if step==Step::Phonon {
            for name in ["phband","phdos"] {
                let path=self.dir.join(name); if path.is_dir(){fs::remove_dir_all(path)?;}
            }
        }
        super::readable_state::complete_files(&cfg, step, &label, &self.outputs)?;
        super::readable_state::update(&cfg.work_dir, step, |doc| {
            doc.set(&label, "Plot version", &self.renderer);
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
fn electronic_request(cfg: &Config) -> Result<Request> {
    let stage = Step::Electronic.directory(cfg);
    let mut inputs = vec![nscf_xml(cfg),
        stage.join(format!("qe_eband/out/{}.save/data-file-schema.xml",cfg.qe.prefix)),
        stage.join("qe_eband/path-points.json"),
        cfg.kpath.clone().context("required: electronic plots require KPATH")?,
        stage.join(format!("qe_eledos/{}.tdos",cfg.qe.prefix))];
    inputs.extend(super::workspace_pdos::files(cfg)?);
    let files=super::combined_plot::ELECTRONIC_FILES.to_vec();
    Request::new(super::stage_results::root(cfg).join("3.qe-electronic"), inputs, &files,
        serde_json::json!({"renderer_version":super::combined_plot::VERSION,"plot":cfg.plot}))
}

pub fn electronic(cfg: &Config, leaf: &str) -> Result<()> {
    if leaf == "qe_eband" {
        super::band_ranges::generate(cfg, Step::Electronic)?;
        return Ok(());
    }
    let pdos = super::workspace_pdos::run(cfg);
    if pdos.as_ref().err().is_some_and(super::native_jobs::paused) { return pdos; }
    let request = electronic_request(cfg)?;
    let fermi = nscf::parse_xml(&nscf_xml(cfg))?.fermi;
    request.run(|| {
        workspace_plot::qe_bands(cfg, &request.dir, fermi)?;
        workspace_plot::qe_dos(cfg, &request.dir, fermi)?;
        super::workspace_pdos::export(cfg, &request.dir, fermi)?;
        super::combined_plot::electronic(cfg, &request.dir, &request.dir)
    })?;
    let names = request.outputs.iter().map(|p|p.file_name().unwrap().to_string_lossy().into_owned()).collect::<Vec<_>>();
    super::stage_results::publish(cfg,Step::Electronic,"plot",&request.dir,&names,
        serde_json::json!({"kind":"electronic_combined","plot":cfg.plot,"fermi_source":"NSCF",
            "fermi_ev":fermi,"sigma_ev":cfg.native_value::<f64>(&[super::native_config::ELECTRONIC,"qe_eledos","dos","DOS","degauss"])?*13.605693122994}),
        request.sources)?;
    super::stage_results::retire_units(cfg,Step::Electronic,&["qe_eband","qe_eledos"])?;
    for leaf in ["qe_eband","qe_eledos"] {
        super::combined_plot::cleanup(&Step::Electronic.directory(cfg).join(leaf).join("plots"))?;
    }
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
        super::stage_results::root(cfg).join(Step::Phonon.directory(cfg).file_name().unwrap()),
        inputs,
        &[
            "phonon.png",
            "phonon.plot.json",
        ],
        serde_json::json!({"renderer_version":super::combined_plot::VERSION,"sigma":cfg.comparison.clone().unwrap_or_default().phonon_sigma_cm1}),
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
            "data_invalid: Voice DOS Unlimited"
        );
        let rows = qf_io::read_numeric_rows(&text);
        let nat: usize = qe.cell.counts.iter().sum();
        ensure!(
            !rows.is_empty()
                && rows
                    .iter()
                    .all(|r| r.len() == nat + 2 && r.iter().all(|v| v.is_finite())),
            "data_invalid: element phonon DOS requires all {nat} atomic projection columns; TDOS or partial projections cannot replace them"
        );
        ensure!(
            rows.windows(2).all(|r| r[0][0] < r[1][0]),
            "data_invalid: Voice DOS The frequency axis is not increasing"
        );
        for (source, relative) in phonon_inputs(cfg) {
            let target = request.dir.join(relative);
            fs::create_dir_all(target.parent().unwrap())?;
            fs::copy(source, target)?;
        }
        let work = request.dir.to_str().context("Drawing Path Not UTF-8")?;
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
                    .context("CSV Path Not UTF-8")?,
                &rows,
            )?;
        }
        let sigma = cfg.comparison.clone().unwrap_or_default().phonon_sigma_cm1;
        super::combined_plot::qe_phonon(cfg,&request.dir,&qe.cell.elements,&qe.cell.counts,sigma)?;
        Ok(())
    })?;
    let old = Step::Phonon.directory(cfg).join("plots");
    for name in ["phband/phonon_band.png","phdos/phonon_dos.png","phonon_combined.jpg"] {
        let path=old.join(name); if path.is_file() { fs::remove_file(path)?; }
    }
    super::stage_results::retire_units(cfg,Step::Phonon,&["","phband","phdos"])?;
    let qe = QeSettings::from_config(cfg)?;
    super::stage_results::tree(
        cfg,
        Step::Phonon,
        "plot",
        &request.dir,
        serde_json::json!({"sources":request.sources,"kind":"qe_phonon","prefix":cfg.qe.prefix,"elements":qe.cell.elements,"counts":qe.cell.counts,"sigma_cm1":cfg.comparison.clone().unwrap_or_default().phonon_sigma_cm1}),
    )
}

pub fn refresh(cfg: &Config, step: Step) -> Result<()> {
    match step {
        Step::Scf | Step::Nscf => super::band_ranges::generate(cfg, step),
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
        if matches!(step, Step::Scf | Step::Nscf | Step::Electronic)
            && stages::reusable(cfg, step)?
            && super::band_ranges::pending(cfg, step)?
        {
            return Ok(true);
        }
        if !matches!(step, Step::Electronic | Step::Phonon) || !stages::reusable(cfg, step)? {
            continue;
        }
        if step == Step::Electronic && super::workspace_pdos::files(cfg)?.is_empty() {
            return Ok(true);
        }
        let requests = if step == Step::Electronic {
            vec![electronic_request(cfg)?]
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
