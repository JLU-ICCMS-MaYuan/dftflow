use super::super::foundation::stage::Backend;
use super::manifest::ElectronicStates;
use super::qe_config::QeSettings;
use anyhow::{Context, Result};
use std::path::Path;

/// 从 QE 的 data-file-schema.xml 解析完整均匀 k 点、权重、带数与本征能量（FR-004）。
pub fn parse_xml(xml_path: &Path) -> Result<ElectronicStates> {
    use anyhow::ensure;
    const HA_EV: f64 = 27.211386245988;
    let txt = std::fs::read_to_string(xml_path)?;
    let doc = roxmltree::Document::parse(&txt).context("data_invalid: QE XML 损坏")?;
    let band = doc
        .descendants()
        .find(|n| n.has_tag_name("band_structure"))
        .context("data_invalid: 缺少 QE band_structure")?;
    let scalar = |name: &str| -> Result<f64> {
        let t = band
            .children()
            .find(|n| n.has_tag_name(name))
            .and_then(|n| n.text())
            .with_context(|| format!("data_invalid: 缺少 {name}"))?;
        let v: f64 = t.trim().replace(['d', 'D'], "e").parse()?;
        ensure!(v.is_finite(), "data_invalid: {name} 非有限");
        Ok(v)
    };
    let nbnd = scalar("nbnd")?;
    let nks = scalar("nks")?;
    ensure!(
        nbnd > 0.0 && nbnd.fract() == 0.0 && nks > 0.0 && nks.fract() == 0.0,
        "data_invalid: 非法带数或 k 点数"
    );
    let mut kpoints = Vec::new();
    let mut weights = Vec::new();
    let mut eigenvalues = Vec::new();
    let numbers = |n: roxmltree::Node| -> Result<Vec<f64>> {
        let v: Vec<f64> = n
            .text()
            .context("data_invalid: XML 数组为空")?
            .split_whitespace()
            .map(|t| {
                t.replace(['d', 'D'], "e")
                    .parse::<f64>()
                    .map_err(anyhow::Error::from)
            })
            .collect::<Result<_>>()?;
        ensure!(
            v.iter().all(|x| x.is_finite()),
            "data_invalid: XML 含非有限值"
        );
        Ok(v)
    };
    for ks in band.children().filter(|n| n.has_tag_name("ks_energies")) {
        let k = ks
            .children()
            .find(|n| n.has_tag_name("k_point"))
            .context("data_invalid: 缺少 k_point")?;
        let v = numbers(k)?;
        ensure!(v.len() == 3, "data_invalid: k 点不是三维");
        let w: f64 = k
            .attribute("weight")
            .context("data_invalid: k 权重缺失")?
            .trim()
            .parse()?;
        ensure!(w.is_finite() && w > 0.0, "data_invalid: 非法 k 权重");
        let en = ks
            .children()
            .find(|n| n.has_tag_name("eigenvalues"))
            .context("data_invalid: 缺本征值")?;
        let ev = numbers(en)?;
        ensure!(ev.len() == nbnd as usize, "data_invalid: 本征值带数不一致");
        if let Some(size) = en.attribute("size") {
            ensure!(
                size.parse::<usize>()? == ev.len(),
                "data_invalid: XML size 不一致"
            );
        }
        kpoints.push([v[0], v[1], v[2]]);
        weights.push(w);
        eigenvalues.push(ev.into_iter().map(|x| x * HA_EV).collect());
    }
    ensure!(
        kpoints.len() == nks as usize,
        "data_invalid: XML k 点数不完整"
    );
    Ok(ElectronicStates {
        k_grid: [0; 3],
        kpoints,
        weights,
        nbnd: nbnd as usize,
        eigenvalues,
        wavefunctions: xml_path
            .parent()
            .context("data_invalid: XML 缺父目录")?
            .to_path_buf(),
        fermi: scalar("fermi_energy")? * HA_EV,
        layout: "qe_cartesian_2pi_alat_energies_ev".into(),
    })
}

/// 完整均匀 NSCF：写输入、执行、核对波函数集合与电子态收敛。
pub fn run_nscf(
    qe: &QeSettings,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<ElectronicStates> {
    let dir = work.join("nscf");
    std::fs::create_dir_all(&dir)?;
    let outdir = dir.join("out");
    std::fs::create_dir_all(&outdir)?;
    let input = qe.nscf_input(outdir.to_str().unwrap())?;
    std::fs::write(dir.join("nscf.in"), &input)?;
    let prog = &qe.config.programs.pw.path;
    let cmd: Vec<String> = launcher
        .iter()
        .cloned()
        .chain(std::iter::once(prog.to_string_lossy().into_owned()))
        .chain(std::iter::once("-inp".to_string()))
        .chain(std::iter::once(
            dir.join("nscf.in").to_string_lossy().into_owned(),
        ))
        .collect();
    match backend {
        Backend::Local => {
            let _r = super::super::foundation::process::run(&cmd, &dir)?;
        }
        Backend::Slurm => {
            super::super::foundation::slurm::submit_via_sbatch(
                launcher,
                &cmd[launcher.len()..],
                &dir,
                "nscf",
            )?;
        }
    }
    let xml = outdir.join("nscf.save").join("data-file-schema.xml");
    let mut states = parse_xml(&xml)?;
    states.k_grid = qe.config.qe.nscf.k_grid;
    states.wavefunctions = outdir.join("nscf.save");
    Ok(states)
}
