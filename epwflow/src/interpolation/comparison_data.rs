//! F3 唯一拥有的成对数据契约、校验与单位约束。
use crate::foundation::store;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

pub const TABLES: [&str; 5] = [
    "electronic_bands.csv",
    "phonon_bands.csv",
    "phonon_reference_points.csv",
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
            "data_invalid: 清单路径越界"
        );
        let path = root.join(&self.relative_path);
        let canonical_root = root.canonicalize()?;
        ensure!(
            path.canonicalize()?.starts_with(canonical_root),
            "data_invalid: 数据链接越界"
        );
        ensure!(
            fs::metadata(&path)?.len() == self.byte_size && store::hash_file(&path)? == self.sha256,
            "data_invalid: 文件指纹不符 {}",
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
    pub quality_status: String,
    pub missing: Vec<String>,
    pub files: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tick {
    pub distance: f64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub title: String,
    pub fermi_ev: f64,
    pub window_ev: [f64; 2],
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
}

// 该数值交换格式只接受无引号 CSV；复杂标签放 metadata，而不是静默拆错列。
fn table(path: &Path, header: &str) -> Result<Vec<Vec<String>>> {
    let text = fs::read_to_string(path)?;
    let mut lines = text.lines();
    ensure!(
        lines.next() == Some(header),
        "data_invalid: CSV 列不符 {}",
        path.display()
    );
    let columns = header.split(',').count();
    lines
        .filter(|l| !l.is_empty())
        .map(|l| {
            ensure!(!l.contains('"'), "data_invalid: 数值 CSV 不支持带引号字段");
            let row: Vec<String> = l.split(',').map(str::to_owned).collect();
            ensure!(row.len() == columns, "data_invalid: CSV 字段数不符");
            Ok(row)
        })
        .collect()
}

fn number(text: &str) -> Result<f64> {
    let n: f64 = text.parse().context("data_invalid: 数值字段无效")?;
    ensure!(n.is_finite(), "data_invalid: 非有限数值");
    Ok(n)
}

fn method(s: &str) -> Result<()> {
    ensure!(matches!(s, "qe" | "epw"), "data_invalid: 未知比较方法");
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
                "data_invalid: 连续声子参考不能冒充直接 PH"
            );
        }
        let seg = &row[1 + offset];
        let id = &row[2 + offset];
        let branch = &row[4 + offset];
        ensure!(
            keys.insert((row[0].clone(), seg.clone(), id.clone(), branch.clone())),
            "data_invalid: 重复路径记录"
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
    paired(&out, false)?;
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
        &root.join(if phonon { TABLES[4] } else { TABLES[3] }),
        header,
    )? {
        method(&row[0])?;
        let kind = row[2].as_str();
        ensure!(
            matches!(kind, "total" | "element" | "orbital") && !(phonon && kind == "orbital"),
            "data_invalid: 投影类别无效"
        );
        let projection = match kind {
            "total" => {
                ensure!(
                    row[3].is_empty() && (phonon || row[4].is_empty()),
                    "data_invalid: 总 DOS 不应带元素标签"
                );
                "total".to_string()
            }
            "element" => {
                ensure!(
                    metadata.elements.contains(&row[3]) && (phonon || row[4].is_empty()),
                    "data_invalid: 元素投影标签不符"
                );
                row[3].clone()
            }
            _ => {
                let key = format!("{}:{}", row[3], row[4]);
                ensure!(
                    metadata.orbitals.contains(&key),
                    "data_invalid: 轨道标签不符"
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
                "data_invalid: 没有可靠投影定义，不能标为元素或轨道 DOS"
            );
        }
        let density = number(row.last().unwrap())?;
        ensure!(density >= -1e-12, "data_invalid: DOS 为负");
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
    paired(&out, true)?;
    Ok(out)
}

fn paired(curves: &[Curve], is_dos: bool) -> Result<()> {
    for curve in curves {
        ensure!(
            curve.points.len() >= 2 && curve.points.windows(2).all(|w| w[0].0 < w[1].0),
            "data_invalid: 路径或 DOS 网格不严格递增"
        );
        let partner = curves.iter().find(|p| {
            p.method != curve.method
                && p.projection == curve.projection
                && p.segment == curve.segment
                && (!is_dos || p.branch == curve.branch)
        });
        let partner = partner.context("data_invalid: 成对方法或投影缺失")?;
        ensure!(
            curve.points.len() == partner.points.len()
                && curve
                    .points
                    .iter()
                    .zip(&partner.points)
                    .all(|(a, b)| (a.0 - b.0).abs() < 1e-10),
            "data_invalid: 两侧路径／DOS 轴不一致"
        );
    }
    Ok(())
}

pub fn load(root: &Path) -> Result<Dataset> {
    let manifest: Manifest = store::read(&root.join("manifest.json"))?;
    ensure!(
        manifest.schema_version == 1
            && matches!(manifest.ph_mode.as_str(), "index" | "coords" | "nosplit")
            && matches!(
                manifest.quality_status.as_str(),
                "passed" | "failed" | "unverified"
            )
            && manifest.upstream_digest.len() == 64,
        "data_invalid: 比较清单版本或状态无效"
    );
    let names: BTreeSet<_> = manifest
        .files
        .iter()
        .map(|a| a.relative_path.clone())
        .collect();
    let expected: BTreeSet<PathBuf> = TABLES
        .iter()
        .chain(["metadata.json", "metrics.json"].iter())
        .map(PathBuf::from)
        .collect();
    ensure!(
        names == expected && names.len() == manifest.files.len(),
        "data_invalid: 比较包文件集合不完整"
    );
    for f in &manifest.files {
        f.verify(root)?;
    }
    let _: serde_json::Value = store::read(&root.join("metrics.json"))?;
    let metadata: Metadata = store::read(&root.join("metadata.json"))?;
    ensure!(
        metadata.fermi_ev.is_finite()
            && metadata.window_ev.iter().all(|v| v.is_finite())
            && metadata.window_ev[0] <= -0.5
            && metadata.window_ev[1] >= 0.5
            && metadata.electronic_energy_unit == "eV_relative_fermi"
            && metadata.phonon_frequency_unit == "cm-1"
            && metadata.electronic_density_unit == "states/(eV*cell)"
            && metadata.phonon_density_unit == "states/(cm-1*cell)",
        "data_invalid: 单位／共同能窗不符"
    );
    for ticks in [&metadata.electronic_ticks, &metadata.phonon_ticks] {
        ensure!(
            ticks.iter().all(|t| t.distance.is_finite())
                && ticks.windows(2).all(|w| w[0].distance < w[1].distance),
            "data_invalid: 路径标记无效"
        );
    }
    let electronic = bands(root, false)?;
    let phonon = bands(root, true)?;
    let electronic_dos = dos(root, false, &metadata)?;
    let phonon_dos = dos(root, true, &metadata)?;
    let mut direct = Vec::new();
    for row in table(
        &root.join(TABLES[2]),
        "point_id,distance,mode,frequency_cm1,source",
    )? {
        ensure!(!row[4].is_empty(), "data_invalid: 直接 PH 来源缺失");
        direct.push((number(&row[1])?, number(&row[3])?));
    }
    let data = Dataset {
        manifest,
        metadata,
        electronic,
        phonon,
        electronic_dos,
        phonon_dos,
        direct,
    };
    // 缺项必须显式登记；一个“完整”标签不能代替两侧投影数据。
    if data.manifest.missing.is_empty() {
        ensure!(
            !data.electronic.is_empty()
                && !data.phonon.is_empty()
                && !data.electronic_dos.is_empty()
                && !data.phonon_dos.is_empty(),
            "data_invalid: 完整包缺少曲线"
        );
        for name in data.metadata.elements.iter().chain(&data.metadata.orbitals) {
            ensure!(
                data.electronic_dos.iter().any(|c| &c.projection == name),
                "data_invalid: 完整包缺少电子投影"
            );
        }
        for name in &data.metadata.elements {
            ensure!(
                data.phonon_dos.iter().any(|c| &c.projection == name),
                "data_invalid: 完整包缺少元素声子 DOS"
            );
        }
    }
    Ok(data)
}
