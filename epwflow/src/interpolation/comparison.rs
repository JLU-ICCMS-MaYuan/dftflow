//! 原生诊断数据适配。仅解析已有结果及累积 DOS，不启动 QE/EPW。
use super::{differences::MEV_CM1, native};
use crate::{
    foundation::store,
    interpolation::comparison_data::{Artifact, Manifest, Metadata, Tick, TABLES},
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
            "data_invalid: It's not the same from the source {}",
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub electronic_stage: Option<ElectronicStage>,
    pub qe_phonon_bands: SourceFile,
    /// 新流程直接消费第 4 步原生 DOS；旧只读来源没有此字段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qe_phonon_dos: Option<SourceFile>,
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

/// 第 6 步只消费第 5 步已完成的电子表格，不强迫电子取点服从声子取点。
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElectronicStage {
    pub bands: SourceFile,
    pub dos: SourceFile,
    pub metrics: SourceFile,
    pub sampling: SourceFile,
}
impl ElectronicStage {
    pub(crate) fn capture(dir: &Path) -> Result<Self> {
        let file = |name: &str| -> Result<SourceFile> {
            let path = dir.join(name).canonicalize()?;
            Ok(SourceFile {
                sha256: store::hash_file(&path)?,
                path,
            })
        };
        Ok(Self {
            bands: file("electronic-bands.csv")?,
            dos: file("electronic-dos.csv")?,
            metrics: file("metrics.json")?,
            sampling: file("sampling.json")?,
        })
    }
    fn load(
        &self,
        base: &Path,
    ) -> Result<(
        String,
        String,
        serde_json::Value,
        super::electronic_sampling::Sampling,
    )> {
        let bands = fs::read_to_string(self.bands.checked_path(base)?)?;
        ensure!(
            bands.lines().next() == Some("method,segment_id,point_id,distance,band,energy_ev"),
            "data_invalid: I don't think so 5 Step electrons can't work with the header"
        );
        let dos = fs::read_to_string(self.dos.checked_path(base)?)?;
        ensure!(
            dos.lines().next() == Some("source,energy_minus_fermi_ev,states_per_ev_cell"),
            "data_invalid: I don't think so 5 Step electrons DOS The header is invalid"
        );
        let mut converted =
            String::from("method,energy_ev,projection_kind,element,orbital,density\n");
        for line in dos.lines().skip(1).filter(|l| !l.trim().is_empty()) {
            let cells: Vec<_> = line.split(',').collect();
            ensure!(
                cells.len() == 3 && matches!(cells[0], "QE" | "EPW"),
                "data_invalid: I don't think so 5 Step DOS Line invalid"
            );
            let e: f64 = cells[1].parse()?;
            let d: f64 = cells[2].parse()?;
            ensure!(
                e.is_finite() && d.is_finite(),
                "data_invalid: I don't think so 5 Step DOS Unlimited"
            );
            converted.push_str(&format!(
                "{},{e},total,,,{d}\n",
                cells[0].to_ascii_lowercase()
            ));
        }
        let metrics = store::read(&self.metrics.checked_path(base)?)?;
        let sampling = store::read(&self.sampling.checked_path(base)?)?;
        Ok((bands, converted, metrics, sampling))
    }
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
        "data_invalid: Wannier The log must have one {name} The window"
    );
    let pattern = regex::Regex::new(r"^\s*(\S+)\s+to\s+(\S+)\s+\(eV\)")?;
    let record = pattern
        .captures(records[0])
        .with_context(|| format!("data_invalid: Wannier {name} Could not initialise Bonobo"))?;
    let value = |i: usize| -> Result<f64> { Ok(record[i].replace(['d', 'D'], "e").parse()?) };
    let window = [value(1)?, value(2)?];
    ensure!(
        window.iter().all(|v| v.is_finite()) && window[0] < window[1],
        "data_invalid: Wannier {name} The window must be finite and incremental"
    );
    Ok(Some(window))
}

