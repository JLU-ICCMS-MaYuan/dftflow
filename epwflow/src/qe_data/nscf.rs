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
        "data_invalid: XML 声明 nks={}，实际 ks_energies={}，k 点数不完整",
        nks,
        kpoints.len()
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
    let states = verified_states(&qe.config, work)?;
    super::xml::validate_save(&outdir, &qe.config.qe.prefix, qe, None)?;
    Ok(states)
}

/// 新执行和编号工作区复用共用实际输入、不可变 SCF 来源及输出核验。
pub fn verified_states(
    cfg: &crate::foundation::config::Config,
    work: &Path,
) -> Result<ElectronicStates> {
    let dir = crate::foundation::layout::path(work, "nscf");
    let relative = format!("{}.save/data-file-schema.xml", cfg.qe.prefix);
    verify_files(
        &dir.join("nscf.in"),
        &crate::foundation::layout::path(work, "scf/snapshot").join(&relative),
        &dir.join("out").join(&relative),
        cfg.qe.nscf.k_grid,
        cfg.qe.nscf.nbnd as usize,
    )
}

/// 核验实际输入而非仅使用配置乘积；保留原始 XML，出错时不自动换模式或重试。
pub fn verify_files(
    input: &Path,
    scf_xml: &Path,
    xml: &Path,
    grid: [u32; 3],
    bands: usize,
) -> Result<ElectronicStates> {
    let result = (|| -> Result<ElectronicStates> {
        let raw = std::fs::read_to_string(input)?;
        let points = input_points(&raw)?;
        let expected = input_points(&qf_input::pw::uniform_kpoints_text(grid.map(i64::from), true)?)?;
        anyhow::ensure!(points == expected,
            "data_invalid: NSCF 实际输入点表与配置网格 {grid:?} 的 kmesh.pl 结果不一致；输入行数={}，要求={}", points.len(), expected.len());
        let mut states = parse_xml(xml).with_context(|| format!("NSCF 输入点数={}", points.len()))?;
        anyhow::ensure!(points.len() == states.kpoints.len(),
            "data_invalid: NSCF 输入点数={}，输出 nks/ks_energies={}，数量不一致", points.len(), states.kpoints.len());
        let geometry = super::xml::Geometry::read(xml)?;
        states.k_grid = grid;
        validate_grid(&states, grid, bands, &geometry)?;
        for (i, (p, out)) in points.iter().zip(&states.kpoints).enumerate() {
            let c = geometry.crystal(*out);
            anyhow::ensure!((0..3).all(|a| {
                let delta = c[a] - p[a];
                delta.is_finite() && (delta - delta.round()).abs() <= 1e-8
            }), "data_invalid: NSCF 第 {} 个点的输出坐标／顺序与实际输入不一致", i + 1);
        }
        Ok(states)
    })().with_context(|| format!(
        "NSCF 输入/输出核验失败：输入 {}；输出 {}。请检查数量、点序及权重；若 pw.x 在 nscf 模式添加或改变 k 点，建议显式设置 stages.\"2.qe-nscf\".calculation='bands' 后按恢复流程重算 NSCF，仍调用 pw.x；bands.x 是后处理程序",
        input.display(), xml.display()))?;
    super::xml::verify_fft(scf_xml, xml)?;
    Ok(result)
}

fn input_points(raw: &str) -> Result<Vec<[f64; 4]>> {
    use anyhow::ensure;
    let lines: Vec<_> = raw
        .lines()
        .map(|l| l.split('!').next().unwrap().trim())
        .filter(|l| !l.is_empty())
        .collect();
    let cards: Vec<_> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.to_ascii_uppercase().starts_with("K_POINTS"))
        .collect();
    ensure!(
        cards.len() == 1,
        "data_invalid: NSCF 必须只有一份 K_POINTS 卡片"
    );
    let (start, header) = cards[0];
    ensure!(
        header.eq_ignore_ascii_case("K_POINTS crystal"),
        "data_invalid: NSCF 需要 K_POINTS crystal"
    );
    let count: usize = lines
        .get(start + 1)
        .context("data_invalid: NSCF 缺输入声明点数")?
        .parse()?;
    let points = lines[start + 2..]
        .iter()
        .map(|line| -> Result<[f64; 4]> {
            super::xml::numbers(line)?
                .try_into()
                .map_err(|_| anyhow::anyhow!("data_invalid: NSCF 输入每点需要三维坐标及权重"))
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        count > 0 && points.len() == count && points.iter().all(|p| p[3] > 0.),
        "data_invalid: NSCF 输入声明点数={count}，实际坐标行数={}；权重须为正",
        points.len()
    );
    Ok(points)
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
    let count = qf_input::pw::uniform_kpoint_count(grid.map(i64::from))?;
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
                .all(|w| w.is_finite() && *w > 0. && (w / sum * count as f64 - 1.0).abs() < 1e-6),
        "data_invalid: NSCF 权重不等或非有限"
    );
    let mut indices = BTreeSet::new();
    for point in &states.kpoints {
        let c = geometry.crystal(*point);
        let mut index = [0u32; 3];
        for a in 0..3 {
            let t = c[a] * grid[a] as f64;
            ensure!(
                t.is_finite()
                    && (t - t.round()).abs() / (grid[a] as f64)
                        <= 1e-8_f64.min(0.25 / grid[a] as f64),
                "data_invalid: NSCF k 点不在固定均匀网格上"
            );
            index[a] = (t.round() as i64).rem_euclid(grid[a] as i64) as u32;
        }
        ensure!(indices.insert(index), "data_invalid: NSCF 存在重复 k 点");
    }
    Ok(())
}
