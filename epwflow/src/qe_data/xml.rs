//! 目标 QE schema 的几何与保存数据验证，坐标约定为 Bohr 和 2π/alat。
use super::{qe_config::QeSettings, save};
use anyhow::{ensure, Context, Result};
use std::path::Path;

pub const BOHR_ANGSTROM: f64 = 0.529177210903;

pub fn numbers(text: &str) -> Result<Vec<f64>> {
    let values = text
        .split_whitespace()
        .map(|s| s.replace(['d', 'D'], "e").parse::<f64>())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        values.iter().all(|x| x.is_finite()),
        "data_invalid: XML 包含非有限值"
    );
    Ok(values)
}
pub fn vector(text: &str) -> Result<[f64; 3]> {
    numbers(text)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("data_invalid: XML 向量长度不是 3"))
}
pub fn text<'a, 'input>(node: roxmltree::Node<'a, 'input>, name: &str) -> Result<&'a str> {
    node.descendants()
        .find(|n| n.has_tag_name(name))
        .and_then(|n| n.text())
        .with_context(|| format!("data_invalid: XML 缺少 {name}"))
}

#[derive(Debug, Clone)]
pub struct Geometry {
    pub alat: f64,
    pub lattice: [[f64; 3]; 3],
    pub reciprocal: [[f64; 3]; 3],
    pub atoms: Vec<(String, [f64; 3])>,
    pub fft: [usize; 3],
}
impl Geometry {
    pub fn read(xml: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(xml)?;
        let doc = roxmltree::Document::parse(&raw)?;
        let out = doc
            .descendants()
            .find(|n| n.has_tag_name("output"))
            .context("data_invalid: XML 缺 output")?;
        let cell = out
            .children()
            .find(|n| n.has_tag_name("atomic_structure"))
            .context("data_invalid: XML 缺结构")?;
        let alat: f64 = cell
            .attribute("alat")
            .context("data_invalid: XML 缺 alat")?
            .parse()?;
        ensure!(
            alat.is_finite() && alat > 0.0,
            "data_invalid: XML alat 非法"
        );
        let mut lattice = [[0.0; 3]; 3];
        let mut reciprocal = [[0.0; 3]; 3];
        for i in 0..3 {
            lattice[i] = vector(text(cell, &format!("a{}", i + 1))?)?;
            reciprocal[i] = vector(text(out, &format!("b{}", i + 1))?)?;
        }
        for i in 0..3 {
            for j in 0..3 {
                let dot: f64 = (0..3)
                    .map(|a| lattice[i][a] * reciprocal[j][a] / alat)
                    .sum();
                ensure!(
                    (dot - if i == j { 1.0 } else { 0.0 }).abs() < 1e-8,
                    "data_invalid: XML 正倒格矢不相容"
                );
            }
        }
        let atoms = cell
            .descendants()
            .filter(|n| n.has_tag_name("atom"))
            .map(|n| {
                Ok((
                    n.attribute("name")
                        .context("data_invalid: 原子无名称")?
                        .into(),
                    vector(n.text().context("data_invalid: 原子无坐标")?)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let nat: usize = cell
            .attribute("nat")
            .context("data_invalid: XML 缺 nat")?
            .parse()?;
        ensure!(
            !atoms.is_empty() && atoms.len() == nat,
            "data_invalid: 原子数不符"
        );
        let fft_node = out
            .descendants()
            .find(|n| n.has_tag_name("fft_grid"))
            .context("data_invalid: 缺 FFT 网格")?;
        let mut fft = [0; 3];
        for (i, v) in fft.iter_mut().enumerate() {
            *v = fft_node
                .attribute(format!("nr{}", i + 1).as_str())
                .context("data_invalid: FFT 维度缺失")?
                .parse()?;
            ensure!(*v > 0, "data_invalid: FFT 维度为零");
        }
        Ok(Self {
            alat,
            lattice,
            reciprocal,
            atoms,
            fft,
        })
    }
    pub fn crystal(&self, cartesian: [f64; 3]) -> [f64; 3] {
        std::array::from_fn(|i| {
            (0..3)
                .map(|j| cartesian[j] * self.lattice[i][j] / self.alat)
                .sum()
        })
    }
    pub fn validate(&self, qe: &QeSettings) -> Result<()> {
        for i in 0..3 {
            for j in 0..3 {
                ensure!(
                    (self.lattice[i][j] * BOHR_ANGSTROM - qe.cell.lattice[i][j]).abs() < 1e-7,
                    "data_invalid: QE 输出晶格与固定结构不符"
                );
            }
        }
        let names = qe
            .cell
            .elements
            .iter()
            .zip(&qe.cell.counts)
            .flat_map(|(el, n)| std::iter::repeat(el).take(*n))
            .collect::<Vec<_>>();
        ensure!(
            self.atoms.len() == names.len(),
            "data_invalid: QE 输出原子数不符"
        );
        for (i, (name, position)) in self.atoms.iter().enumerate() {
            ensure!(name == names[i], "data_invalid: QE 输出原子顺序不符");
            let expected = if qe.cell.coord_type.to_lowercase().starts_with('d') {
                std::array::from_fn(|a| {
                    (0..3)
                        .map(|b| qe.cell.positions[i][b] * qe.cell.lattice[b][a])
                        .sum()
                })
            } else {
                qe.cell.positions[i]
            };
            let fractional: [f64; 3] = std::array::from_fn(|a| {
                (0..3)
                    .map(|b| {
                        (position[b] - expected[b] / BOHR_ANGSTROM) * self.reciprocal[a][b]
                            / self.alat
                    })
                    .sum()
            });
            ensure!(
                fractional.iter().all(|x| (x - x.round()).abs() < 1e-7),
                "data_invalid: QE 输出原子坐标不符"
            );
        }
        Ok(())
    }
}

/// 限定当前已验证的非磁性 QE 7.4.1 portable 保存布局。
pub fn validate_save(
    outdir: &Path,
    prefix: &str,
    qe: &QeSettings,
    scf_energy: Option<f64>,
) -> Result<Geometry> {
    let dir = outdir.join(format!("{prefix}.save"));
    let xml = dir.join("data-file-schema.xml");
    save::nonempty(&xml)?;
    save::nonempty(&dir.join("charge-density.dat"))?;
    let raw = std::fs::read_to_string(&xml)?;
    let doc = roxmltree::Document::parse(&raw)?;
    let out = doc
        .descendants()
        .find(|n| n.has_tag_name("output"))
        .context("data_invalid: XML 缺 output")?;
    for flag in ["lsda", "noncolin", "spinorbit"] {
        ensure!(
            text(out, flag)?.trim() == "false",
            "data_invalid: 当前原生适配仅验证非磁性标量波函数"
        );
    }
    let nks: usize = text(out, "nks")?.trim().parse()?;
    ensure!(nks > 0, "data_invalid: 保存树电子态为空");
    for k in 1..=nks {
        save::nonempty(&dir.join(format!("wfc{k}.dat")))?;
    }
    if let Some(energy) = scf_energy {
        ensure!(
            text(out, "convergence_achieved")?.trim() == "true",
            "data_invalid: SCF XML 未收敛"
        );
        let xml_energy = numbers(text(out, "etot")?)?;
        ensure!(
            xml_energy.len() == 1 && (2.0 * xml_energy[0] - energy).abs() < 1e-7,
            "data_invalid: SCF XML 与日志总能不一致"
        );
    }
    let geo = Geometry::read(&xml)?;
    geo.validate(qe)?;
    Ok(geo)
}
