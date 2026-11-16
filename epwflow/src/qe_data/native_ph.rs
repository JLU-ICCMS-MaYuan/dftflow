//! QE 7.4.1 的 PH XML、扰动基底与复数 REAL(8) 响应势交接。
use super::{
    manifest::{PhResponse, QPoint},
    save,
    xml::{self, Geometry},
};
use anyhow::{ensure, Context, Result};
use std::{fs, path::Path};

pub const FORMAT: &str = "qe7.4.1-ph-xml-c16";

pub fn control(path: &Path, points: &[QPoint]) -> Result<()> {
    let raw = fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&raw)?;
    let n = doc.root_element();
    let count: usize = xml::text(n, "NUMBER_OF_Q_POINTS")?.trim().parse()?;
    ensure!(
        count == points.len(),
        "data_invalid: PH Global control q Numbers do not match"
    );
    let coords = xml::numbers(xml::text(n, "Q-POINT_COORDINATES")?)?;
    ensure!(
        coords.len() == count * 3,
        "data_invalid: PH Global control q List Break"
    );
    for (q, cart) in points.iter().zip(coords.chunks_exact(3)) {
        ensure!(
            q.cartesian
                .iter()
                .zip(cart)
                .all(|(a, b)| (a - b).abs() < 1e-8),
            "data_invalid: PH Global control with dyn0 q The order does not match"
        );
    }
    Ok(())
}

pub fn dyn_modes(path: &Path) -> Result<usize> {
    let raw = fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&raw)?;
    let nat: usize = xml::text(doc.root_element(), "NUMBER_OF_ATOMS")?
        .trim()
        .parse()?;
    ensure!(nat > 0, "data_invalid: PH Zero atoms");
    let freq = doc
        .descendants()
        .find(|n| n.has_tag_name("FREQUENCIES_THZ_CMM1"))
        .context("data_invalid: PH Missing actual frequency")?;
    let nodes = freq
        .children()
        .filter(|n| n.is_element() && n.tag_name().name().starts_with("OMEGA."))
        .count();
    ensure!(nodes == 3 * nat, "data_invalid: PH Models incomplete");
    for m in 1..=3 * nat {
        ensure!(
            xml::numbers(xml::text(freq, &format!("OMEGA.{m}"))?)?.len() == 2,
            "data_invalid: PH Inconsistent frequency format"
        );
    }
    Ok(3 * nat)
}

pub fn patterns(path: &Path, local: usize, modes: usize) -> Result<()> {
    let raw = fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&raw)?;
    let n = doc.root_element();
    let q: usize = xml::text(n, "QPOINT_NUMBER")?.trim().parse()?;
    ensure!(q == local, "data_invalid: PH patterns Internal q Numbers do not match");
    let irreps: usize = xml::text(n, "NUMBER_IRR_REP")?.trim().parse()?;
    ensure!(irreps > 0, "data_invalid: PH Disturbation is zero");
    let mut basis = vec![];
    for irrep in 1..=irreps {
        let rep = n
            .descendants()
            .find(|x| x.has_tag_name(format!("REPRESENTION.{irrep}").as_str()))
            .context("data_invalid: PH Undisturbed sign")?;
        let count: usize = xml::text(rep, "NUMBER_OF_PERTURBATIONS")?.trim().parse()?;
        let children = rep
            .children()
            .filter(|x| x.is_element() && x.tag_name().name().starts_with("PERTURBATION."))
            .count();
        ensure!(
            count > 0 && children == count,
            "data_invalid: PH Disturbing means cut"
        );
        for p in 1..=count {
            let node = rep
                .children()
                .find(|x| x.has_tag_name(format!("PERTURBATION.{p}").as_str()))
                .context("data_invalid: PH Disturbing mode missing")?;
            let v = xml::numbers(xml::text(node, "DISPLACEMENT_PATTERN")?)?;
            ensure!(v.len() == 2 * modes, "data_invalid: PH Disturbing mode dimensions do not match");
            basis.push(v);
        }
    }
    ensure!(basis.len() == modes, "data_invalid: PH Disturbing base incomplete");
    for i in 0..modes {
        for j in 0..=i {
            let re: f64 = basis[i]
                .chunks_exact(2)
                .zip(basis[j].chunks_exact(2))
                .map(|(a, b)| a[0] * b[0] + a[1] * b[1])
                .sum();
            let im: f64 = basis[i]
                .chunks_exact(2)
                .zip(basis[j].chunks_exact(2))
                .map(|(a, b)| a[0] * b[1] - a[1] * b[0])
                .sum();
            ensure!(
                (re - if i == j { 1.0 } else { 0.0 }).abs() < 1e-8 && im.abs() < 1e-8,
                "data_invalid: PH Disturbing the base is not going to the bottom in its entirety"
            );
        }
    }
    Ok(())
}

