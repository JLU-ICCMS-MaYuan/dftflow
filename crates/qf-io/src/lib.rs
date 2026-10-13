//! QE / VASP / LOBSTER 原生输出集中读取（严格对齐 `qeflow/io_reader`）。
//!
//! 移植自 Python `qeflow/io_reader/{common,pw,ph,matdyn,dos}.py`，字段与解析语义 1:1 对齐，
//! 供后续 P2 各 CLI 与对照 pytest 黄金基线复用。当前覆盖：`scf/nscf/eband` 的 PW 输出、
//! `ph.x` 输出、`q2r`/`matdyn` 输出、`dyn0` 的 q 点列表、声子谱频率、`TDOS`，
//! 以及 VASP 的 `EIGENVAL`/`OUTCAR`/`DOSCAR`（COHPCAR.lobster 与 LOCPOT/CHGCAR 留待后续补齐）。

pub mod qe_proj;
pub mod vasp_cohpar;
pub mod vasp_doscar;
pub mod vasp_eigenval;
pub mod vasp_locpot;
pub mod vasp_outcar;

use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use std::path::Path;

/// 错误标记集合，对齐 `qeflow/io_reader/common.py:ERROR_MARKERS`。
const ERROR_MARKERS: &[&str] = &[
    "%%%%%%%%%%%%%%",
    "error in routine",
    "error:",
    "stopping ...",
    "convergence not achieved",
];

/// 判断文本是否含 QE 错误标记（大小写不敏感），对齐 `has_error_marker`。
pub fn has_error_marker(text: &str) -> bool {
    let lowered = text.to_lowercase();
    ERROR_MARKERS.iter().any(|m| lowered.contains(&m.to_lowercase()))
}

/// 读取文本；文件不存在返回空串，非法 UTF-8 用 lossy，对齐 `read_text_if_exists`。
pub fn read_text_if_exists<P: AsRef<Path>>(path: P) -> String {
    match std::fs::read(&path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => String::new(),
    }
}