/// 读取 Wannier 实际采用的绝对能量外窗，也覆盖未显式设置 dis_win 的情况。
pub fn parse_outer_window(text: &str) -> Result<[f64; 2]> {
    parse_window(text, "Outer")?.context("data_invalid: Wannier Log Missing Outer The window")
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
        "data_invalid: DOS Grid, state, weight or nuclear width invalid"
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

/// 将实际原生坐标放回声明的物理路径；不生成能量或频率，不要求内部点重合。
pub(crate) fn native_path(
    reference: &[PathPoint],
    coordinates: &[[f64; 3]],
    reciprocal: &[[f64; 3]; 3],
) -> Result<Vec<PathPoint>> {
    ensure!(
        reference.len() >= 2 && coordinates.len() >= 2,
        "data_invalid: Incomplete native path coordinates or endpoints"
    );
    let mut path: Vec<PathPoint> = Vec::new();
    for (i, point) in coordinates.iter().enumerate() {
        ensure!(
            point.iter().all(|v| v.is_finite()),
            "data_invalid: Non-finite path coordinates"
        );
        let mut found = None;
        for edge in reference
            .windows(2)
            .filter(|w| w[0].segment_id == w[1].segment_id)
        {
            let a = cart(edge[0].crystal, reciprocal);
            let b = cart(edge[1].crystal, reciprocal);
            let delta: [f64; 3] = std::array::from_fn(|j| b[j] - a[j]);
            let norm: f64 = delta.iter().map(|v| v * v).sum();
            if norm <= 1e-24 {
                continue;
            }
            let t = if same(point, &a) {
                0.
            } else if same(point, &b) {
                1.
            } else {
                (0..3).map(|j| (point[j] - a[j]) * delta[j]).sum::<f64>() / norm
            };
            let projected: [f64; 3] = std::array::from_fn(|j| a[j] + t * delta[j]);
            if !(-1e-5..=1.00001).contains(&t) || !same(point, &projected) {
                continue;
            }
            let distance =
                edge[0].distance + t.clamp(0., 1.) * (edge[1].distance - edge[0].distance);
            if path.last().is_some_and(|p| distance <= p.distance + 1e-10) {
                continue;
            }
            found = Some(PathPoint {
                segment_id: edge[0].segment_id,
                point_id: i,
                distance,
                crystal: std::array::from_fn(|j| {
                    edge[0].crystal[j] + t.clamp(0., 1.) * (edge[1].crystal[j] - edge[0].crystal[j])
                }),
            });
            break;
        }
        path.push(
            found
                .with_context(|| format!("data_invalid: Native coordinate deviation from physical path or point sequence error, point {i}"))?,
        );
    }
    // 必须保留真实拐点和断段端点，不能用跨过拐点的直线冒充原路径。
    for (i, p) in reference.iter().enumerate() {
        let endpoint = i == 0
            || i + 1 == reference.len()
            || reference[i - 1].segment_id != p.segment_id
            || reference[i + 1].segment_id != p.segment_id;
        let corner = if endpoint {
            false
        } else {
            let a: [f64; 3] = std::array::from_fn(|j| p.crystal[j] - reference[i - 1].crystal[j]);
            let b: [f64; 3] = std::array::from_fn(|j| reference[i + 1].crystal[j] - p.crystal[j]);
            let cross = [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ];
            cross.iter().map(|v| v * v).sum::<f64>() > 1e-20
                || a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>() < 0.
        };
        ensure!(
            !(endpoint || corner)
                || path.iter().any(|q| q.segment_id == p.segment_id
                    && (q.distance - p.distance).abs() < 1e-6
                    && same(&q.crystal, &p.crystal)),
            "data_invalid: No high symmetric endpoint or turning point for native path coordinates"
        );
    }
    Ok(path)
}

fn common_phonon_difference(
    qe_path: &[PathPoint],
    qe: &[Vec<f64>],
    epw_path: &[PathPoint],
    epw: &[Vec<f64>],
    modes: usize,
) -> Result<super::differences::Statistics> {
    let mut points = Vec::new();
    let mut distance = Vec::new();
    let mut q = Vec::new();
    let mut e = Vec::new();
    for (i, p) in qe_path.iter().enumerate() {
        if let Some(j) = epw_path.iter().position(|r| {
            p.segment_id == r.segment_id
                && same(&p.crystal, &r.crystal)
                && (p.distance - r.distance).abs() < 1e-6
        }) {
            points.push(p.crystal);
            distance.push(p.distance);
            q.push(qe[i].clone());
            e.push(epw[j].clone());
        }
    }
    super::differences::phonon(&points, Some(&distance), &q, &e, modes)
}

fn axis(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| lo + (hi - lo) * i as f64 / (n - 1) as f64)
        .collect()
}
/// 保留 QE 原生频率轴和总 DOS；其余原子投影列不在这里重新累计。
fn qe_dos(text: &str) -> Result<(Vec<f64>, Vec<f64>)> {
    let mut axis = Vec::new();
    let mut density = Vec::new();
    for line in text.lines().map(str::trim).filter(|s| !s.is_empty() && !s.starts_with('#')) {
        let values = crate::qe_data::xml::numbers(line)?;
        ensure!(values.len() >= 2 && values.iter().all(|v| v.is_finite()) && values[1] >= 0.,
            "data_invalid: QE phonon DOS has invalid values");
        axis.push(values[0]); density.push(values[1]);
    }
    ensure!(axis.len() >= 2 && axis.windows(2).all(|w| w[0] < w[1]),
        "data_invalid: QE phonon DOS axis is incomplete or unordered");
    Ok((axis, density))
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
        "data_invalid: Source version or PH Mode invalid"
    );
    let base = source.parent().context("data_invalid: Source directory missing")?;
    ensure!(!output.exists(), "input_conflict: Compare Output Exists");
    ensure!(
        s.path.len() >= 2
            && s.path
                .iter()
                .all(|p| p.crystal.iter().all(|v| v.is_finite()) && p.distance.is_finite())
            && s.path.windows(2).all(|p| p[0].distance < p[1].distance)
            && s.reciprocal.iter().flatten().all(|v| v.is_finite()),
        "data_invalid: Path incomplete or distance is not incremental"
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
    let qph =
        native::parse_matdyn_path(&fs::read_to_string(s.qe_phonon_bands.checked_path(base)?)?)
            .context("data_invalid: QE matdyn Frequency File")?;
    let eph = native::parse_bands(&fs::read_to_string(s.epw_phonon_bands.checked_path(base)?)?)?;
    native::checked_log(&fs::read_to_string(s.epw_log.checked_path(base)?)?, 0)?;
    for p in &s.provenance {
        p.checked_path(base)?;
    }
    let electronic_stage = s
        .electronic_stage
        .as_ref()
        .map(|files| files.load(base))
        .transpose()?;
    let path_count = |len: usize| -> Result<usize> {
        let count = len.checked_sub(s.mesh.len()).context("data_invalid: Numerical grid points incomplete")?;
        ensure!(count >= 2, "data_invalid: Native Path Endpoint Incomplete");
        Ok(count)
    };
    let native_qe_dos = s.qe_phonon_dos.as_ref().map(|file| {
        qe_dos(&fs::read_to_string(file.checked_path(base)?)?)
    }).transpose()?;
    let nq = if native_qe_dos.is_some() { qph.values.len() } else { path_count(qph.values.len())? };
    let ne = path_count(eph.values.len())?;
    let qe_ph_path = native_path(&s.path, &qph.points_cartesian[..nq], &s.reciprocal)?;
    let epw_ph_path = native_path(&s.path, &eph.points_cartesian[..ne], &s.reciprocal)?;
    let electronic_paths = if electronic_stage.is_none() {
        let count = path_count(epw.values.len())?;
        Some((native_path(&s.path, &qe.kpoints, &s.reciprocal)?,
            native_path(&s.path, &epw.points_cartesian[..count], &s.reciprocal)?))
    } else { None };
    ensure!(mesh.kpoints.len() == s.mesh.len(), "data_invalid: QE DOS Grid points incomplete");
    let mut mesh_set = std::collections::BTreeSet::new();
    ensure!(
        s.mesh_shape.iter().all(|n| *n > 0)
            && s.mesh_shape
                .iter()
                .try_fold(1usize, |a, b| a.checked_mul(*b))
                == Some(s.mesh.len()),
        "data_invalid: DOS It has to come from a complete even grid"
    );
    for p in &s.mesh {
        let mut key = [0usize; 3];
        for i in 0..3 {
            let x = p[i] * s.mesh_shape[i] as f64;
            ensure!(
                x.is_finite() && (x - x.round()).abs() < 1e-7 && p[i] >= 0.0 && p[i] < 1.0,
                "data_invalid: DOS Grid is not fixed Gamma Flat Grid"
            );
            key[i] = x.round() as usize;
        }
        ensure!(mesh_set.insert(key), "data_invalid: Repeat DOS Grid Point");
    }
    ensure!(
        mesh.weights
            .iter()
            .all(|w| (*w - mesh.weights[0]).abs() < 1e-8),
        "data_invalid: Full grid weight is uneven"
    );
    // DOS 使用自身完整均匀网格；路径采样独立，不参与网格身份检查。
    for (i, p) in s.mesh.iter().enumerate() {
        let expected = cart(*p, &s.reciprocal);
        ensure!((native_qe_dos.is_some() || same(&expected, &qph.points_cartesian[nq+i]))
            && same(&expected, &eph.points_cartesian[ne+i])
            && electronic_paths.as_ref().is_none_or(|(_, path)| same(&expected, &epw.points_cartesian[path.len()+i])),
            "data_invalid: Native DOS Grid coordinates don't match, point {i}");
    }
    let modes = eph.values[0].len();
    ensure!(
        modes > 0
            && eph
                .values
                .iter()
                .chain(&qph.values)
                .all(|v| v.len() == modes),
        "data_invalid: Sound submodule incomplete"
    );
    // 总量比较按实际原子顺序验证 3N；缺少算符时不推断元素／轨道投影。
    let qe_text = fs::read_to_string(s.qe_mesh_xml.checked_path(base)?)?;
    let doc = roxmltree::Document::parse(&qe_text)?;
    let output_structure = doc
        .descendants()
        .find(|n| n.has_tag_name("output"))
        .context("data_invalid: Missing QE output Structure")?;
    let atoms: Vec<_> = output_structure
        .descendants()
        .find(|n| n.has_tag_name("atomic_positions"))
        .context("data_invalid: Missing Atom Order")?
        .children()
        .filter(|n| n.has_tag_name("atom"))
        .map(|n| n.attribute("name").context("data_invalid: Atom Missing Element Name"))
        .collect::<Result<_>>()?;
    ensure!(
        !atoms.is_empty() && modes == 3 * atoms.len(),
        "data_invalid: The number of acoustics differs from the number of atoms"
    );
    let path_text = fs::read_to_string(s.qe_bands_xml.checked_path(base)?)?;
    let path_doc = roxmltree::Document::parse(&path_text)?;
    let positions = |root: roxmltree::Node<'_, '_>| -> Result<Vec<(String, [f64; 3])>> {
        root.descendants()
            .find(|n| n.has_tag_name("output"))
            .context("data_invalid: Missing QE output")?
            .descendants()
            .find(|n| n.has_tag_name("atomic_positions"))
            .context("data_invalid: Missing Atom Location")?
            .children()
            .filter(|n| n.has_tag_name("atom"))
            .map(|n| {
                Ok((
                    n.attribute("name")
                        .context("data_invalid: Missing Element Name")?
                        .into(),
                    crate::qe_data::xml::vector(n.text().context("data_invalid: Missing Atomic Coordinates")?)?,
                ))
            })
            .collect()
    };
    ensure!(
        positions(path_doc.root_element())? == positions(doc.root_element())?,
        "input_conflict: Paths and DOS The grid atoms have different positions or sequences"
    );
    let mut elements = Vec::new();
    for atom in &atoms {
        if !elements.iter().any(|e| e == atom) {
            elements.push(*atom);
        }
    }
    // 旧 MgB2 包保留已请求但不可用的轨道标签；其他体系不能猜测轨道集合。
    let orbitals = if atoms == ["Mg", "B", "B"] {
        vec!["Mg:s".into(), "Mg:p".into(), "B:s".into(), "B:p".into()]
    } else {
        vec![]
    };
    ensure!(
        s.window_ev.iter().all(|v| v.is_finite()) && s.window_ev[0] < s.window_ev[1],
        "data_invalid: We don't have enough windows together"
    );
    let mut electronic = String::from("method,segment_id,point_id,distance,band,energy_ev\n");
    let mut phonon =
        String::from("method,reference_kind,segment_id,point_id,distance,mode,frequency_cm1\n");
    if let Some((qe_path, epw_path)) = &electronic_paths {
        for (method, path, values) in [("qe", qe_path, &qe.eigenvalues), ("epw", epw_path, &epw.values)] {
            for (i, point) in path.iter().enumerate() {
                for (b, e) in values[i].iter().enumerate() {
                    electronic.push_str(&format!("{method},{},{i},{},{},{}\n", point.segment_id, point.distance, b+1, e-fermi));
                }
            }
        }
    }
    for (method, kind, path, values, factor) in [
        ("qe", "qe_matdyn", &qe_ph_path, &qph.values, 1.0),
        ("epw", "epw", &epw_ph_path, &eph.values, MEV_CM1),
    ] {
        for (i, point) in path.iter().enumerate() {
            for (m, f) in values[i].iter().enumerate() {
                phonon.push_str(&format!("{method},{kind},{},{i},{},{},{}\n", point.segment_id, point.distance, m+1, f*factor));
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
            "data_invalid: even QE k PointMap Missing or Repeated"
        );
        qe_mesh.push(
            mesh.eigenvalues[indices[0]]
                .iter()
                .map(|v| v - fermi)
                .collect::<Vec<_>>(),
        );
    }
    let epw_mesh: Vec<Vec<f64>> = if electronic_stage.is_some() {
        vec![]
    } else {
        epw.values[electronic_paths.as_ref().unwrap().1.len()..]
            .iter()
            .map(|r| r.iter().map(|v| v - fermi).collect())
            .collect()
    };
    let weights = vec![2.0 / s.mesh.len() as f64; s.mesh.len()];
    let ex = axis(s.window_ev[0], s.window_ev[1], 501);
    let edos = if electronic_stage.is_some() {
        [Vec::new(), Vec::new()]
    } else {
        [
            gaussian_dos(&ex, &qe_mesh, &weights, s.electronic_sigma_ev)?,
            gaussian_dos(&ex, &epw_mesh, &weights, s.electronic_sigma_ev)?,
        ]
    };
    let epw_freq: Vec<Vec<f64>> = eph.values[ne..]
        .iter()
        .map(|r| r.iter().map(|v| v * MEV_CM1).collect())
        .collect();
    let qe_mesh_frequency = &qph.values[nq..];
    let lo = qe_mesh_frequency.iter().flatten().chain(epw_freq.iter().flatten())
        .copied().fold(0.0, f64::min) - 8.0 * s.phonon_sigma_cm1;
    let hi = qe_mesh_frequency.iter().flatten().chain(epw_freq.iter().flatten())
        .copied().fold(0.0, f64::max) + 8.0 * s.phonon_sigma_cm1;
    let px = axis(lo, hi, 2001);
    let pw = vec![1.0 / s.mesh.len() as f64; s.mesh.len()];
    let qe_density = if let Some(data) = native_qe_dos { data } else {
        (px.clone(), gaussian_dos(&px, qe_mesh_frequency, &pw, s.phonon_sigma_cm1)?)
    };
    let epw_density = gaussian_dos(&px, &epw_freq, &pw, s.phonon_sigma_cm1)?;
    let mut e_csv = String::from("method,energy_ev,projection_kind,element,orbital,density\n");
    let mut p_csv = String::from("method,frequency_cm1,projection_kind,element,density\n");
    for (idx, method) in ["qe", "epw"].iter().enumerate() {
        for (x, y) in ex.iter().zip(&edos[idx]) {
            e_csv.push_str(&format!("{method},{x},total,,,{y}\n"));
        }
        let (x_axis, density) = if idx == 0 { (&qe_density.0, &qe_density.1) } else { (&px, &epw_density) };
        for (x, y) in x_axis.iter().zip(density) {
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
    let differences = super::differences::Differences {
        electronic_path: if let Some((_, _, metrics, _)) = &electronic_stage {
            serde_json::from_value(metrics["electronic_path"].clone())?
        } else {
            let (qe_path, epw_path) = electronic_paths.as_ref().unwrap();
            super::electronic_sampling::common_difference(qe_path, &qe.eigenvalues,
                epw_path, &epw.values[..epw_path.len()], fermi, s.window_ev)?
        },
        electronic_mesh: if let Some((_, _, metrics, _)) = &electronic_stage {
            serde_json::from_value(metrics["electronic_mesh"].clone())?
        } else {
            super::differences::electronic(&s.mesh, None, &qe_mesh, &epw_mesh, 0.0, s.window_ev)?
        },
        phonon_path: common_phonon_difference(&qe_ph_path, &qph.values[..nq],
            &epw_ph_path, &eph.values[..ne], modes)?,
        phonon_mesh: if s.qe_phonon_dos.is_some() {
            // 第 4 步没有相同网格上的逐模频率，不补算或伪造网格差值。
            super::differences::phonon(&[], None, &[], &[], modes)?
        } else {
            super::differences::phonon(&s.mesh, None, qe_mesh_frequency, &eph.values[ne..], modes)?
        },
    };
    differences.validate()?;
    differences.print();
    let mut metrics = serde_json::json!({"schema_version":2,"differences":differences,"electronic_matching":"ordered_minimum_squared_energy_pairs","electronic_dos":{"qe_integral":integral(&ex,&edos[0]),"epw_integral":integral(&ex,&edos[1]),"absolute_l1":integral(&ex,&delta),"normalized_l1":(integral(&ex,&edos[0])>0.0).then(||integral(&ex,&delta)/integral(&ex,&edos[0]))},"phonon_dos":{"qe_integral":integral(&qe_density.0,&qe_density.1),"epw_integral":integral(&px,&epw_density),"expected_integral":modes},"kernel":"exp[-(x/sigma)^2]/(sigma*sqrt(pi))","electronic_sigma_ev":s.electronic_sigma_ev,"phonon_sigma_cm1":s.phonon_sigma_cm1,"source_manifest_sha256":store::hash_file(source)?});
    if let Some((bands, dos, _, _)) = &electronic_stage {
        electronic = bands.clone();
        e_csv = dos.clone();
        // DOS 直接复用原生表格，不以占位网格造出差值或归一化结论。
        metrics.as_object_mut().unwrap().remove("electronic_dos");
    }
    let mut metadata = Metadata {
        title: format!("{} QE / EPW diagnostic", atoms.join(" ")),
        fermi_ev: fermi,
        window_ev: s.window_ev,
        disentanglement_window_ev,
        frozen_window_ev,
        electronic_independent_sampling: electronic_stage.is_some(),
        electronic_ticks: electronic_stage.as_ref().map(|(_,_,_,sampling)|&sampling.ticks).unwrap_or(&s.ticks)
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
            if s.qe_phonon_dos.is_some() {
                "QE phonons/DOS reused unchanged from stage 4 (its native ASR and DOS method); EPW uses native interpolation and its own Gaussian DOS. Differences can include treatment and sampling effects."
            } else { "QE: direct PW bands / q2r-matdyn phonons. Energy pairs do not establish eigenstate identity." }
                .into(),
        ],
    };
    if s.qe_phonon_dos.is_some() {
        metadata.notes[0] = format!("EPW phonon DOS: {}x{}x{} mesh; Gaussian sigma={} cm-1. QE DOS: unchanged stage-4 native integration.",
            s.mesh_shape[0], s.mesh_shape[1], s.mesh_shape[2], s.phonon_sigma_cm1);
        let asr = regex::Regex::new(r"(?im)^\s*(zasr|asr)\s*=\s*([^,!\n/]+)")?;
        let owner = base.join(&s.qe_phonon_bands.path);
        for input in &s.provenance {
            let path = base.join(&input.path);
            let name = path.file_name().and_then(|n|n.to_str()).unwrap_or("");
            if path.parent() == owner.parent() && matches!(name, "q2r.in" | "matdyn_phband.in" | "matdyn_phdos.in") {
                let text = fs::read_to_string(&path)?;
                let setting = asr.captures(&text).map(|c|format!("{}={}", &c[1], c[2].trim()))
                    .unwrap_or_else(|| "ASR not specified; native default".into());
                metadata.notes.push(format!("QE stage 4 {name}: {setting}"));
            }
        }
    }
    let destination = output;
    let _lock = store::Lock::acquire(destination)?;
    ensure!(!destination.exists(), "input_conflict: Compare Output Exists");
    let stage = destination
        .parent()
        .context("data_invalid: Compare output missing parent directory")?
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

#[cfg(test)]
mod sampling_tests {
    use super::*;
    fn point(i: usize, x: f64) -> PathPoint {
        PathPoint {
            segment_id: 0,
            point_id: i,
            distance: x,
            crystal: [x, 0., 0.],
        }
    }
    #[test]
    fn native_path_keeps_nonuniform_samples_but_rejects_wrong_path_and_missing_endpoints() {
        let reference = vec![point(0, 0.), point(1, 0.5), point(2, 1.)];
        let reciprocal = [[1., 0.5, 0.], [0., 1., 0.], [0., 0., 1.]];
        let coordinates = [[0., 0., 0.], [0.13, 0.065, 0.], [1., 0.5, 0.]];
        let path = native_path(&reference, &coordinates, &reciprocal).unwrap();
        assert_eq!(path.len(), 3);
        assert!((path[1].crystal[0] - 0.13).abs() < 1e-10);
        let mut bad = coordinates;
        bad[1][1] = 0.2;
        assert!(native_path(&reference, &bad, &reciprocal).is_err());
        assert!(native_path(&reference, &coordinates[..2], &reciprocal).is_err());
    }
    #[test]
    fn phonon_differences_use_only_existing_common_points_and_allow_none() {
        let qe = vec![point(0, 0.), point(1, 0.25), point(2, 1.)];
        let epw = vec![point(0, 0.), point(1, 0.7), point(2, 1.)];
        let q = vec![vec![0.; 3], vec![999.; 3], vec![2.; 3]];
        let e = vec![vec![1. / MEV_CM1; 3], vec![-999.; 3], vec![3. / MEV_CM1; 3]];
        let stats = common_phonon_difference(&qe, &q, &epw, &e, 3).unwrap();
        assert_eq!(stats.points, 2);
        assert_eq!(stats.paired_states, 6);
        assert!((stats.maximum.unwrap().absolute_difference - 1.).abs() < 1e-10);
        let none =
            common_phonon_difference(&qe, &q, &[point(0, 0.123)], &[vec![0.; 3]], 3).unwrap();
        assert_eq!(none.points, 0);
        assert!(none.maximum.is_none());
    }
}
