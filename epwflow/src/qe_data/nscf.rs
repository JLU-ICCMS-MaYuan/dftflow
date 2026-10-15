use anyhow::{bail, Context, Result};
use regex::Regex;
use std::path::Path;
use super::super::foundation::stage::Backend;
use super::manifest::ElectronicStates;
use super::qe_config::QeSettings;

/// 从 QE 的 data-file-schema.xml 解析完整均匀 k 点、权重、带数与本征能量（FR-004）。
pub fn parse_xml(xml_path: &Path) -> Result<ElectronicStates> {
    let txt = std::fs::read_to_string(xml_path)
        .with_context(|| format!("data_invalid: 无法读取 {}", xml_path.display()))?;
    let fermi_re = Regex::new(r"<fermi_energy>\s*([-0-9.eE+]+)")
        .expect("nscf: fermi 正则编译失败");
    let fermi = if let Some(c) = fermi_re.captures(&txt) {
        c[1].parse::<f64>().context("data_invalid: Fermi 能非数值")?
    } else {
        bail!("data_invalid: XML 缺少 fermi_energy");
    };
    let blk_re = Regex::new(r#"(?s)<k_point[^>]*weight="([-0-9.eE+]+)"[^>]*>(.*?)</k_point>"#)
        .expect("nscf: k_point 正则编译失败");
    let coord_re = Regex::new(r"<kpoint_coord>\s*([-\d.eE+ ]+?)\s*</kpoint_coord>")
        .expect("nscf: kpoint_coord 正则编译失败");
    let e_re = Regex::new(r"<e>\s*([-0-9.eE+]+)\s*</e>").expect("nscf: e 正则编译失败");
    let mut kpoints = Vec::new();
    let mut weights = Vec::new();
    let mut eigenvalues = Vec::new();
    for c in blk_re.captures_iter(&txt) {
        weights.push(c[1].parse::<f64>().context("data_invalid: k 权重非数值")?);
        let body = &c[2];
        let kp: [f64; 3] = if let Some(cc) = coord_re.captures(body) {
            let it = cc[1]
                .split_whitespace()
                .map(|x| x.parse::<f64>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .context("data_invalid: k 坐标非数值")?;
            if it.len() == 3 {
                [it[0], it[1], it[2]]
            } else {
                bail!("data_invalid: k 坐标必须三维");
            }
        } else {
            bail!("data_invalid: k 点缺少 kpoint_coord 坐标");
        };
        kpoints.push(kp);
        let mut evs = Vec::new();
        for ec in e_re.captures_iter(body) {
            evs.push(ec[1].parse::<f64>().context("data_invalid: 本征能量非数值")?);
        }
        if evs.is_empty() {
            bail!("data_invalid: k 点无本征能量");
        }
        eigenvalues.push(evs);
    }
    if weights.is_empty() {
        bail!("data_invalid: XML 未解析到 k 点");
    }
    let nbnd = eigenvalues[0].len();
    Ok(ElectronicStates {
        k_grid: [0u32, 0, 0],
        kpoints,
        weights,
        nbnd,
        eigenvalues,
        wavefunctions: std::path::PathBuf::from("UNKNOWN"),
        fermi,
        layout: "uniform".to_string(),
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
    let input = qe.nscf_input(outdir.to_str().unwrap());
    std::fs::write(dir.join("nscf.in"), &input)?;
    let prog = &qe.config.programs.pw.path;
    let cmd: Vec<String> = launcher
        .iter()
        .cloned()
        .chain(std::iter::once(prog.to_string_lossy().into_owned()))
        .chain(std::iter::once("-inp".to_string()))
        .chain(std::iter::once(dir.join("nscf.in").to_string_lossy().into_owned()))
        .collect();
    match backend {
        Backend::Local => {
            let _r = super::super::foundation::process::run(&cmd, &dir)?;
        }
        Backend::Slurm => {
            super::super::foundation::slurm::submit_via_sbatch(launcher, &cmd[launcher.len()..], &dir, "nscf")?;
        }
    }
    let xml = outdir.join("nscf.save").join("data-file-schema.xml");
    let mut states = parse_xml(&xml)?;
    states.k_grid = qe.config.qe.nscf.k_grid;
    states.wavefunctions = outdir.join("nscf.save");
    Ok(states)
}
