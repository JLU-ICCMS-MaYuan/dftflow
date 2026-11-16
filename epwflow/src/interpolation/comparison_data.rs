//! F3 唯一拥有的成对数据契约、校验与单位约束。
use crate::foundation::store;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

pub const TABLES: [&str; 4] = [
    "electronic_bands.csv",
    "phonon_bands.csv",
    "electronic_dos.csv",
    "phonon_dos.csv",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub relative_path: PathBuf,
    pub byte_size: u64,
    pub sha256: String,
}

impl Artifact {
    pub fn capture(root: &Path, name: &str) -> Result<Self> {
        let path = root.join(name);
        Ok(Self {
            relative_path: name.into(),
            byte_size: fs::metadata(&path)?.len(),
            sha256: store::hash_file(&path)?,
        })
    }
    pub fn verify(&self, root: &Path) -> Result<PathBuf> {
        ensure!(
            self.relative_path
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
            "data_invalid: manifest routes crossed the border"
        );
        let path = root.join(&self.relative_path);
        let canonical_root = root.canonicalize()?;
        ensure!(
            path.canonicalize()?.starts_with(canonical_root),
            "data_invalid: Data links across borders"
        );
        ensure!(
            fs::metadata(&path)?.len() == self.byte_size && store::hash_file(&path)? == self.sha256,
            "data_invalid: File prints don't match {}",
            path.display()
        );
        Ok(path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub run_id: String,
    pub ph_mode: String,
    pub upstream_digest: String,
    /// 仅用于历史 v1 包读取；新包不写入。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_status: Option<String>,
    pub missing: Vec<String>,
    pub files: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tick {
    pub distance: f64,
    pub label: String,
}

/// 仅用于版本 1 历史图包重绘的直接 PH 标注；新包不生成此数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhononErrorMarker {
    pub scope: String,
    pub q_crystal: [f64; 3],
    pub mode_sorted: usize,
    pub qe_cm1: f64,
    pub epw_cm1: f64,
    pub error_cm1: f64,
    pub path_distance: Option<f64>,
    pub path_label: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub title: String,
    pub fermi_ev: f64,
    pub window_ev: [f64; 2],
    /// 来自已核验 .wout 的绝对 eV 外窗；旧包没有该字段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disentanglement_window_ev: Option<[f64; 2]>,
    /// 同一 .wout 的实际冻结窗；绝对 eV，缺失时不推断。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frozen_window_ev: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub electronic_independent_sampling: bool,
    pub electronic_ticks: Vec<Tick>,
    pub phonon_ticks: Vec<Tick>,
    pub electronic_energy_unit: String,
    pub phonon_frequency_unit: String,
    pub electronic_density_unit: String,
    pub phonon_density_unit: String,
    pub elements: Vec<String>,
    pub orbitals: Vec<String>,
    pub electronic_projection: String,
    pub phonon_projection: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phonon_error_markers: Vec<PhononErrorMarker>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Curve {
    pub method: String,
    pub projection: String,
    pub segment: String,
    pub branch: String,
    pub points: Vec<(f64, f64)>,
}

pub struct Dataset {
    pub manifest: Manifest,
    pub metadata: Metadata,
    pub electronic: Vec<Curve>,
    pub phonon: Vec<Curve>,
    pub electronic_dos: Vec<Curve>,
    pub phonon_dos: Vec<Curve>,
    pub direct: Vec<(f64, f64)>,
    pub differences: Option<super::differences::Differences>,
}

// 该数值交换格式只接受无引号 CSV；复杂标签放 metadata，而不是静默拆错列。
fn table(path: &Path, header: &str) -> Result<Vec<Vec<String>>> {
    let text = fs::read_to_string(path)?;
    let mut lines = text.lines();
    ensure!(
        lines.next() == Some(header),
        "data_invalid: CSV Columns do not match {}",
        path.display()
    );
    let columns = header.split(',').count();
    lines
        .filter(|l| !l.is_empty())
        .map(|l| {
            ensure!(!l.contains('"'), "data_invalid: Value CSV Quoted fields are not supported");
            let row: Vec<String> = l.split(',').map(str::to_owned).collect();
            ensure!(row.len() == columns, "data_invalid: CSV Fields do not match");
            Ok(row)
        })
        .collect()
}

fn number(text: &str) -> Result<f64> {
    let n: f64 = text.parse().context("data_invalid: Numeric field invalid")?;
    ensure!(n.is_finite(), "data_invalid: Non-finite values");
    Ok(n)
}

fn method(s: &str) -> Result<()> {
    ensure!(matches!(s, "qe" | "epw"), "data_invalid: Unknown method of comparison");
    Ok(())
}

fn bands(root: &Path, phonon: bool) -> Result<Vec<Curve>> {
    let name = if phonon { TABLES[1] } else { TABLES[0] };
    let header = if phonon {
        "method,reference_kind,segment_id,point_id,distance,mode,frequency_cm1"
    } else {
        "method,segment_id,point_id,distance,band,energy_ev"
    };
    let rows = table(&root.join(name), header)?;
    let mut curves: BTreeMap<(String, String, String), Vec<(f64, f64)>> = BTreeMap::new();
    let mut keys = BTreeSet::new();
    for row in rows {
        method(&row[0])?;
        let offset = usize::from(phonon);
        if phonon {
            ensure!(
                row[1] == if row[0] == "qe" { "qe_matdyn" } else { "epw" },
                "data_invalid: phonon references cannot be assumed to be direct PH"
            );
        }
        let seg = &row[1 + offset];
        let id = &row[2 + offset];
        let branch = &row[4 + offset];
        ensure!(
            keys.insert((row[0].clone(), seg.clone(), id.clone(), branch.clone())),
            "data_invalid: Repeat Path Record"
        );
        curves
            .entry((row[0].clone(), seg.clone(), branch.clone()))
            .or_default()
            .push((number(&row[3 + offset])?, number(&row[5 + offset])?));
    }
    let out: Vec<Curve> = curves
        .into_iter()
        .map(|((method, segment, branch), points)| Curve {
            method,
            segment,
            branch,
            projection: "total".into(),
            points,
        })
        .collect();
    paired_axes(&out, false)?;
    Ok(out)
}

fn dos(root: &Path, phonon: bool, metadata: &Metadata) -> Result<Vec<Curve>> {
    let header = if phonon {
        "method,frequency_cm1,projection_kind,element,density"
    } else {
        "method,energy_ev,projection_kind,element,orbital,density"
    };
    let mut curves: BTreeMap<(String, String), Vec<(f64, f64)>> = BTreeMap::new();
    for row in table(
        &root.join(if phonon { TABLES[3] } else { TABLES[2] }),
        header,
    )? {
        method(&row[0])?;
        let kind = row[2].as_str();
        ensure!(
            matches!(kind, "total" | "element" | "orbital") && !(phonon && kind == "orbital"),
            "data_invalid: Projection category invalid"
        );
        let projection = match kind {
            "total" => {
                ensure!(
                    row[3].is_empty() && (phonon || row[4].is_empty()),
                    "data_invalid: Total DOS No Element Label"
                );
                "total".to_string()
            }
            "element" => {
                ensure!(
                    metadata.elements.contains(&row[3]) && (phonon || row[4].is_empty()),
                    "data_invalid: The element projection label does not match"
                );
                row[3].clone()
            }
            _ => {
                let key = format!("{}:{}", row[3], row[4]);
                ensure!(
                    metadata.orbitals.contains(&key),
                    "data_invalid: Orbital tags don't match"
                );
                key
            }
        };
        if kind != "total" {
            ensure!(
                if phonon {
                    metadata.phonon_projection == "mass_weighted_eigenvectors"
                } else {
                    metadata.electronic_projection == "atomic_projector_operator"
                },
                "data_invalid: There is no definition of a reliable projection, which cannot be marked as an element or orbit DOS"
            );
        }
        let density = number(row.last().unwrap())?;
        ensure!(density >= -1e-12, "data_invalid: DOS Negative");
        curves
            .entry((row[0].clone(), projection))
            .or_default()
            .push((number(&row[1])?, density));
    }
    let out: Vec<Curve> = curves
        .into_iter()
        .map(|((method, projection), points)| Curve {
            method,
            projection,
            segment: String::new(),
            branch: String::new(),
            points,
        })
        .collect();
    paired_axes(&out, true)?;
    Ok(out)
}

fn paired_axes(curves: &[Curve], is_dos: bool) -> Result<()> {
    for curve in curves {
        ensure!(
            curve.points.len() >= 2 && curve.points.windows(2).all(|w| w[0].0 < w[1].0),
            "data_invalid: Path or DOS Grids are not strictly incremental"
        );
        let partner = curves.iter().find(|p| {
            p.method != curve.method
                && p.projection == curve.projection
                && p.segment == curve.segment
                && (!is_dos || p.branch == curve.branch)
        });
        let partner = partner.context("data_invalid: Loss of pairing method or projection")?;
        // 曲线各自保留原始采样，只核对同一物理段的起终位置。
        ensure!(
            is_dos || ((curve.points[0].0 - partner.points[0].0).abs() < 1e-8
                && (curve.points.last().unwrap().0 - partner.points.last().unwrap().0).abs() < 1e-8),
            "data_invalid: Inconsistent physical path endpoints on both sides"
        );
    }
    Ok(())
}

pub fn load(root: &Path) -> Result<Dataset> {
    let manifest: Manifest = store::read(&root.join("manifest.json"))?;
    ensure!(
        matches!(manifest.schema_version, 1 | 2)
            && matches!(manifest.ph_mode.as_str(), "index" | "coords" | "nosplit")
            && manifest.upstream_digest.len() == 64,
        "data_invalid: Invalid comparison of manifest version or status"
    );
    let names: BTreeSet<_> = manifest
        .files
        .iter()
        .map(|a| a.relative_path.clone())
        .collect();
    let mut expected: BTreeSet<PathBuf> = TABLES
        .iter()
        .chain(["metadata.json", "metrics.json"].iter())
        .map(PathBuf::from)
        .collect();
    if manifest.schema_version == 1 {
        expected.insert("phonon_reference_points.csv".into());
    } else {
        ensure!(
            manifest.quality_status.is_none(),
            "data_invalid: The new package does not allow quality ratings"
        );
    }
    ensure!(
        names == expected && names.len() == manifest.files.len(),
        "data_invalid: Compare package file collection incomplete"
    );
    for f in &manifest.files {
        f.verify(root)?;
    }
    let metrics: serde_json::Value = store::read(&root.join("metrics.json"))?;
    let differences: Option<super::differences::Differences> = if manifest.schema_version == 2 {
        ensure!(
            metrics["schema_version"] == 2
                && metrics["electronic_matching"] == "ordered_minimum_squared_energy_pairs",
            "data_invalid: Maximum margin version or pair definition inconsistent"
        );
        let stats: super::differences::Differences = serde_json::from_value(
            metrics
                .get("differences")
                .context("data_invalid: Missing four maximum margin values")?
                .clone(),
        )?;
        stats.validate()?;
        Some(stats)
    } else {
        None
    };
    let metadata: Metadata = store::read(&root.join("metadata.json"))?;
    ensure!(
        manifest.schema_version == 1 || metadata.phonon_error_markers.is_empty(),
        "data_invalid: The new package does not allow sampling"
    );
    if let Some(window) = metadata.disentanglement_window_ev {
        ensure!(
            window.iter().all(|v| v.is_finite())
                && window[0] < window[1]
                && window
                    .iter()
                    .zip(metadata.window_ev)
                    .all(
                        |(absolute, relative)| (absolute - metadata.fermi_ev - relative).abs()
                            < 1e-8
                    ),
            "data_invalid: Wannier Outside and common Fermi Can/Show Windows Unmatched"
        );
    }
    if let Some(frozen) = metadata.frozen_window_ev {
        let outer = metadata
            .disentanglement_window_ev
            .context("data_invalid: Frozen windows lack the corresponding actual windows")?;
        ensure!(
            frozen.iter().all(|v| v.is_finite())
                && frozen[0] < frozen[1]
                && frozen[0] >= outer[0]
                && frozen[1] <= outer[1],
            "data_invalid: Freezing windows must be finite, incremental and included in the outer window"
        );
    }
    ensure!(
        metadata.fermi_ev.is_finite()
            && metadata.window_ev.iter().all(|v| v.is_finite())
            && metadata.window_ev[0] < metadata.window_ev[1]
            && metadata.electronic_energy_unit == "eV_relative_fermi"
            && metadata.phonon_frequency_unit == "cm-1"
            && metadata.electronic_density_unit == "states/(eV*cell)"
            && metadata.phonon_density_unit == "states/(cm-1*cell)",
        "data_invalid: Units/commons do not match"
    );
    for ticks in [&metadata.electronic_ticks, &metadata.phonon_ticks] {
        ensure!(
            ticks.iter().all(|t| t.distance.is_finite())
                && ticks.windows(2).all(|w| w[0].distance < w[1].distance),
            "data_invalid: Path mark invalid"
        );
    }
    let electronic = bands(root, false)?;
    let phonon = bands(root, true)?;
    let mut scopes = BTreeSet::new();
    let mode_count = phonon
        .iter()
        .map(|c| &c.branch)
        .collect::<BTreeSet<_>>()
        .len();
    for marker in &metadata.phonon_error_markers {
        ensure!(
            matches!(marker.scope.as_str(), "sampled_maximum" | "path_maximum")
                && scopes.insert(&marker.scope)
                && marker.q_crystal.iter().all(|v| v.is_finite())
                && marker.mode_sorted > 0
                && marker.mode_sorted <= mode_count
                && [marker.qe_cm1, marker.epw_cm1, marker.error_cm1]
                    .iter()
                    .all(|v| v.is_finite())
                && marker.error_cm1 >= 0.0
                && ((marker.qe_cm1 - marker.epw_cm1).abs() - marker.error_cm1).abs() < 1e-8
                && marker.path_distance.is_some() == marker.path_label.is_some()
                && (marker.scope != "path_maximum" || marker.path_distance.is_some()),
            "data_invalid: Invalid mode, value or path identity for voice error"
        );
        if let Some(x) = marker.path_distance {
            ensure!(
                x.is_finite()
                    && phonon
                        .iter()
                        .any(|c| c.points.iter().any(|p| (p.0 - x).abs() < 1e-8)),
                "data_invalid: The sound error label is not on the path sample point"
            );
        }
    }
    let electronic_dos = dos(root, false, &metadata)?;
    if metadata.disentanglement_window_ev.is_some() {
        ensure!(
            electronic_dos.iter().all(|curve| curve
                .points
                .first()
                .is_some_and(|p| p.0 <= metadata.window_ev[0] + 1e-8)
                && curve
                    .points
                    .last()
                    .is_some_and(|p| p.0 >= metadata.window_ev[1] - 1e-8)),
            "data_invalid: Electronic DOS Actual data not covered Wannier Outside windows"
        );
    }
    let phonon_dos = dos(root, true, &metadata)?;
    let mut direct = Vec::new();
    if manifest.schema_version == 1 {
        for row in table(
            &root.join("phonon_reference_points.csv"),
            "point_id,distance,mode,frequency_cm1,source",
        )? {
            ensure!(!row[4].is_empty(), "data_invalid: Direct PH Source Missing");
            direct.push((number(&row[1])?, number(&row[3])?));
        }
    }
    let data = Dataset {
        manifest,
        metadata,
        electronic,
        phonon,
        electronic_dos,
        phonon_dos,
        direct,
        differences,
    };
    // 缺项必须显式登记；一个“完整”标签不能代替两侧投影数据。
    if data.manifest.missing.is_empty() {
        ensure!(
            !data.electronic.is_empty()
                && !data.phonon.is_empty()
                && !data.electronic_dos.is_empty()
                && !data.phonon_dos.is_empty(),
            "data_invalid: Full package missing curve"
        );
        for name in data.metadata.elements.iter().chain(&data.metadata.orbitals) {
            ensure!(
                data.electronic_dos.iter().any(|c| &c.projection == name),
                "data_invalid: The complete package lacks an electronic projection"
            );
        }
        for name in &data.metadata.elements {
            ensure!(
                data.phonon_dos.iter().any(|c| &c.projection == name),
                "data_invalid: Full package missing element sound DOS"
            );
        }
    }
    Ok(data)
}
