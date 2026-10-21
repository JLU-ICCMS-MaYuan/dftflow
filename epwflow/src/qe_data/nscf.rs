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
    let dir = crate::foundation::layout::path(&work, "nscf");
    std::fs::create_dir_all(&dir)?;
    let outdir = dir.join("out");
    if !crate::foundation::native_jobs::continuing(&dir, "nscf")? {
        super::save::stage_scf(work, &outdir, &qe.config.qe.prefix)?;
        let input = qe.nscf_input(outdir.to_str().unwrap())?;
        std::fs::write(dir.join("nscf.in"), &input)?;
    }
    super::execution::run(
        &qe.config.programs.pw,
        &dir.join("nscf.in"),
        &dir,
        launcher,
        backend,
        "nscf",
        false,
    )?;
    super::save::verify_snapshot(work)?;
    super::xml::validate_save(&outdir, &qe.config.qe.prefix, qe, None)?;
    let xml = outdir
        .join(format!("{}.save", qe.config.qe.prefix))
        .join("data-file-schema.xml");
    let mut states = parse_xml(&xml)?;
    states.k_grid = qe.config.qe.nscf.k_grid;
    validate_grid(
        &states,
        qe.config.qe.nscf.k_grid,
        qe.config.qe.nscf.nbnd as usize,
        &super::xml::Geometry::read(&xml)?,
    )?;
    states.wavefunctions = xml.parent().unwrap().into();
    Ok(states)
}

/// 验证实际完整 Γ 中心网格及等权重，保留 XML 原始笛卡尔坐标约定。
pub fn validate_grid(
    states: &ElectronicStates,
    grid: [u32; 3],
    bands: usize,
    geometry: &super::xml::Geometry,
) -> Result<()> {
    use anyhow::ensure;
    use std::collections::BTreeSet;
    let count = grid.iter().map(|&n| n as usize).product::<usize>();
    ensure!(
        states.nbnd == bands
            && states.kpoints.len() == count
            && states.eigenvalues.len() == count
            && states.weights.len() == count,
        "data_invalid: NSCF 实际带数或完整 k 网格不符"
    );
    let sum: f64 = states.weights.iter().sum();
    ensure!(
        sum.is_finite()
            && sum > 0.0
            && states
                .weights
                .iter()
                .all(|w| (w / sum - 1.0 / count as f64).abs() < 1e-9),
        "data_invalid: NSCF 权重不等或非有限"
    );
    let mut indices = BTreeSet::new();
    for point in &states.kpoints {
        let c = geometry.crystal(*point);
        let mut index = [0u32; 3];
        for a in 0..3 {
            let t = c[a] * grid[a] as f64;
            ensure!(
                (t - t.round()).abs() < 1e-8,
                "data_invalid: NSCF k 点不在固定均匀网格上"
            );
            index[a] = (t.round() as i64).rem_euclid(grid[a] as i64) as u32;
        }
        ensure!(indices.insert(index), "data_invalid: NSCF 存在重复 k 点");
    }
    Ok(())
}
