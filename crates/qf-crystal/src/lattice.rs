//! 晶格参数判定与三种 QE 晶格写出模式（复刻 `qeflow/crystal/lattice.py` 与 `qeflow/input_deck/lattice.py`）。

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{bail, Result};
use regex::Regex;

use qf_config::{Config, LATTICE_MODE_CELL_PARAMETERS, LATTICE_MODE_IBRAV_CELLDM};

use crate::Structure;

pub const BOHR_RADIUS_ANGS: f64 = 0.529177210903;

/// 解析得到的 QE 晶格参数。
#[derive(Debug, Clone)]
pub struct LatticeParameters {
    pub ibrav: i32,
    pub celldm: [f64; 6],
    pub source: String,
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn eqq(x: f64, y: f64) -> bool {
    (x - y).abs() < 1.0e-5
}

fn neqq(x: f64, y: f64) -> bool {
    (x - y).abs() > 1.0e-5
}

/// 将 Å 晶格转换为 Bohr 晶格。
pub fn lattice_to_bohr(lattice: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut out = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = lattice[i][j] / BOHR_RADIUS_ANGS;
        }
    }
    out
}

/// 由晶格矢量判定 QE ibrav（复刻 `_at2ibrav`）。
pub fn at2ibrav(a1: [f64; 3], a2: [f64; 3], a3: [f64; 3]) -> i32 {
    let v1 = norm(a1);
    let v2 = norm(a2);
    let v3 = norm(a3);
    let cosbc = dot(a2, a3) / v2 / v3;
    let cosac = dot(a1, a3) / v1 / v3;
    let cosab = dot(a1, a2) / v1 / v2;
    let mut ibrav = 14;
    if eqq(v1, v2) && eqq(v1, v3) {
        if eqq(cosab, cosac) && eqq(cosab, cosbc) {
            if eqq(cosab, 0.0) {
                ibrav = 1;
            } else if eqq(cosab, 0.5) {
                ibrav = 2;
            } else if eqq(cosab, -1.0 / 3.0) {
                ibrav = -3;
            } else if eqq(a1[2].abs(), a2[2].abs()) && eqq(a2[2].abs(), a3[2].abs()) {
                ibrav = 5;
            } else {
                ibrav = -5;
            }
        } else if eqq(cosab, cosac) && neqq(cosab, cosbc) {
            if eqq(a1[0].abs(), a1[1].abs()) && eqq(a2[0].abs(), a2[1].abs()) {
                ibrav = 7;
            } else {
                ibrav = 3;
            }
        } else if eqq(cosab, -cosac) && eqq(cosab, cosbc) && eqq(cosab, 1.0 / 3.0) {
            ibrav = 3;
        } else if eqq(a1[0].abs(), a2[0].abs()) && eqq(a2[1].abs(), a2[1].abs()) {
            ibrav = 11;
        }
    } else if eqq(v1, v2) && neqq(v1, v3) {
        if eqq(cosab, 0.0) && eqq(cosac, 0.0) && eqq(cosbc, 0.0) {
            ibrav = 6;
        } else if eqq(cosab, -0.5) && eqq(cosac, 0.0) && eqq(cosbc, 0.0) {
            ibrav = 4;
        } else if eqq(cosac, 0.0) && eqq(cosbc, 0.0) {
            if eqq(a1[0], a2[0]) && eqq(a1[1], -a2[1]) {
                ibrav = -9;
            } else if eqq(a1[0], -a2[0]) && eqq(a1[1], a2[1]) {
                ibrav = 9;
            }
        } else if eqq(cosac, -cosbc) {
            ibrav = -13;
        }
    } else if eqq(v1, v3) && neqq(v1, v2) {
        ibrav = 13;
    } else if eqq(v2, v3) && neqq(v1, v2) {
        ibrav = 91;
    } else if neqq(v1, v2) && neqq(v1, v3) && neqq(v2, v3) {
        if eqq(cosab, 0.0) && eqq(cosac, 0.0) && eqq(cosbc, 0.0) {
            ibrav = 8;
        } else if neqq(cosab, 0.0) && eqq(cosac, 0.0) && eqq(cosbc, 0.0) {
            ibrav = 12;
        } else if eqq(cosab, 0.0) && neqq(cosac, 0.0) && eqq(cosbc, 0.0) {
            ibrav = -12;
        } else if neqq(cosab, 0.0) && neqq(cosac, 0.0) && neqq(cosbc, 0.0) {
            if eqq(a1[0].abs(), a2[0].abs())
                && eqq(a1[2].abs(), a3[2].abs())
                && eqq(a2[1].abs(), a3[1].abs())
            {
                ibrav = 10;
            } else {
                ibrav = 14;
            }
        }
    }
    ibrav
}

