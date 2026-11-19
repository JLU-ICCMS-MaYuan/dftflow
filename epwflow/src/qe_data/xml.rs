//! 目标 QE schema 的几何与保存数据验证，坐标约定为 Bohr 和 2π/alat。
use super::{qe_config::QeSettings, save};
use anyhow::{ensure, Context, Result};
use std::path::Path;

pub const BOHR_ANGSTROM: f64 = 0.529177210903;

#[derive(Debug, PartialEq, Eq)]
pub struct FftGrids {
    pub dense: [usize; 3],
    pub smooth: [usize; 3],
}

/// 只使用实际输出，不将输入参数或响应文件字节数当成三维 FFT 身份。
pub fn fft_grids(xml: &Path) -> Result<FftGrids> {
    let raw = std::fs::read_to_string(xml)?;
    let doc = roxmltree::Document::parse(&raw)?;
    let out = doc
        .descendants()
        .find(|n| n.has_tag_name("output"))
        .context("data_invalid: XML Missing output")?;
    let read = |tag: &str| -> Result<[usize; 3]> {
        let node = out
            .descendants()
            .find(|n| n.has_tag_name(tag))
            .with_context(|| format!("data_invalid: {} Missing {tag}", xml.display()))?;
        let mut dims = [0; 3];
        for (i, dim) in dims.iter_mut().enumerate() {
            let name = format!("nr{}", i + 1);
            *dim = node
                .attribute(name.as_str())
                .with_context(|| format!("data_invalid: {} Missing {tag}.{name}", xml.display()))?
                .parse()?;
            ensure!(
                *dim > 0,
                "data_invalid: {} It's {tag}.{name} It has to be right",
                xml.display()
            );
        }
        Ok(dims)
    };
    Ok(FftGrids {
        dense: read("fft_grid")?,
        smooth: read("fft_smooth")?,
    })
}

pub fn verify_fft(scf_xml: &Path, nscf_xml: &Path) -> Result<()> {
    let scf = fft_grids(scf_xml)?;
    let nscf = fft_grids(nscf_xml)?;
    ensure!(scf == nscf,
        "data_invalid: SCF/NSCF FFT Inconsistencies;SCF {}: Density {:?}, Smooth {:?};NSCF {}: Density {:?}, Smooth {:?}.Please check the stages SYSTEM It's nr1/nr2/nr3 with nr1s/nr2s/nr3s ,and PH Source;no continuation of response data",
        scf_xml.display(), scf.dense, scf.smooth, nscf_xml.display(), nscf.dense, nscf.smooth);
    Ok(())
}

pub fn numbers(text: &str) -> Result<Vec<f64>> {
    let values = text
        .split_whitespace()
        .map(|s| s.replace(['d', 'D'], "e").parse::<f64>())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        values.iter().all(|x| x.is_finite()),
        "data_invalid: XML Include non-finite values"
    );
    Ok(values)
}
pub fn vector(text: &str) -> Result<[f64; 3]> {
    numbers(text)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("data_invalid: XML Vector length is not 3"))
}
pub fn text<'a, 'input>(node: roxmltree::Node<'a, 'input>, name: &str) -> Result<&'a str> {
    node.descendants()
        .find(|n| n.has_tag_name(name))
        .and_then(|n| n.text())
        .with_context(|| format!("data_invalid: XML Missing {name}"))
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
            .context("data_invalid: XML Missing output")?;
        let cell = out
            .children()
            .find(|n| n.has_tag_name("atomic_structure"))
            .context("data_invalid: XML Missing Structure")?;
        let alat: f64 = cell
            .attribute("alat")
            .context("data_invalid: XML Missing alat")?
            .parse()?;
        ensure!(
            alat.is_finite() && alat > 0.0,
            "data_invalid: XML alat Illegal"
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
                    "data_invalid: XML It's not compatible"
                );
            }
        }
        let atoms = cell
            .descendants()
            .filter(|n| n.has_tag_name("atom"))
            .map(|n| {
                Ok((
                    n.attribute("name")
                        .context("data_invalid: Atom without name")?
                        .into(),
                    vector(n.text().context("data_invalid: Atoms without coordinates")?)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let nat: usize = cell
            .attribute("nat")
            .context("data_invalid: XML Missing nat")?
            .parse()?;
        ensure!(
            !atoms.is_empty() && atoms.len() == nat,
            "data_invalid: Atoms don't match"
        );
        let fft_node = out
            .descendants()
            .find(|n| n.has_tag_name("fft_grid"))
            .context("data_invalid: Missing FFT Grid")?;
        let mut fft = [0; 3];
        for (i, v) in fft.iter_mut().enumerate() {
            *v = fft_node
                .attribute(format!("nr{}", i + 1).as_str())
                .context("data_invalid: FFT Utility Missing")?
                .parse()?;
            ensure!(*v > 0, "data_invalid: FFT Zero dimensions");
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
                    "data_invalid: QE The output crystal does not match the fixed structure"
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
            "data_invalid: QE The output atoms do not match"
        );
        for (i, (name, position)) in self.atoms.iter().enumerate() {
            ensure!(name == names[i], "data_invalid: QE The output atom sequence does not match");
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
                "data_invalid: QE The output atomic coordinates don't match"
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
    let raw = std::fs::read_to_string(&xml)?;
    let doc = roxmltree::Document::parse(&raw)?;
    let out = doc
        .descendants()
        .find(|n| n.has_tag_name("output"))
        .context("data_invalid: XML Missing output")?;
    for flag in ["lsda", "noncolin", "spinorbit"] {
        ensure!(
            text(out, flag)?.trim() == "false",
            "data_invalid: Current native appliance only validates non-magnetic marker wave functions"
        );
    }
    let nks: usize = text(out, "nks")?.trim().parse()?;
    ensure!(nks > 0, "data_invalid: Save tree electrons as empty");
    if let Some(energy) = scf_energy {
        ensure!(
            text(out, "convergence_achieved")?.trim() == "true",
            "data_invalid: SCF XML Unrecovered"
        );
        let xml_energy = numbers(text(out, "etot")?)?;
        ensure!(
            xml_energy.len() == 1 && (2.0 * xml_energy[0] - energy).abs() < 1e-7,
            "data_invalid: SCF XML It doesn't always match the log"
        );
    }
    let geo = Geometry::read(&xml)?;
    geo.validate(qe)?;
    Ok(geo)
}
