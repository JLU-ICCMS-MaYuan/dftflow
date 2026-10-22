//! 原生诊断数据适配。仅解析已有结果及累积 DOS，不启动 QE/EPW。
use super::{differences::MEV_CM1, native};
use crate::{
    foundation::store,
    interpolation::comparison_data::{
        Artifact, Manifest, Metadata, Tick, TABLES,
    },
    qe_data::nscf,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    pub path: PathBuf,
    pub sha256: String,
}

impl SourceFile {
    pub(crate) fn checked_path(&self, base: &Path) -> Result<PathBuf> {
        let path = base.join(&self.path);
        ensure!(
            self.sha256.len() == 64 && store::hash_file(&path)? == self.sha256,
            "data_invalid: 比较来源指纹不符 {}",
            path.display()
        );
        Ok(path)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathPoint {
    pub segment_id: usize,
    pub point_id: usize,
    pub distance: f64,
    pub crystal: [f64; 3],
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sources {
    pub schema_version: u32,
    pub run_id: String,
    pub ph_mode: String,
    pub fermi_reference: SourceFile,
    pub qe_bands_xml: SourceFile,
    pub qe_mesh_xml: SourceFile,
    pub epw_bands: SourceFile,
    pub qe_phonon_bands: SourceFile,
    pub epw_phonon_bands: SourceFile,
    pub epw_log: SourceFile,
    pub provenance: Vec<SourceFile>,
    pub path: Vec<PathPoint>,
    pub ticks: Vec<Tick>,
    pub mesh: Vec<[f64; 3]>,
    pub mesh_shape: [usize; 3],
    /// 晶体坐标右乘倒格矢矩阵，得到 QE 2pi/alat 坐标。
    pub reciprocal: [[f64; 3]; 3],
    pub window_ev: [f64; 2],
    /// 实际 Wannier 日志的外窗；给出时替代旧的诊断显示窗。
    #[serde(default)]
    pub wannier_wout: Option<SourceFile>,
    pub electronic_sigma_ev: f64,
    pub phonon_sigma_cm1: f64,
}

fn parse_window(text: &str, name: &str) -> Result<Option<[f64; 2]>> {
    let label = format!("{name}:");
    let records: Vec<_> = text
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix('|'))
        .filter_map(|line| line.trim_start().strip_prefix(&label))
        .collect();
    if records.is_empty() {
        return Ok(None);
    }
    ensure!(
        records.len() == 1,
        "data_invalid: Wannier 日志必须有唯一 {name} 能窗"
    );
    let pattern = regex::Regex::new(r"^\s*(\S+)\s+to\s+(\S+)\s+\(eV\)")?;
    let record = pattern
        .captures(records[0])
        .with_context(|| format!("data_invalid: Wannier {name} 能窗格式无效"))?;
    let value = |i: usize| -> Result<f64> { Ok(record[i].replace(['d', 'D'], "e").parse()?) };
    let window = [value(1)?, value(2)?];
    ensure!(
        window.iter().all(|v| v.is_finite()) && window[0] < window[1],
        "data_invalid: Wannier {name} 能窗必须有限且递增"
    );
    Ok(Some(window))
}

/// 读取 Wannier 实际采用的绝对能量外窗，也覆盖未显式设置 dis_win 的情况。
pub fn parse_outer_window(text: &str) -> Result<[f64; 2]> {
    parse_window(text, "Outer")?.context("data_invalid: Wannier 日志缺少 Outer 能窗")
}

/// 未记录 Inner 时返回缺失，不能用外窗或显示窗代替冻结窗。
pub fn parse_frozen_window(text: &str) -> Result<Option<[f64; 2]>> {
    parse_window(text, "Inner")
}

/// 相同高斯约定和物理权重用于两侧；不按输出曲线重新缩放。
pub fn gaussian_dos(
    axis: &[f64],
    states: &[Vec<f64>],
    weights: &[f64],
    sigma: f64,
) -> Result<Vec<f64>> {
    ensure!(
        sigma.is_finite()
            && sigma > 0.0
            && states.len() == weights.len()
            && !states.is_empty()
            && axis.iter().all(|v| v.is_finite())
            && axis.windows(2).all(|w| w[0] < w[1])
            && states.iter().flatten().all(|v| v.is_finite())
            && weights.iter().all(|v| v.is_finite() && *v > 0.0),
        "data_invalid: DOS 网格、状态、权重或核宽度无效"
    );
    let norm = sigma * std::f64::consts::PI.sqrt();
    Ok(axis
        .iter()
        .map(|x| {
            states
                .iter()
                .zip(weights)
                .map(|(es, w)| {
                    w * es
                        .iter()
                        .map(|e| (-((x - e) / sigma).powi(2)).exp() / norm)
                        .sum::<f64>()
                })
                .sum()
        })
        .collect())
}

fn cart(p: [f64; 3], reciprocal: &[[f64; 3]; 3]) -> [f64; 3] {
    std::array::from_fn(|i| (0..3).map(|j| p[j] * reciprocal[j][i]).sum())
}
fn same(a: &[f64; 3], b: &[f64; 3]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-6)
}

fn axis(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| lo + (hi - lo) * i as f64 / (n - 1) as f64)
        .collect()
}
fn integral(x: &[f64], y: &[f64]) -> f64 {
    x.windows(2)
        .zip(y.windows(2))
        .map(|(x, y)| (x[1] - x[0]) * (y[1] + y[0]) * 0.5)
        .sum()
}