/// 由 ibrav 与晶格矢量推导 celldm（复刻 `_at2celldm`）。
pub fn at2celldm(ibrav: i32, alat: f64, a1: [f64; 3], a2: [f64; 3], a3: [f64; 3]) -> [f64; 6] {
    let mut celldm = [0.0f64; 6];
    match ibrav {
        0 => celldm[0] = 1.0,
        1 => celldm[0] = norm(a1),
        2 => celldm[0] = (dot(a1, a1) * 2.0).sqrt(),
        3 | -3 => celldm[0] = (dot(a1, a1) / 3.0).sqrt() * 2.0,
        4 => {
            celldm[0] = norm(a1);
            celldm[2] = norm(a3) / celldm[0];
        }
        5 | -5 => {
            celldm[0] = norm(a1);
            celldm[3] = dot(a1, a2) / celldm[0] / norm(a2);
        }
        6 => {
            celldm[0] = norm(a1);
            celldm[2] = norm(a3) / celldm[0];
        }
        7 => {
            celldm[0] = (a1[0].abs()) * 2.0;
            celldm[2] = a1[2] / a1[0];
        }
        8 => {
            celldm[0] = norm(a1);
            celldm[1] = norm(a2) / celldm[0];
            celldm[2] = norm(a3) / celldm[0];
        }
        9 | -9 => {
            celldm[0] = (a1[0].abs()) * 2.0;
            celldm[1] = (a2[1].abs()) * 2.0 / celldm[0];
            celldm[2] = (a3[2].abs()) / celldm[0];
        }
        91 => {
            celldm[0] = norm(a1);
            celldm[1] = (a2[1].abs()) * 2.0 / celldm[0];
            celldm[2] = (a3[2].abs()) * 2.0 / celldm[0];
        }
        10 => {
            celldm[0] = (a1[0].abs()) * 2.0;
            celldm[1] = (a2[1].abs()) * 2.0 / celldm[0];
            celldm[2] = (a3[2].abs()) * 2.0 / celldm[0];
        }
        11 => {
            celldm[0] = (a1[0].abs()) * 2.0;
            celldm[1] = (a1[1].abs()) * 2.0 / celldm[0];
            celldm[2] = (a1[2].abs()) * 2.0 / celldm[0];
        }
        12 | -12 => {
            celldm[0] = norm(a1);
            celldm[1] = norm(a2) / celldm[0];
            celldm[2] = norm(a3) / celldm[0];
            if ibrav == 12 {
                celldm[3] = dot(a1, a2) / celldm[0] / norm(a2);
            } else {
                celldm[4] = dot(a1, a3) / celldm[0] / norm(a3);
            }
        }
        13 => {
            celldm[0] = (a1[0].abs()) * 2.0;
            celldm[1] = norm(a2) / celldm[0];
            celldm[2] = a1[2] / a1[0];
            celldm[3] = a2[0] / a1[0] / celldm[1] / 2.0;
        }
        -13 => {
            celldm[0] = (a1[0].abs()) * 2.0;
            celldm[1] = a2[1] / a2[0];
            celldm[2] = norm(a3) / celldm[0];
            celldm[4] = a3[0] / a1[0] / celldm[2] / 2.0;
        }
        14 => {
            celldm[0] = norm(a1);
            celldm[1] = norm(a2) / celldm[0];
            celldm[2] = norm(a3) / celldm[0];
            celldm[3] = dot(a3, a2) / norm(a2) / norm(a3);
            celldm[4] = dot(a3, a1) / celldm[0] / norm(a3);
            celldm[5] = dot(a1, a2) / celldm[0] / norm(a2);
        }
        _ => {}
    }
    celldm[0] *= alat;
    celldm
}

