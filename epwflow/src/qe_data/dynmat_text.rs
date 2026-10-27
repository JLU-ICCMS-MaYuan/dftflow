//! QE 旧文本动力学矩阵到 io_dyn_mat XML；复矩阵按 Fortran 列序输出。
use super::xml::{self, Geometry};
use anyhow::{ensure, Context, Result};
use std::fmt::Write;

fn nums(line: &str) -> Result<Vec<f64>> {
    xml::numbers(&line.replace(['(', ')', ','], " "))
}
fn tag(out: &mut String, name: &str, values: &[f64]) {
    writeln!(out, "<{name}>").unwrap();
    if name.starts_with("PHI.") || name.starts_with("DISPLACEMENT.") {
        for pair in values.chunks_exact(2) {
            writeln!(out, "{:.16e} {:.16e}", pair[0], pair[1]).unwrap();
        }
    } else {
        writeln!(
            out,
            "{}",
            values
                .iter()
                .map(|x| format!("{x:.16e}"))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .unwrap();
    }
    writeln!(out, "</{name}>").unwrap();
}
fn transpose_complex(rows: &[f64]) -> Vec<f64> {
    (0..3)
        .flat_map(|j| (0..3).flat_map(move |i| [rows[6 * i + 2 * j], rows[6 * i + 2 * j + 1]]))
        .collect()
}
fn tensor(lines: &[&str], cursor: &mut usize, rows: bool) -> Result<Vec<f64>> {
    let mut data = vec![];
    while lines.get(*cursor).is_some_and(|s| s.trim().is_empty()) {
        *cursor += 1;
    }
    for _ in 0..3 {
        let row = nums(lines.get(*cursor).context("data_invalid: 张量截断")?)?;
        ensure!(row.len() == 3, "data_invalid: 张量维数错误");
        data.extend(row);
        *cursor += 1;
    }
    Ok(if rows {
        (0..3)
            .flat_map(|j| (0..3).map(move |i| (i, j)))
            .map(|(i, j)| data[3 * i + j])
            .collect()
    } else {
        data
    })
}

fn dielectric(lines: &[&str], nat: usize) -> Result<String> {
    let eps = lines.iter().position(|s| s.contains("Dielectric Tensor"));
    let born = lines.iter().position(|s| s.contains("Effective Charges"));
    let raman = lines.iter().position(|s| s.contains("Raman tensor (A^2)"));
    ensure!(
        born.is_none() || eps.is_some(),
        "data_invalid: Born 电荷缺介电张量"
    );
    let mut out = format!(
        "<DIELECTRIC_PROPERTIES epsil=\"{}\" zstar=\"{}\" raman=\"{}\">\n",
        eps.is_some(),
        born.is_some(),
        raman.is_some()
    );
    if let Some(p) = eps {
        let mut cursor = p + 1;
        tag(&mut out, "EPSILON", &tensor(lines, &mut cursor, true)?);
    }
    if let Some(p) = born {
        let mut cursor = p + 1;
        out.push_str("<ZSTAR>\n");
        for a in 1..=nat {
            while lines.get(cursor).is_some_and(|s| s.trim().is_empty()) {
                cursor += 1;
            }
            let line = lines
                .get(cursor)
                .context("data_invalid: Born 电荷原子截断")?;
            ensure!(
                line.split_once('#')
                    .context("data_invalid: Born 原子头错误")?
                    .1
                    .trim()
                    .parse::<usize>()?
                    == a,
                "data_invalid: Born 原子错序"
            );
            cursor += 1;
            tag(
                &mut out,
                &format!("Z_AT_.{a}"),
                &tensor(lines, &mut cursor, true)?,
            );
        }
        out.push_str("</ZSTAR>\n");
    }
    if let Some(p) = raman {
        ensure!(eps.is_some(), "data_invalid: Raman 缺介电张量");
        let mut cursor = p + 1;
        out.push_str("<RAMAN_TENSOR_A2>\n");
        for a in 1..=nat {
            for k in 1..=3 {
                while lines.get(cursor).is_some_and(|s| s.trim().is_empty()) {
                    cursor += 1;
                }
                let head = lines
                    .get(cursor)
                    .context("data_invalid: Raman 头截断")?
                    .split_once('#')
                    .context("data_invalid: Raman 原子头错误")?
                    .1;
                let (atom, pol) = head
                    .split_once("pol.")
                    .context("data_invalid: Raman 极化头错误")?;
                ensure!(
                    atom.trim().parse::<usize>()? == a && pol.trim().parse::<usize>()? == k,
                    "data_invalid: Raman 原子/极化错序"
                );
                cursor += 1;
                tag(
                    &mut out,
                    &format!("RAMAN_S_ALPHA.{a}.{k}"),
                    &tensor(lines, &mut cursor, false)?,
                );
            }
        }
        out.push_str("</RAMAN_TENSOR_A2>\n");
    }
    out.push_str("</DIELECTRIC_PROPERTIES>\n");
    Ok(out)
}

/// 与 QE Modules/latgen.f90 的晶轴约定一致，返回以 celldm(1) 为单位的基矢。
fn bravais(n: i32, c: &[f64]) -> Result<Vec<f64>> {
    let (b, z, u, v, w) = (c[1], c[2], c[3], c[4], c[5]);
    let matrix = match n {
        1 => [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        2 => [[-0.5, 0., 0.5], [0., 0.5, 0.5], [-0.5, 0.5, 0.]],
        3 => [[0.5, 0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, -0.5, 0.5]],
        -3 => [[-0.5, 0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, -0.5]],
        4 => [[1., 0., 0.], [-0.5, 3f64.sqrt() / 2., 0.], [0., 0., z]],
        5 => {
            let x = (1. - u).sqrt() / 2f64.sqrt();
            let y = -x / 3f64.sqrt();
            let t = (1. + 2. * u).sqrt() / 3f64.sqrt();
            [[x, y, t], [0., -2. * y, t], [-x, y, t]]
        }
        -5 => {
            let a = ((1. + 2. * u).sqrt() - 2. * (1. - u).sqrt()) / 3.;
            let b = ((1. + 2. * u).sqrt() + (1. - u).sqrt()) / 3.;
            [[a, b, b], [b, a, b], [b, b, a]]
        }
        6 => [[1., 0., 0.], [0., 1., 0.], [0., 0., z]],
        7 => [
            [0.5, -0.5, z / 2.],
            [0.5, 0.5, z / 2.],
            [-0.5, -0.5, z / 2.],
        ],
        8 => [[1., 0., 0.], [0., b, 0.], [0., 0., z]],
        9 => [[0.5, b / 2., 0.], [-0.5, b / 2., 0.], [0., 0., z]],
        -9 => [[0.5, -b / 2., 0.], [0.5, b / 2., 0.], [0., 0., z]],
        91 => [[1., 0., 0.], [0., b / 2., -z / 2.], [0., b / 2., z / 2.]],
        10 => [[0.5, 0., z / 2.], [0.5, b / 2., 0.], [0., b / 2., z / 2.]],
        11 => [
            [0.5, b / 2., z / 2.],
            [-0.5, b / 2., z / 2.],
            [-0.5, -b / 2., z / 2.],
        ],
        12 => [
            [1., 0., 0.],
            [b * u, b * (1. - u * u).sqrt(), 0.],
            [0., 0., z],
        ],
        -12 => [
            [1., 0., 0.],
            [0., b, 0.],
            [z * v, 0., z * (1. - v * v).sqrt()],
        ],
        13 => [
            [0.5, 0., -z / 2.],
            [b * u, b * (1. - u * u).sqrt(), 0.],
            [0.5, 0., z / 2.],
        ],
        -13 => [
            [0.5, b / 2., 0.],
            [-0.5, b / 2., 0.],
            [z * v, 0., z * (1. - v * v).sqrt()],
        ],
        14 => {
            let sin = (1. - w * w).sqrt();
            [
                [1., 0., 0.],
                [b * w, b * sin, 0.],
                [
                    z * v,
                    z * (u - v * w) / sin,
                    z * ((1. + 2. * u * v * w - u * u - v * v - w * w) / (1. - w * w)).sqrt(),
                ],
            ]
        }
        _ => anyhow::bail!("data_invalid: 未知 ibrav={n}"),
    };
    ensure!(
        matrix.iter().flatten().all(|v| v.is_finite()),
        "data_invalid: 非法 celldm"
    );
    Ok(matrix.into_iter().flatten().collect())
}

/// 返回规范 XML 及全部 q 星；不补造频率、振型或动力学矩阵。
pub fn convert(raw: &str, geometry: &Geometry) -> Result<(String, Vec<[f64; 3]>)> {
    let lines = raw.lines().collect::<Vec<_>>();
    ensure!(
        lines
            .first()
            .is_some_and(|s| s.trim() == "Dynamical matrix file"),
        "data_invalid: 不是 QE 文本动力学矩阵"
    );
    let head = nums(lines.get(2).context("data_invalid: dyn 头截断")?)?;
    ensure!(
        head.len() == 9
            && head[0] > 0.0
            && head[1] > 0.0
            && head[..3].iter().all(|n| n.fract() == 0.0),
        "data_invalid: dyn 头格式错误"
    );
    let (nt, nat, ibrav) = (head[0] as usize, head[1] as usize, head[2] as i32);
    ensure!(
        nat == geometry.atoms.len() && nt <= nat,
        "data_invalid: dyn 原子数不符"
    );
    // 非零 ibrav 从来源 celldm 重建，逐项核验当前 XML，不能猜晶格。
    let mut cursor = 3;
    let lattice: Vec<f64> = if ibrav == 0 {
        ensure!(
            lines
                .get(cursor)
                .is_some_and(|s| s.trim() == "Basis vectors"),
            "data_invalid: dyn 缺 Basis vectors"
        );
        cursor += 1;
        let mut a = vec![];
        for _ in 0..3 {
            let row = nums(lines.get(cursor).context("data_invalid: 晶格截断")?)?;
            ensure!(row.len() == 3, "data_invalid: 晶格维数错误");
            a.extend(row);
            cursor += 1;
        }
        a
    } else {
        bravais(ibrav, &head[3..])?
    };
    ensure!(
        (head[3] - geometry.alat).abs() < 1e-6
            && lattice
                .iter()
                .zip(geometry.lattice.iter().flatten())
                .all(|(x, y)| (x - y / geometry.alat).abs() < 1e-8),
        "data_invalid: dyn 晶格/alat 与当前 SCF 不符"
    );
    let mut types = vec![];
    for n in 1..=nt {
        let parts = lines
            .get(cursor)
            .context("data_invalid: 质量表截断")?
            .split('\'')
            .collect::<Vec<_>>();
        ensure!(
            parts.len() == 3 && parts[0].trim().parse::<usize>()? == n,
            "data_invalid: 元素编号错误"
        );
        let mass = nums(parts[2])?;
        ensure!(
            mass.len() == 1
                && mass[0] > 0.0
                && parts[1].trim().chars().all(|c| c.is_ascii_alphabetic()),
            "data_invalid: 元素/质量错误"
        );
        // 与 QE Modules/constants.f90 的 AMU_RY 完全相同。
        types.push((
            parts[1].trim().to_owned(),
            mass[0] / (1.66053906660e-27 / 9.1093837015e-31 / 2.0),
        ));
        cursor += 1;
    }
    let mut atoms = vec![];
    for n in 1..=nat {
        let row = nums(lines.get(cursor).context("data_invalid: 原子坐标截断")?)?;
        ensure!(
            row.len() == 5
                && row[0] == n as f64
                && row[1] >= 1.0
                && row[1] <= nt as f64
                && row[1].fract() == 0.0,
            "data_invalid: 原子编号错误"
        );
        let t = row[1] as usize;
        ensure!(
            types[t - 1].0 == geometry.atoms[n - 1].0
                && row[2..]
                    .iter()
                    .zip(geometry.atoms[n - 1].1)
                    .all(|(x, y)| (x - y / geometry.alat).abs() < 1e-8),
            "data_invalid: dyn 原子顺序/元素/坐标不符"
        );
        atoms.push((t, row[2..].to_vec()));
        cursor += 1;
    }
    let mut matrices = String::new();
    let mut points = vec![];
    while let Some(offset) = lines[cursor..]
        .iter()
        .position(|s| s.contains("Dynamical  Matrix in cartesian axes"))
    {
        cursor += offset + 1;
        while lines.get(cursor).is_some_and(|s| s.trim().is_empty()) {
            cursor += 1;
        }
        let qline = lines.get(cursor).context("data_invalid: q 坐标截断")?;
        ensure!(qline.trim().starts_with("q ="), "data_invalid: 缺 q 坐标");
        let q: [f64; 3] = nums(qline.split_once('=').unwrap().1)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("data_invalid: q 维数错误"))?;
        points.push(q);
        cursor += 1;
        writeln!(matrices, "<DYNAMICAL_MAT_.{}>", points.len())?;
        tag(&mut matrices, "Q_POINT", &q);
        for i in 1..=nat {
            for j in 1..=nat {
                while lines.get(cursor).is_some_and(|s| s.trim().is_empty()) {
                    cursor += 1;
                }
                let pair = nums(lines.get(cursor).context("data_invalid: 原子对截断")?)?;
                ensure!(
                    pair == [i as f64, j as f64],
                    "data_invalid: 重复/错序的动力学矩阵原子对"
                );
                cursor += 1;
                let mut rows = vec![];
                for _ in 0..3 {
                    let row = nums(lines.get(cursor).context("data_invalid: 复矩阵截断")?)?;
                    ensure!(row.len() == 6, "data_invalid: 复矩阵行截断");
                    rows.extend(row);
                    cursor += 1;
                }
                tag(
                    &mut matrices,
                    &format!("PHI.{i}.{j}"),
                    &transpose_complex(&rows),
                );
            }
        }
        writeln!(matrices, "</DYNAMICAL_MAT_.{}>", points.len())?;
    }
    ensure!(!points.is_empty(), "data_invalid: dyn 无动力学矩阵");
    let mut tail = String::from("<FREQUENCIES_THZ_CMM1>\n");
    let freq_lines = lines
        .iter()
        .enumerate()
        .filter(|(_, s)| s.trim().starts_with("freq ("))
        .collect::<Vec<_>>();
    ensure!(
        freq_lines.len() == 3 * nat,
        "data_invalid: dyn 频率/振型不完整"
    );
    for (m, (i, line)) in freq_lines.into_iter().enumerate() {
        let parts = line.split('=').collect::<Vec<_>>();
        ensure!(parts.len() == 3, "data_invalid: 频率格式错误");
        let index = nums(parts[0].trim().strip_prefix("freq").unwrap())?;
        ensure!(index == [m as f64 + 1.0], "data_invalid: 模式编号重复/错序");
        let omega = [
            parts[1]
                .split_whitespace()
                .next()
                .context("频率缺失")?
                .parse::<f64>()?,
            parts[2]
                .split_whitespace()
                .next()
                .context("频率缺失")?
                .parse::<f64>()?,
        ];
        ensure!(
            omega.iter().all(|x| x.is_finite()),
            "data_invalid: 非有限频率"
        );
        tag(&mut tail, &format!("OMEGA.{}", m + 1), &omega);
        let mut u = vec![];
        for a in 0..nat {
            let row = nums(lines.get(i + 1 + a).context("data_invalid: 振型截断")?)?;
            ensure!(row.len() == 6, "data_invalid: 振型维数错误");
            u.extend(row);
        }
        tag(&mut tail, &format!("DISPLACEMENT.{}", m + 1), &u);
    }
    tail.push_str("</FREQUENCIES_THZ_CMM1>\n");
    let dielectric = dielectric(&lines, nat)?;
    let mut out = String::from("<?xml version=\"1.0\"?>\n<Root>\n<GEOMETRY_INFO>\n");
    for (name, n) in [
        ("NUMBER_OF_TYPES", nt),
        ("NUMBER_OF_ATOMS", nat),
        ("BRAVAIS_LATTICE_INDEX", 0),
        ("SPIN_COMPONENTS", 1),
    ] {
        writeln!(out, "<{name}>\n{n}\n</{name}>")?;
    }
    tag(&mut out, "CELL_DIMENSIONS", &head[3..]);
    tag(&mut out, "AT", &lattice);
    tag(
        &mut out,
        "BG",
        &geometry
            .reciprocal
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>(),
    );
    let a = &geometry.lattice;
    let volume = (a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]))
        .abs();
    tag(&mut out, "UNIT_CELL_VOLUME_AU", &[volume]);
    for (i, (name, mass)) in types.iter().enumerate() {
        writeln!(out, "<TYPE_NAME.{}>\n{name}\n</TYPE_NAME.{}>", i + 1, i + 1)?;
        tag(&mut out, &format!("MASS.{}", i + 1), &[*mass]);
    }
    for (i, (t, tau)) in atoms.iter().enumerate() {
        writeln!(
            out,
            "<ATOM.{} SPECIES=\"{}\" INDEX=\"{t}\" TAU=\"{}\">\n \n</ATOM.{}>",
            i + 1,
            types[t - 1].0,
            tau.iter()
                .map(|x| format!("{x:.16e}"))
                .collect::<Vec<_>>()
                .join(" "),
            i + 1
        )?;
    }
    writeln!(
        out,
        "<NUMBER_OF_Q>\n{}\n</NUMBER_OF_Q>\n</GEOMETRY_INFO>",
        points.len()
    )?;
    out.push_str(&dielectric);
    out.push_str(&matrices);
    out.push_str(&tail);
    out.push_str("</Root>\n");
    Ok((out, points))
}