fn first_float(text: &str, pat: &str) -> Option<f64> {
    let re = Regex::new(pat).ok()?;
    re.captures(text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

fn first_int(text: &str, pat: &str) -> Option<usize> {
    let re = Regex::new(pat).ok()?;
    re.captures(text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

fn float_values(line: &str) -> Vec<f64> {
    let re = Regex::new(r"[-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?").unwrap();
    re.find_iter(line)
        .filter_map(|m| m.as_str().parse::<f64>().ok())
        .collect()
}

fn read_cell_angstrom(text: &str) -> Option<[[f64; 3]; 3]> {
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if !line.to_uppercase().contains("CELL_PARAMETERS") {
            continue;
        }
        let mut rows = [[0.0f64; 3]; 3];
        let mut ok = true;
        for off in 0..3 {
            match lines.get(i + 1 + off) {
                Some(l) => {
                    let p: Vec<f64> = l.split_whitespace().filter_map(|s| s.parse().ok()).collect();
                    if p.len() < 3 {
                        ok = false;
                        break;
                    }
                    rows[off] = [p[0], p[1], p[2]];
                }
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            return Some(rows);
        }
    }
    None
}

/// `pw.x` 输出解析结果，对齐 `qeflow/io_reader/pw.py:PwOutputData`。
#[derive(Debug, Clone, Default, Serialize)]
pub struct PwOutput {
    pub finished: bool,
    pub has_error: bool,
    pub fermi_energy_ev: Option<f64>,
    pub total_energy_ry: Option<f64>,
    pub number_of_atoms: Option<usize>,
    pub number_of_kpoints: Option<usize>,
    pub cell_angstrom: Option<[[f64; 3]; 3]>,
}

/// 解析 `pw.x` 输出文本（纯函数）。
pub fn parse_pw_output(text: &str) -> PwOutput {
    PwOutput {
        finished: text.contains("JOB DONE"),
        has_error: has_error_marker(text),
        fermi_energy_ev: first_float(text, r"(?i)the Fermi energy is\s+([-\d.]+)\s+ev"),
        total_energy_ry: first_float(text, r"!\s+total energy\s+=\s+([-\d.]+)\s+Ry"),
        number_of_atoms: first_int(text, r"(?i)number of atoms/cell\s*=\s*(\d+)"),
        number_of_kpoints: first_int(text, r"(?i)number of k points\s*=\s*(\d+)"),
        cell_angstrom: read_cell_angstrom(text),
    }
}

/// 读取并解析 `pw.x` 输出文件。
pub fn read_pw_output<P: AsRef<Path>>(path: P) -> Result<PwOutput> {
    let text = read_text_if_exists(&path);
    if text.is_empty() {
        return Err(anyhow::anyhow!(
            "read {}: file missing or empty",
            path.as_ref().display()
        ))
        .with_context(|| format!("parse pw output"));
    }
    Ok(parse_pw_output(&text))
}

/// 兼容别名：scf/nscf/eband 共用同一解析。
pub fn read_scf_output<P: AsRef<Path>>(path: P) -> Result<PwOutput> {
    read_pw_output(path)
}

/// `ph.x` 输出解析结果，对齐 `qeflow/io_reader/ph.py:PhOut`。
#[derive(Debug, Clone, Default, Serialize)]
pub struct PhOut {
    pub finished: bool,
    pub has_error: bool,
}

/// 解析 `ph.x` 输出文本。
pub fn parse_ph_out(text: &str) -> PhOut {
    PhOut {
        finished: text.contains("JOB DONE"),
        has_error: has_error_marker(text),
    }
}

/// 读取并解析 `ph.x` 输出文件。
pub fn read_ph_out<P: AsRef<Path>>(path: P) -> Result<PhOut> {
    Ok(parse_ph_out(&read_text_if_exists(path)))
}

/// `q2r.x` / `matdyn.x` 输出解析结果，对齐 `qeflow/io_reader/matdyn.py:Q2rOut/MatdynOut`。
#[derive(Debug, Clone, Default, Serialize)]
pub struct Q2rOut {
    pub finished: bool,
    pub has_error: bool,
}

/// 解析 `q2r.x`/`matdyn.x` 输出文本。
pub fn parse_q2r_out(text: &str) -> Q2rOut {
    Q2rOut {
        finished: text.contains("JOB DONE"),
        has_error: has_error_marker(text),
    }
}

/// dyn0 q 点列表项，对齐 `qeflow/io_reader/ph.py:parse_dyn0_qpoints`。
#[derive(Debug, Clone, Serialize)]
pub struct Dyn0QPoint {
    pub index: usize,
    pub q_crystal: [f64; 3],
}

/// 解析 dyn0 的 q 点列表：第 1 行 `nq1 nq2 nq3`，第 2 行 q 点数量，其后每行一个 `q_crystal`。
pub fn parse_dyn0_qpoints(text: &str) -> Result<Vec<Dyn0QPoint>> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() < 2 {
        anyhow::bail!("dyn0 文件格式不完整");
    }
    if lines[0].split_whitespace().count() != 3 {
        anyhow::bail!("dyn0 第一行无法解析 nq1/nq2/nq3: {}", lines[0]);
    }
    let q_count: usize = lines[1]
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("dyn0 第二行无法解析 q 点数量: {}", lines[1]))?;
    if lines.len() < 2 + q_count {
        anyhow::bail!("dyn0 q 点数量不足: 期望 {q_count}，实际 {}", lines.len() - 2);
    }
    let mut v = Vec::with_capacity(q_count);
    for idx in 0..q_count {
        let parts: Vec<f64> = lines[2 + idx]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() != 3 {
            anyhow::bail!("dyn0 q 点行无法解析为 3 个分量: {}", lines[2 + idx]);
        }
        v.push(Dyn0QPoint {
            index: idx + 1,
            q_crystal: [parts[0], parts[1], parts[2]],
        });
    }
    Ok(v)
}

/// 高对称路径标签项，对齐 `qeflow/io_reader/matdyn.py:read_labelinfo`。
#[derive(Debug, Clone, Serialize)]
pub struct LabelInfo {
    pub label: String,
    pub point_index: usize,
    pub distance: f64,
    pub kx: f64,
    pub ky: f64,
    pub kz: f64,
}

/// 解析高对称路径标签文件（`labelinfo`）。
pub fn read_labelinfo(text: &str) -> Vec<LabelInfo> {
    let mut v = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }
        let label = parts[0].replace("GAMMA", "Γ");
        if let (Ok(pi), Ok(d), Ok(kx), Ok(ky), Ok(kz)) = (
            parts[1].parse::<usize>(),
            parts[2].parse::<f64>(),
            parts[3].parse::<f64>(),
            parts[4].parse::<f64>(),
            parts[5].parse::<f64>(),
        ) {
            v.push(LabelInfo {
                label,
                point_index: pi,
                distance: d,
                kx,
                ky,
                kz,
            });
        }
    }
    v
}

