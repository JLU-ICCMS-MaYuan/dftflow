//! 原生诊断数据适配。仅解析已有结果及累积 DOS，不启动 QE/EPW。
use super::{
    native,
    native_quality::{self, SourceFile},
};
use crate::{
    foundation::store,
    interpolation::comparison_data::{
        Artifact, Manifest, Metadata, PhononErrorMarker, Tick, TABLES,
    },
    qe_data::nscf,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

const MEV_CM1: f64 = 8.065543937;

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
    pub quality_reference: SourceFile,
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

/// 读取 Wannier 实际采用的绝对能量外窗，也覆盖未显式设置 dis_win 的情况。
pub fn parse_outer_window(text: &str) -> Result<[f64; 2]> {
    let pattern = regex::Regex::new(r"(?m)^\s*\|\s*Outer:\s*(\S+)\s+to\s+(\S+)\s+\(eV\)")?;
    let records: Vec<_> = pattern.captures_iter(text).collect();
    ensure!(
        records.len() == 1,
        "data_invalid: Wannier 日志必须有唯一 Outer 能窗"
    );
    let value = |i: usize| -> Result<f64> { Ok(records[0][i].replace(['d', 'D'], "e").parse()?) };
    let window = [value(1)?, value(2)?];
    ensure!(
        window.iter().all(|v| v.is_finite()) && window[0] < window[1],
        "data_invalid: Wannier 外窗必须有限且递增"
    );
    Ok(window)
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

fn phonon_error_markers(
    reference: &native::NativeBands,
    interpolated: &native::NativeBands,
    source: &Sources,
) -> Result<Vec<PhononErrorMarker>> {
    // 用倒格矢的对偶基恢复晶体坐标，不能把 QE 笛卡尔坐标直接标成晶体坐标。
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|i| a[i] * b[i]).sum::<f64>();
    let b = source.reciprocal;
    let dual = [cross(b[1], b[2]), cross(b[2], b[0]), cross(b[0], b[1])];
    let determinant = dot(b[0], dual[0]);
    ensure!(
        determinant.is_finite() && determinant.abs() > 1e-12,
        "data_invalid: 倒格矢矩阵不可逆"
    );
    let mut samples = Vec::new();
    for (i, q) in reference.points_cartesian.iter().enumerate() {
        let path_point = source.path.iter().find(|p| same(q, &cart(p.crystal, &b)));
        let path_label = path_point.map(|p| {
            if let Some(t) = source
                .ticks
                .iter()
                .find(|t| (t.distance - p.distance).abs() < 1e-8)
            {
                format!("{} (high-symmetry point)", t.label)
            } else {
                let left = source.ticks.iter().rev().find(|t| t.distance < p.distance);
                let right = source.ticks.iter().find(|t| t.distance > p.distance);
                match (left, right) {
                    (Some(l), Some(r)) => format!("{} - {} (interior)", l.label, r.label),
                    _ => "path interior".into(),
                }
            }
        });
        let mut qe = reference.values[i].clone();
        let mut epw: Vec<_> = interpolated.values[i].iter().map(|v| v * MEV_CM1).collect();
        qe.sort_by(f64::total_cmp);
        epw.sort_by(f64::total_cmp);
        for (mode, (qe, epw)) in qe.into_iter().zip(epw).enumerate() {
            samples.push(PhononErrorMarker {
                scope: "sampled_maximum".into(),
                q_crystal: std::array::from_fn(|j| dot(*q, dual[j]) / determinant),
                mode_sorted: mode + 1,
                qe_cm1: qe,
                epw_cm1: epw,
                error_cm1: (qe - epw).abs(),
                path_distance: path_point.map(|p| p.distance),
                path_label: path_label.clone(),
            });
        }
    }
    let largest = |on_path: bool| {
        samples
            .iter()
            .filter(|s| !on_path || s.path_distance.is_some())
            .max_by(|a, b| a.error_cm1.total_cmp(&b.error_cm1))
            .cloned()
    };
    let mut markers = vec![largest(false).context("data_invalid: 声子验证点为空")?];
    if let Some(mut marker) = largest(true) {
        marker.scope = "path_maximum".into();
        markers.push(marker);
    }
    Ok(markers)
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
        s.schema_version == 1 && matches!(s.ph_mode.as_str(), "nosplit" | "index" | "coords"),
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
    let disentanglement_window_ev = s
        .wannier_wout
        .as_ref()
        .map(|file| parse_outer_window(&fs::read_to_string(file.checked_path(base)?)?))
        .transpose()?;
    if let Some(window) = disentanglement_window_ev {
        s.window_ev = window.map(|e| e - fermi);
    }
    let qe = nscf::parse_xml(&s.qe_bands_xml.checked_path(base)?)?;
    let mesh = nscf::parse_xml(&s.qe_mesh_xml.checked_path(base)?)?;
    let epw = native::parse_bands(&fs::read_to_string(s.epw_bands.checked_path(base)?)?)?;
    let qph = native::parse_matdyn(&fs::read_to_string(s.qe_phonon_bands.checked_path(base)?)?)
        .context("data_invalid: QE matdyn 频率文件")?;
    let eph = native::parse_bands(&fs::read_to_string(s.epw_phonon_bands.checked_path(base)?)?)?;
    native::checked_log(&fs::read_to_string(s.epw_log.checked_path(base)?)?, 0)?;
    let quality_path = s.quality_reference.checked_path(base)?;
    let quality = native_quality::inspect(&quality_path)?;
    let quality_sources: native_quality::QualitySources = store::read(&quality_path)?;
    let reference_base = quality_path
        .parent()
        .context("data_invalid: 质量来源缺目录")?;
    let direct = native::parse_bands(&fs::read_to_string(
        quality_sources
            .phonon_reference_cm1
            .checked_path(reference_base)?,
    )?)?;
    let quality_interpolated = native::parse_bands(&fs::read_to_string(
        quality_sources
            .phonon_interpolated_mev
            .checked_path(reference_base)?,
    )?)?;
    let error_markers = phonon_error_markers(&direct, &quality_interpolated, &s)?;
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
        modes == 9
            && eph
                .values
                .iter()
                .chain(&qph.values)
                .all(|v| v.len() == modes),
        "data_invalid: 声子模式不完整"
    );
    // 首个原生导出适配器限定本次 MgB2 诊断，不能误标其他体系的元素。
    let qe_text = fs::read_to_string(s.qe_mesh_xml.checked_path(base)?)?;
    let doc = roxmltree::Document::parse(&qe_text)?;
    let atoms: Vec<_> = doc
        .descendants()
        .find(|n| n.has_tag_name("atomic_positions"))
        .context("data_invalid: 缺少原子顺序")?
        .children()
        .filter(|n| n.has_tag_name("atom"))
        .filter_map(|n| n.attribute("name"))
        .collect();
    ensure!(
        atoms == ["Mg", "B", "B"],
        "data_invalid: 本适配器只验证 MgB2 原子顺序"
    );
    ensure!(
        s.window_ev.iter().all(|v| v.is_finite())
            && s.window_ev[0] <= -0.5
            && s.window_ev[1] >= 0.5,
        "data_invalid: 共同能窗不足"
    );
    let mut electronic = String::from("method,segment_id,point_id,distance,band,energy_ev\n");
    let mut phonon =
        String::from("method,reference_kind,segment_id,point_id,distance,mode,frequency_cm1\n");
    let mut direct_csv = String::from("point_id,distance,mode,frequency_cm1,source\n");
    for point in &s.path {
        let expected = cart(point.crystal, &s.reciprocal);
        for (q, values) in direct.points_cartesian.iter().zip(&direct.values) {
            if same(&expected, q) {
                for (m, f) in values.iter().enumerate() {
                    direct_csv.push_str(&format!(
                        "{},{},{},{},qe_ph_direct\n",
                        point.point_id,
                        point.distance,
                        m + 1,
                        f
                    ));
                }
            }
        }
    }
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
    let mut missing = vec![
        "electronic element/orbital PDOS: complex atomic projector adapter unavailable".into(),
        "phonon element DOS: EPW mass-weighted eigenvectors unavailable".into(),
    ];
    let mut uncovered = 0;
    for (q, e) in qe_mesh.iter().zip(&epw_mesh) {
        let in_window: Vec<_> = q
            .iter()
            .filter(|v| {
                **v >= s.window_ev[0] - 6.0 * s.electronic_sigma_ev
                    && **v <= s.window_ev[1] + 6.0 * s.electronic_sigma_ev
            })
            .collect();
        let mut used = std::collections::BTreeSet::new();
        for v in in_window {
            if let Some((j, _)) = e
                .iter()
                .enumerate()
                .filter(|(j, _)| !used.contains(j))
                .min_by(|a, b| (a.1 - v).abs().total_cmp(&(b.1 - v).abs()))
            {
                if (e[j] - v).abs() <= 0.05 {
                    used.insert(j);
                } else {
                    uncovered += 1;
                }
            } else {
                uncovered += 1;
            }
        }
    }
    if uncovered > 0 {
        missing.push(format!("electronic DOS window/broadening coverage: {uncovered} unmatched reference states; Wannier subspace diagnostic only"));
    }
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
    let metrics = serde_json::json!({"schema_version":1,"quality":quality,"electronic_dos":{"qe_integral":integral(&ex,&edos[0]),"epw_integral":integral(&ex,&edos[1]),"absolute_l1":integral(&ex,&delta),"normalized_l1":(integral(&ex,&edos[0])>0.0).then(||integral(&ex,&delta)/integral(&ex,&edos[0])),"unmatched_window_states":uncovered},"phonon_dos":{"qe_integral":integral(&px,&pdos[0]),"epw_integral":integral(&px,&pdos[1]),"expected_integral":modes,"absolute_l1":integral(&px,&pdelta),"normalized_l1":(integral(&px,&pdos[0])>0.0).then(||integral(&px,&pdelta)/integral(&px,&pdos[0]))},"kernel":"exp[-(x/sigma)^2]/(sigma*sqrt(pi))","electronic_sigma_ev":s.electronic_sigma_ev,"phonon_sigma_cm1":s.phonon_sigma_cm1,"source_manifest_sha256":store::hash_file(source)?});
    let metadata = Metadata {
        title: "MgB2 QE / EPW diagnostic".into(),
        fermi_ev: fermi,
        window_ev: s.window_ev,
        disentanglement_window_ev,
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
        elements: vec!["Mg".into(), "B".into()],
        orbitals: vec!["Mg:s".into(), "Mg:p".into(), "B:s".into(), "B:p".into()],
        electronic_projection: "unavailable".into(),
        phonon_projection: "unavailable".into(),
        phonon_error_markers: error_markers,
        notes: vec![
            format!(
                "fixed {}x{}x{} DOS mesh; Gaussian sigma_e={} eV, sigma_ph={} cm-1",
                s.mesh_shape[0],
                s.mesh_shape[1],
                s.mesh_shape[2],
                s.electronic_sigma_ev,
                s.phonon_sigma_cm1
            ),
            "QE: direct PW bands / q2r-matdyn phonons. Quality failed data remain diagnostic."
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
        (TABLES[2], direct_csv),
        (TABLES[3], e_csv),
        (TABLES[4], p_csv),
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
            schema_version: 1,
            run_id: s.run_id,
            ph_mode: s.ph_mode,
            upstream_digest: store::hash_file(source)?,
            quality_status: if quality.passed { "passed" } else { "failed" }.into(),
            missing,
            files,
        },
    )?;
    super::comparison_data::load(output)?;
    fs::rename(output, destination)?;
    Ok(())
}