fn celldm2abc(ibrav: i32, celldm: [f64; 6]) -> (f64, f64, f64, f64, f64, f64) {
    let a = celldm[0] * BOHR_RADIUS_ANGS;
    let b = celldm[0] * celldm[1] * BOHR_RADIUS_ANGS;
    let c = celldm[0] * celldm[2] * BOHR_RADIUS_ANGS;
    let (cosab, cosac, cosbc) = if ibrav == 14 || ibrav == 0 {
        (celldm[5], celldm[4], celldm[3])
    } else if ibrav == -12 || ibrav == -13 {
        (0.0, celldm[4], 0.0)
    } else if ibrav == -5 || ibrav == 5 || ibrav == 12 || ibrav == 13 {
        (celldm[3], 0.0, 0.0)
    } else {
        (0.0, 0.0, 0.0)
    };
    (a, b, c, cosab, cosac, cosbc)
}

fn required_celldm(ibrav: i32, celldm: [f64; 6]) -> Vec<(i32, f64)> {
    let indices: &[i32] = match ibrav {
        1 | 2 | 3 | -3 => &[1],
        4 | 5 | -5 | 6 | 7 => &[1, 3],
        8 | 9 | -9 | 91 | 10 | 11 => &[1, 2, 3],
        12 => &[1, 2, 3, 4],
        -12 => &[1, 2, 3, 5],
        13 => &[1, 2, 3, 4],
        -13 => &[1, 2, 3, 5],
        14 => &[1, 2, 3, 4, 5, 6],
        _ => &[],
    };
    indices
        .iter()
        .map(|&i| (i, celldm[(i - 1) as usize]))
        .collect()
}