pub fn export(source: &Path, output: &Path) -> Result<()> {
    let mut s: Sources = store::read(source)?;
    ensure!(
        s.schema_version == 2 && matches!(s.ph_mode.as_str(), "nosplit" | "index" | "coords"),
        "data_invalid: 来源版本或 PH 模式无效"
    );
    let base = source.parent().context("data_invalid: 来源目录缺失")?;
    ensure!(!output.exists(), "input_conflict: 比较输出已存在");
    ensure!(
        s.path.len() >= 2
            && s.path
                .iter()
                .all(|p| p.crystal.iter().all(|v| v.is_finite()) && p.distance.is_finite())
            && s.path.windows(2).all(|p| p[0].distance < p[1].distance)
            && s.reciprocal.iter().flatten().all(|v| v.is_finite()),
        "data_invalid: 路径不完整或距离不递增"
    );
    let fermi = nscf::parse_xml(&s.fermi_reference.checked_path(base)?)?.fermi;
    let wannier_log = s
        .wannier_wout
        .as_ref()
        .map(|file| fs::read_to_string(file.checked_path(base)?).map_err(anyhow::Error::from))
        .transpose()?;
    let disentanglement_window_ev = wannier_log.as_deref().map(parse_outer_window).transpose()?;
    let frozen_window_ev = wannier_log
        .as_deref()
        .map(parse_frozen_window)
        .transpose()?
        .flatten();
    if let Some(window) = disentanglement_window_ev {
        s.window_ev = window.map(|e| e - fermi);
    }
    let qe = nscf::parse_xml(&s.qe_bands_xml.checked_path(base)?)?;
    let mesh = nscf::parse_xml(&s.qe_mesh_xml.checked_path(base)?)?;
    let epw = native::parse_bands(&fs::read_to_string(s.epw_bands.checked_path(base)?)?)?;
    let qph = native::parse_matdyn_path(&fs::read_to_string(s.qe_phonon_bands.checked_path(base)?)?)
        .context("data_invalid: QE matdyn 频率文件")?;
    let eph = native::parse_bands(&fs::read_to_string(s.epw_phonon_bands.checked_path(base)?)?)?;
    native::checked_log(&fs::read_to_string(s.epw_log.checked_path(base)?)?, 0)?;
    for p in &s.provenance {
        p.checked_path(base)?;
    }
    let n = s.path.len();
    let total = n + s.mesh.len();
    ensure!(
        qe.kpoints.len() == n
            && epw.values.len() == total
            && qph.values.len() == total
            && eph.values.len() == total
            && mesh.kpoints.len() == s.mesh.len(),
        "data_invalid: 原生路径／网格点数不完整"
    );
    let mut mesh_set = std::collections::BTreeSet::new();
    ensure!(
        s.mesh_shape.iter().all(|n| *n > 0)
            && s.mesh_shape
                .iter()
                .try_fold(1usize, |a, b| a.checked_mul(*b))
                == Some(s.mesh.len()),
        "data_invalid: DOS 必须来自完整均匀网格"
    );
    for p in &s.mesh {
        let mut key = [0usize; 3];
        for i in 0..3 {
            let x = p[i] * s.mesh_shape[i] as f64;
            ensure!(
                x.is_finite() && (x - x.round()).abs() < 1e-7 && p[i] >= 0.0 && p[i] < 1.0,
                "data_invalid: DOS 网格并非固定 Gamma 均匀网格"
            );
            key[i] = x.round() as usize;
        }
        ensure!(mesh_set.insert(key), "data_invalid: 重复 DOS 网格点");
    }
    ensure!(
        mesh.weights
            .iter()
            .all(|w| (*w - mesh.weights[0]).abs() < 1e-8),
        "data_invalid: 完整网格权重不均匀"
    );
    for (i, p) in s
        .path
        .iter()
        .map(|p| p.crystal)
        .chain(s.mesh.iter().copied())
        .enumerate()
    {
        let expected = cart(p, &s.reciprocal);
        ensure!(
            same(&expected, &epw.points_cartesian[i])
                && same(&expected, &qph.points_cartesian[i])
                && same(&expected, &eph.points_cartesian[i]),
            "data_invalid: QE／EPW 路径或网格坐标不符，点 {i}"
        );
        if i < n {
            ensure!(
                same(&expected, &qe.kpoints[i]),
                "data_invalid: QE 电子路径坐标不符"
            );
        }
    }
    let modes = eph.values[0].len();
    ensure!(
        modes > 0
            && eph
                .values
                .iter()
                .chain(&qph.values)
                .all(|v| v.len() == modes),
        "data_invalid: 声子模式不完整"
    );
    // 总量比较按实际原子顺序验证 3N；缺少算符时不推断元素／轨道投影。
    let qe_text = fs::read_to_string(s.qe_mesh_xml.checked_path(base)?)?;
    let doc = roxmltree::Document::parse(&qe_text)?;
    let output_structure = doc.descendants().find(|n| n.has_tag_name("output"))
        .context("data_invalid: 缺少 QE output 结构")?;
    let atoms: Vec<_> = output_structure
        .descendants()
        .find(|n| n.has_tag_name("atomic_positions"))
        .context("data_invalid: 缺少原子顺序")?
        .children()
        .filter(|n| n.has_tag_name("atom"))
        .map(|n| n.attribute("name").context("data_invalid: 原子缺少元素名"))
        .collect::<Result<_>>()?;
    ensure!(
        !atoms.is_empty() && modes == 3 * atoms.len(),
        "data_invalid: 声子模式数与原子数不同"
    );
    let path_text = fs::read_to_string(s.qe_bands_xml.checked_path(base)?)?;
    let path_doc = roxmltree::Document::parse(&path_text)?;
    let positions = |root: roxmltree::Node<'_, '_>| -> Result<Vec<(String, [f64; 3])>> {
        root.descendants().find(|n| n.has_tag_name("output"))
            .context("data_invalid: 缺少 QE output")?
            .descendants().find(|n| n.has_tag_name("atomic_positions"))
            .context("data_invalid: 缺少原子位置")?.children().filter(|n| n.has_tag_name("atom"))
            .map(|n| Ok((n.attribute("name").context("data_invalid: 缺少元素名")?.into(),
                crate::qe_data::xml::vector(n.text().context("data_invalid: 缺少原子坐标")?)?)))
            .collect()
    };
    ensure!(positions(path_doc.root_element())? == positions(doc.root_element())?,
        "input_conflict: 路径与 DOS 网格原子位置或顺序不同");
    let mut elements = Vec::new();
    for atom in &atoms {
        if !elements.iter().any(|e| e == atom) { elements.push(*atom); }
    }
    // 旧 MgB2 包保留已请求但不可用的轨道标签；其他体系不能猜测轨道集合。
    let orbitals = if atoms == ["Mg", "B", "B"] {
        vec!["Mg:s".into(), "Mg:p".into(), "B:s".into(), "B:p".into()]
    } else { vec![] };
    ensure!(
        s.window_ev.iter().all(|v| v.is_finite()) && s.window_ev[0] < s.window_ev[1],
        "data_invalid: 共同能窗不足"
    );
    let mut electronic = String::from("method,segment_id,point_id,distance,band,energy_ev\n");
    let mut phonon =
        String::from("method,reference_kind,segment_id,point_id,distance,mode,frequency_cm1\n");
    for i in 0..n {
        // 相邻段共享端点，但每条线分别保存，不连接断点。
        let segments = if i + 1 < n && s.path[i].segment_id != s.path[i + 1].segment_id {
            vec![s.path[i].segment_id, s.path[i + 1].segment_id]
        } else {
            vec![s.path[i].segment_id]
        };
        for segment in segments {
            for (method, values) in [("qe", &qe.eigenvalues[i]), ("epw", &epw.values[i])] {
                for (b, e) in values.iter().enumerate() {
                    electronic.push_str(&format!(
                        "{method},{segment},{i},{},{},{}\n",
                        s.path[i].distance,
                        b + 1,
                        e - fermi
                    ));
                }
            }
            for (method, kind, values, factor) in [
                ("qe", "qe_matdyn", &qph.values[i], 1.0),
                ("epw", "epw", &eph.values[i], MEV_CM1),
            ] {
                for (m, f) in values.iter().enumerate() {
                    phonon.push_str(&format!(
                        "{method},{kind},{segment},{i},{},{},{}\n",
                        s.path[i].distance,
                        m + 1,
                        f * factor
                    ));
                }
            }
        }
    }
    let mut qe_mesh = Vec::new();
    for p in &s.mesh {
        let expected = cart(*p, &s.reciprocal);
        let indices: Vec<_> = mesh
            .kpoints
            .iter()
            .enumerate()
            .filter(|(_, p)| same(&expected, p))
            .map(|(i, _)| i)
            .collect();
        ensure!(
            indices.len() == 1,
            "data_invalid: 均匀 QE k 点映射缺失或重复"
        );
        qe_mesh.push(
            mesh.eigenvalues[indices[0]]
                .iter()
                .map(|v| v - fermi)
                .collect::<Vec<_>>(),
        );
    }
    let epw_mesh: Vec<Vec<f64>> = epw.values[n..]
        .iter()
        .map(|r| r.iter().map(|v| v - fermi).collect())
        .collect();
    let weights = vec![2.0 / s.mesh.len() as f64; s.mesh.len()];
    let ex = axis(s.window_ev[0], s.window_ev[1], 501);
    let edos = [
        gaussian_dos(&ex, &qe_mesh, &weights, s.electronic_sigma_ev)?,
        gaussian_dos(&ex, &epw_mesh, &weights, s.electronic_sigma_ev)?,
    ];
    let epw_freq: Vec<Vec<f64>> = eph.values[n..]
        .iter()
        .map(|r| r.iter().map(|v| v * MEV_CM1).collect())
        .collect();
    let lo = qph.values[n..]
        .iter()
        .flatten()
        .chain(epw_freq.iter().flatten())
        .copied()
        .fold(0.0, f64::min)
        - 8.0 * s.phonon_sigma_cm1;
    let hi = qph.values[n..]
        .iter()
        .flatten()
        .chain(epw_freq.iter().flatten())
        .copied()
        .fold(0.0, f64::max)
        + 8.0 * s.phonon_sigma_cm1;
    let px = axis(lo, hi, 2001);
    let pw = vec![1.0 / s.mesh.len() as f64; s.mesh.len()];
    let pdos = [
        gaussian_dos(&px, &qph.values[n..], &pw, s.phonon_sigma_cm1)?,
        gaussian_dos(&px, &epw_freq, &pw, s.phonon_sigma_cm1)?,
    ];
    let mut e_csv = String::from("method,energy_ev,projection_kind,element,orbital,density\n");
    let mut p_csv = String::from("method,frequency_cm1,projection_kind,element,density\n");
    for (idx, method) in ["qe", "epw"].iter().enumerate() {
        for (x, y) in ex.iter().zip(&edos[idx]) {
            e_csv.push_str(&format!("{method},{x},total,,,{y}\n"));
        }
        for (x, y) in px.iter().zip(&pdos[idx]) {
            p_csv.push_str(&format!("{method},{x},total,,{y}\n"));
        }
    }
    let missing = vec![
        "electronic element/orbital PDOS: complex atomic projector adapter unavailable".into(),
    ];
    let delta: Vec<f64> = edos[0]
        .iter()
        .zip(&edos[1])
        .map(|(a, b)| (a - b).abs())
        .collect();
    let pdelta: Vec<f64> = pdos[0]
        .iter()
        .zip(&pdos[1])
        .map(|(a, b)| (a - b).abs())
        .collect();
    let distances: Vec<_> = s.path.iter().map(|p|p.distance).collect();
    let path_coordinates: Vec<_> = s.path.iter().map(|p|p.crystal).collect();
    let differences = super::differences::Differences {
        electronic_path: super::differences::electronic(&path_coordinates, Some(&distances),
            &qe.eigenvalues, &epw.values[..n], fermi, s.window_ev)?,
        electronic_mesh: super::differences::electronic(&s.mesh, None,
            &qe_mesh, &epw_mesh, 0.0, s.window_ev)?,
        phonon_path: super::differences::phonon(&path_coordinates, Some(&distances),
            &qph.values[..n], &eph.values[..n], modes)?,
        phonon_mesh: super::differences::phonon(&s.mesh, None,
            &qph.values[n..], &eph.values[n..], modes)?,
    };
    differences.validate()?;
    differences.print();
    let metrics = serde_json::json!({"schema_version":2,"differences":differences,"electronic_matching":"ordered_minimum_squared_energy_pairs","electronic_dos":{"qe_integral":integral(&ex,&edos[0]),"epw_integral":integral(&ex,&edos[1]),"absolute_l1":integral(&ex,&delta),"normalized_l1":(integral(&ex,&edos[0])>0.0).then(||integral(&ex,&delta)/integral(&ex,&edos[0]))},"phonon_dos":{"qe_integral":integral(&px,&pdos[0]),"epw_integral":integral(&px,&pdos[1]),"expected_integral":modes,"absolute_l1":integral(&px,&pdelta),"normalized_l1":(integral(&px,&pdos[0])>0.0).then(||integral(&px,&pdelta)/integral(&px,&pdos[0]))},"kernel":"exp[-(x/sigma)^2]/(sigma*sqrt(pi))","electronic_sigma_ev":s.electronic_sigma_ev,"phonon_sigma_cm1":s.phonon_sigma_cm1,"source_manifest_sha256":store::hash_file(source)?});
    let metadata = Metadata {
        title: format!("{} QE / EPW diagnostic", atoms.join(" ")),
        fermi_ev: fermi,
        window_ev: s.window_ev,
        disentanglement_window_ev,
        frozen_window_ev,
        electronic_ticks: s
            .ticks
            .iter()
            .map(|t| Tick {
                distance: t.distance,
                label: t.label.clone(),
            })
            .collect(),
        phonon_ticks: s.ticks,
        electronic_energy_unit: "eV_relative_fermi".into(),
        phonon_frequency_unit: "cm-1".into(),
        electronic_density_unit: "states/(eV*cell)".into(),
        phonon_density_unit: "states/(cm-1*cell)".into(),
        elements: elements.iter().map(|e| (*e).into()).collect(),
        orbitals,
        electronic_projection: "unavailable".into(),
        phonon_projection: "unavailable".into(),
        phonon_error_markers: vec![],
        notes: vec![
            format!(
                "fixed {}x{}x{} DOS mesh; Gaussian sigma_e={} eV, sigma_ph={} cm-1",
                s.mesh_shape[0],
                s.mesh_shape[1],
                s.mesh_shape[2],
                s.electronic_sigma_ev,
                s.phonon_sigma_cm1
            ),
            "QE: direct PW bands / q2r-matdyn phonons. Energy pairs do not establish eigenstate identity."
                .into(),
        ],
    };
    let destination = output;
    let _lock = store::Lock::acquire(destination)?;
    ensure!(!destination.exists(), "input_conflict: 比较输出已存在");
    let stage = destination
        .parent()
        .context("data_invalid: 比较输出缺父目录")?
        .join(format!(".comparison-{}", store::token()));
    fs::create_dir(&stage)?;
    let output = stage.as_path();
    for (name, text) in [
        (TABLES[0], electronic),
        (TABLES[1], phonon),
        (TABLES[2], e_csv),
        (TABLES[3], p_csv),
    ] {
        store::atomic_bytes(&output.join(name), text.as_bytes())?;
    }
    store::write(&output.join("metadata.json"), &metadata)?;
    store::write(&output.join("metrics.json"), &metrics)?;
    let files = TABLES
        .iter()
        .chain(["metadata.json", "metrics.json"].iter())
        .map(|name| Artifact::capture(output, name))
        .collect::<Result<_>>()?;
    store::write(
        &output.join("manifest.json"),
        &Manifest {
            schema_version: 2,
            run_id: s.run_id,
            ph_mode: s.ph_mode,
            upstream_digest: store::hash_file(source)?,
            quality_status: None,
            missing,
            files,
        },
    )?;
    super::comparison_data::load(output)?;
    fs::rename(output, destination)?;
    Ok(())
}