pub fn verify(response: &PhResponse, q: &QPoint, geometry: &Geometry) -> Result<()> {
    ensure!(
        response.format == FORMAT,
        "data_invalid: PH Not a validated target native format"
    );
    ensure!(
        response.q_id == q.global_index,
        "data_invalid: PH Global q Inconsistencies"
    );
    let raw = fs::read_to_string(&response.dyn_file)?;
    let doc = roxmltree::Document::parse(&raw)?;
    let root = doc.root_element();
    let nat = geometry.atoms.len();
    ensure!(
        response.perturbations == nat * 3,
        "data_invalid: PH The number of perturbation on the manifest does not match"
    );
    ensure!(
        dyn_modes(&response.dyn_file)? == nat * 3,
        "data_invalid: PH with SCF Atoms don't match"
    );
    let first = root
        .children()
        .find(|n| n.has_tag_name("DYNAMICAL_MAT_.1"))
        .context("data_invalid: PH Motivated Matrix")?;
    let cart = xml::vector(xml::text(first, "Q_POINT")?)?;
    ensure!(
        cart.iter()
            .zip(q.cartesian)
            .all(|(a, b)| (a - b).abs() < 1e-8),
        "data_invalid: PH Dynamic Matrix Physics q Inconsistencies"
    );
    let at = xml::numbers(xml::text(root, "AT")?)?;
    ensure!(
        at.len() == 9
            && at
                .iter()
                .zip(geometry.lattice.iter().flatten())
                .all(|(a, b)| (a - b / geometry.alat).abs() < 1e-8),
        "data_invalid: PH Crystal and SCF Inconsistencies"
    );
    for i in 1..=nat {
        let atom = root
            .descendants()
            .find(|n| n.has_tag_name(format!("ATOM.{i}").as_str()))
            .context("data_invalid: PH Interrupt Atom List")?;
        let tau = xml::vector(
            atom.attribute("TAU")
                .context("data_invalid: PH Atoms without coordinates")?,
        )?;
        ensure!(
            tau.iter()
                .zip(geometry.atoms[i - 1].1)
                .all(|(a, b)| (a - b / geometry.alat).abs() < 1e-8),
            "data_invalid: PH Atom sequence or coordinates do not match"
        );
        for j in 1..=nat {
            ensure!(
                xml::numbers(xml::text(first, &format!("PHI.{i}.{j}"))?)?.len() == 18,
                "data_invalid: PH Cartesian The power matrix cut"
            );
        }
    }
    patterns(&response.patterns, q.local_index, nat * 3)?;
    let dv = response
        .dvscf
        .as_ref()
        .context("data_invalid: PH Missing dvscf")?;
    save::nonempty(dv)?;
    let expected = geometry
        .fft
        .iter()
        .try_fold(16u64 * (nat * 3) as u64, |p, &n| p.checked_mul(n as u64))
        .context("data_invalid: PH Response size spill")?;
    ensure!(
        fs::metadata(dv)?.len() == expected,
        "data_invalid: PH dvscf Interrupting or FFT /The number of perturbation doesn't match:{}(Yes {} bytes, actual {} Bytes)",
        dv.display(),
        expected,
        fs::metadata(dv)?.len()
    );
    // 逐块检查复数 REAL(8)，不把任意等长文件当作有效响应。
    use std::io::Read;
    let mut reader = std::io::BufReader::new(fs::File::open(dv)?);
    let mut bytes = [0u8; 8];
    for _ in 0..expected / 8 {
        reader.read_exact(&mut bytes)?;
        ensure!(
            f64::from_le_bytes(bytes).is_finite(),
            "data_invalid: PH dvscf Contains non-finite values:{}",
            dv.display()
        );
    }
    Ok(())
}