fn required_abc(ibrav: i32, celldm: [f64; 6]) -> Vec<(String, f64)> {
    let (a, b, c, cosab, cosac, cosbc) = celldm2abc(ibrav, celldm);
    let mapping: Vec<(&str, f64)> = match ibrav {
        1 | 2 | 3 | -3 => vec![("A", a)],
        4 | 6 | 7 => vec![("A", a), ("C", c)],
        5 | -5 => vec![("A", a), ("cosAB", cosab)],
        8 | 9 | -9 | 91 | 10 | 11 => vec![("A", a), ("B", b), ("C", c)],
        12 => vec![("A", a), ("B", b), ("C", c), ("cosAB", cosab)],
        -12 => vec![("A", a), ("B", b), ("C", c), ("cosAC", cosac)],
        13 => vec![("A", a), ("B", b), ("C", c), ("cosAB", cosab)],
        -13 => vec![("A", a), ("B", b), ("C", c), ("cosAC", cosac)],
        14 => vec![
            ("A", a),
            ("B", b),
            ("C", c),
            ("cosAB", cosab),
            ("cosAC", cosac),
            ("cosBC", cosbc),
        ],
        _ => vec![],
    };
    mapping.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn format_float(value: f64) -> String {
    // 近似 Python 的 :.12g：12 位精度后去除多余尾零与小数点。
    let s = format!("{value:.12}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

/// 解析并写出 &SYSTEM 中的晶格段（复刻 `write_system_lattice`）。
pub fn write_system_lattice(config: &Config, structure: &Structure) -> Result<String> {
    let mode = config.lattice_mode()?;
    if mode == LATTICE_MODE_CELL_PARAMETERS {
        return Ok("  ibrav = 0\n".to_string());
    }
    let bohr = lattice_to_bohr(structure.lattice);
    let params = resolve_lattice_parameters(config, bohr)?;
    let mut s = format!("  ibrav = {}\n", params.ibrav);
    if mode == LATTICE_MODE_IBRAV_CELLDM {
        for (i, v) in required_celldm(params.ibrav, params.celldm) {
            s.push_str(&format!("  celldm({i}) = {}\n", format_float(v)));
        }
    } else {
        for (k, v) in required_abc(params.ibrav, params.celldm) {
            s.push_str(&format!("  {k} = {}\n", format_float(v)));
        }
    }
    Ok(s)
}

/// 写出 ATOMIC_POSITIONS 段（复刻 `write_atomic_positions`）。
pub fn write_atomic_positions(structure: &Structure) -> String {
    let coord = if structure.coord_type.to_lowercase().starts_with('d') {
        "crystal"
    } else {
        "angstrom"
    };
    let mut s = format!("ATOMIC_POSITIONS {{{coord}}}\n");
    let mut atom_index = 0usize;
    for (ei, element) in structure.elements.iter().enumerate() {
        for _ in 0..structure.counts[ei] {
            let p = structure.positions[atom_index];
            s.push_str(&format!(
                "  {element:3} {p0:21.17} {p1:21.17} {p2:21.17}\n",
                p0 = p[0],
                p1 = p[1],
                p2 = p[2]
            ));
            atom_index += 1;
        }
    }
    s.push('\n');
    s
}

/// 必要时写出 CELL_PARAMETERS 段（复刻 `write_cell_parameters_if_needed`）。
pub fn write_cell_parameters_if_needed(config: &Config, structure: &Structure) -> Result<String> {
    if config.lattice_mode()? != LATTICE_MODE_CELL_PARAMETERS {
        return Ok(String::new());
    }
    let mut s = "CELL_PARAMETERS {angstrom}\n".to_string();
    for vec in structure.lattice {
        s.push_str(&format!(
            "  {v0:21.17} {v1:21.17} {v2:21.17}\n",
            v0 = vec[0],
            v1 = vec[1],
            v2 = vec[2]
        ));
    }
    s.push('\n');
    Ok(s)
}

/// 解析晶格参数：优先调用 QE `cell2ibrav.x`，失败回退 Python 等价判定（复刻 `resolve_lattice_parameters`）。
pub fn resolve_lattice_parameters(
    config: &Config,
    lattice_bohr: [[f64; 3]; 3],
) -> Result<LatticeParameters> {
    if let Some(exe) = find_cell2ibrav_executable(config.qe.executable_path()) {
        if let Ok(p) = run_cell2ibrav(&exe, lattice_bohr) {
            return Ok(p);
        }
    }
    Ok(python_cell2ibrav(lattice_bohr))
}

fn find_cell2ibrav_executable(exec_path: &str) -> Option<String> {
    if exec_path.trim().is_empty() {
        return None;
    }
    for token in exec_path.split_whitespace().rev() {
        if Path::new(token)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "cell2ibrav.x")
            .unwrap_or(false)
            && Path::new(token).is_file()
        {
            return Some(token.to_string());
        }
        if Path::new(token).is_dir() {
            let tool = format!("{token}/cell2ibrav.x");
            if Path::new(&tool).is_file() {
                return Some(tool);
            }
        }
    }
    None
}

fn run_cell2ibrav(executable: &str, lattice_bohr: [[f64; 3]; 3]) -> Result<LatticeParameters> {
    let input = lattice_bohr
        .iter()
        .map(|row| row.iter().map(|v| format!("{v:.16}")).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n");
    let input = format!("{input}\n1\n");

    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let ibrav_re = Regex::new(r"ibrav\s*=\s*([+-]?\d+)")?;
    let ibrav = ibrav_re
        .captures(&stdout)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<i32>().ok())
        .ok_or_else(|| anyhow::anyhow!("cell2ibrav.x 输出中未找到 ibrav"))?;

    let mut celldm = [0.0f64; 6];
    let cd_re =
        Regex::new(r"celldm\((\d)\)\s*=\s*([+-]?\d+(?:\.\d*)?(?:[EeDd][+-]?\d+)?)")?;
    for cap in cd_re.captures_iter(&stdout) {
        let idx: usize = cap[1].parse()?;
        if (1..=6).contains(&idx) {
            let v: f64 = cap[2].replace('D', "E").replace('d', "e").parse()?;
            celldm[idx - 1] = v;
        }
    }

    if let Some(m) = Regex::new(r"Discrepancy old/new in bohr\s*=\s*([+-]?\d+(?:\.\d*)?(?:[EeDd][+-]?\d+)?)")?
        .captures(&stdout)
    {
        let disc: f64 = m[1].replace('D', "E").replace('d', "e").parse()?;
        if disc > 1.0e-4 {
            bail!("cell2ibrav.x 重构晶胞偏差过大: {disc} bohr");
        }
    }

    Ok(LatticeParameters {
        ibrav,
        celldm,
        source: "cell2ibrav.x".to_string(),
    })
}

fn python_cell2ibrav(lattice_bohr: [[f64; 3]; 3]) -> LatticeParameters {
    let ibrav = at2ibrav(lattice_bohr[0], lattice_bohr[1], lattice_bohr[2]);
    let celldm = at2celldm(ibrav, 1.0, lattice_bohr[0], lattice_bohr[1], lattice_bohr[2]);
    LatticeParameters {
        ibrav,
        celldm,
        source: "python".to_string(),
    }
}