/// 解析声子谱频率（对齐 `qeflow/io_reader/matdyn.py:read_band_frequencies`），返回每个 q 点的频率列表。
pub fn read_band_frequencies(text: &str, expected_modes: usize) -> Result<Vec<Vec<f64>>> {
    let lines: Vec<&str> = text.lines().collect();
    if let Some(rows) = read_qe_plot_frequencies(&lines, expected_modes) {
        return Ok(rows);
    }

    let freq_re = Regex::new(
        r"(?i)freq\s*\(\s*\d+\s*\)\s*=\s*[-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?\s*(?:\[?[A-Za-z]+\]?)?\s*=\s*([-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?)",
    )
    .unwrap();
    let mut grouped: Vec<Vec<f64>> = Vec::new();
    let mut current: Vec<f64> = Vec::new();
    let mut numeric_rows: Vec<Vec<f64>> = Vec::new();
    for line in &lines {
        if let Some(cap) = freq_re.captures(line) {
            if let Some(m) = cap.get(1) {
                if let Ok(v) = m.as_str().parse::<f64>() {
                    current.push(v);
                    if current.len() == expected_modes {
                        grouped.push(std::mem::take(&mut current));
                    }
                    continue;
                }
            }
        }
        let values = float_values(line);
        if values.len() >= expected_modes {
            numeric_rows.push(values[values.len() - expected_modes..].to_vec());
        }
    }
    if !grouped.is_empty() {
        return Ok(grouped);
    }
    if !current.is_empty() {
        anyhow::bail!("声子谱 freq 行数量不是模式数的整数倍。");
    }
    if !numeric_rows.is_empty() {
        return Ok(numeric_rows);
    }
    anyhow::bail!("无法从声子谱文件解析频率");
}

fn read_qe_plot_frequencies(lines: &[&str], expected_modes: usize) -> Option<Vec<Vec<f64>>> {
    let header_re =
        Regex::new(r"(?i)&plot\s+nbnd\s*=\s*(\d+)\s*,\s*nks\s*=\s*(\d+)").unwrap();
    let mut header_index = None;
    let mut nbnd = 0usize;
    let mut nks = 0usize;
    for (i, line) in lines.iter().enumerate() {
        if let Some(cap) = header_re.captures(line) {
            nbnd = cap.get(1)?.as_str().parse().ok()?;
            nks = cap.get(2)?.as_str().parse().ok()?;
            header_index = Some(i);
            break;
        }
    }
    let header_index = header_index?;
    if nbnd != expected_modes {
        return None;
    }
    let mut rows = Vec::with_capacity(nks);
    let mut li = header_index + 1;
    for _ in 1..=nks {
        li = next_nonempty(lines, li);
        if li >= lines.len() {
            return None;
        }
        let coords = float_values(lines[li]);
        if coords.len() < 3 {
            return None;
        }
        li += 1;
        let mut modes = Vec::new();
        while modes.len() < nbnd {
            li = next_nonempty(lines, li);
            if li >= lines.len() {
                return None;
            }
            modes.extend(float_values(lines[li]));
            li += 1;
        }
        if modes.len() != nbnd {
            return None;
        }
        rows.push(modes);
    }
    if rows.len() != nks {
        return None;
    }
    Some(rows)
}

fn next_nonempty(lines: &[&str], start: usize) -> usize {
    let mut i = start;
    while i < lines.len() && lines[i].trim().is_empty() {
        i += 1;
    }
    i
}

/// 解析纯数值行（跳过空行与 `#` 注释），对齐 `qeflow/io_reader/matdyn.py:read_numeric_rows`。
pub fn read_numeric_rows(text: &str) -> Vec<Vec<f64>> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        let values = float_values(s);
        if !values.is_empty() {
            rows.push(values);
        }
    }
    rows
}

/// 态密度（TDOS）解析结果，对齐 `qeflow/io_reader/dos.py:parse_tdos`。
#[derive(Debug, Clone, Default, Serialize)]
pub struct Tdos {
    pub energies: Vec<f64>,
    pub tdos: Vec<f64>,
    pub itdos: Vec<f64>,
    pub fermi: Option<f64>,
}

/// 解析 TDOS 文件：以 `#` 开头的行可含 `EFermi=...`；数据行取前三列。
pub fn parse_tdos(text: &str) -> Result<Tdos> {
    let fermi_re = Regex::new(r"EFermi\s*=\s*([-\d.]+)").unwrap();
    let mut energies = Vec::new();
    let mut tdos = Vec::new();
    let mut itdos = Vec::new();
    let mut fermi = None;
    for line in text.lines() {
        if line.trim_start().starts_with('#') {
            if let Some(c) = fermi_re.captures(line) {
                fermi = c.get(1).and_then(|m| m.as_str().parse().ok());
            }
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[0].parse::<f64>().is_ok() {
            energies.push(parts[0].parse().unwrap());
            tdos.push(parts[1].parse().unwrap());
            itdos.push(parts[2].parse().unwrap());
        }
    }
    if energies.is_empty() {
        anyhow::bail!("未从 TDOS 文件解析到数据");
    }
    Ok(Tdos {
        energies,
        tdos,
        itdos,
        fermi,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCF: &str = "\
     number of atoms/cell = 2
     number of k points= 100
     CELL_PARAMETERS (angstrom)
       3.0  0.0  0.0
       0.0  3.0  0.0
       0.0  0.0  3.5
     !    total energy   = -123.456 Ry
     the Fermi energy is  12.34 ev
     JOB DONE.
";

    #[test]
    fn parses_pw_output() {
        let r = parse_pw_output(SCF);
        assert!(r.finished);
        assert!(!r.has_error);
        assert_eq!(r.number_of_atoms, Some(2));
        assert_eq!(r.number_of_kpoints, Some(100));
        assert_eq!(r.total_energy_ry, Some(-123.456));
        assert_eq!(r.fermi_energy_ev, Some(12.34));
        let cell = r.cell_angstrom.unwrap();
        assert_eq!(cell[0], [3.0, 0.0, 0.0]);
        assert_eq!(cell[2], [0.0, 0.0, 3.5]);
    }

    #[test]
    fn detects_error_marker() {
        assert!(has_error_marker("some text\n error in routine foo\n bar"));
        assert!(!has_error_marker("normal output JOB DONE"));
    }

    #[test]
    fn parses_dyn0_qpoints() {
        let txt = "4 4 1\n3\n0.0 0.0 0.0\n0.5 0.0 0.0\n0.0 0.5 0.0\n";
        let qs = parse_dyn0_qpoints(txt).unwrap();
        assert_eq!(qs.len(), 3);
        assert_eq!(qs[0].q_crystal, [0.0, 0.0, 0.0]);
        assert_eq!(qs[1].index, 2);
    }

    #[test]
    fn parses_band_frequencies() {
        // 两个 q 点、每个 3 个模式（数值行形式）。
        let txt = "1.0 2.0 3.0\n4.0 5.0 6.0\n";
        let rows = read_band_frequencies(txt, 3).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1], vec![4.0, 5.0, 6.0]);
    }

    #[test]
    fn parses_tdos() {
        let txt = "# EFermi = 5.0 eV\n-1.0 0.1 0.01\n0.0 0.5 0.05\n1.0 0.2 0.02\n";
        let t = parse_tdos(txt).unwrap();
        assert_eq!(t.fermi, Some(5.0));
        assert_eq!(t.energies, vec![-1.0, 0.0, 1.0]);
        assert_eq!(t.tdos, vec![0.1, 0.5, 0.2]);
        assert_eq!(t.itdos.len(), 3);
    }

    #[test]
    fn parses_labelinfo() {
        let txt = "GAMMA 1 0.0 0.0 0.0 0.0\nM 2 0.5 0.5 0.0 0.0\n";
        let labels = read_labelinfo(txt);
        assert_eq!(labels[0].label, "Γ");
        assert_eq!(labels[1].point_index, 2);
        assert_eq!(labels[1].distance, 0.5);
    }
}
