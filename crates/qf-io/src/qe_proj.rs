//! QE `projwfc.x` / `dos.x` 后处理（复刻 `qeflow/eband/qe_process_eband.py` 与
//! `qeflow/eledos/qe_process_dos.py` 及其 `io_reader/bands.py`、`io_reader/dos.py`）。
//!
//! 纯文件后处理：不 spawn 外部程序、不读写状态文件。输出格式与 Python 逐字段一致：
//! - eband: `<prefix>_eleband_proj.dat` / `<prefix>_eleband_proj_sum.dat` / `<band_prefix>_band.dat`
//! - eledos: `<prefix>_dos_proj.dat` / `_dos_proj_sum.dat` / `_idos_proj.dat` / `_idos_proj_sum.dat`

use anyhow::{bail, Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

/// 复刻 `io_reader/common.is_float`。
pub fn is_float(text: &str) -> bool {
    text.parse::<f64>().is_ok()
}

fn next_nonempty(lines: &[impl AsRef<str>], start: usize) -> usize {
    let mut idx = start;
    while idx < lines.len() && lines[idx].as_ref().trim().is_empty() {
        idx += 1;
    }
    idx
}

// ---------------------------------------------------------------------------
// 轨道命名（io_reader/bands.orbital_from_lm / io_reader/dos.orbital_components）
// ---------------------------------------------------------------------------

pub fn orbital_from_lm(l_val: i64, m_val: i64) -> String {
    match l_val {
        0 => "s".to_string(),
        1 => match m_val {
            1 => "pz".to_string(),
            2 => "px".to_string(),
            3 => "py".to_string(),
            _ => format!("p{m_val}"),
        },
        2 => match m_val {
            1 => "dz2".to_string(),
            2 => "dxz".to_string(),
            3 => "dyz".to_string(),
            4 => "dx2-y2".to_string(),
            5 => "dxy".to_string(),
            _ => format!("d{m_val}"),
        },
        3 => match m_val {
            1 => "fz3".to_string(),
            2 => "fxz2".to_string(),
            3 => "fyz2".to_string(),
            4 => "fxyz".to_string(),
            5 => "fx2y2".to_string(),
            6 => "fy3x2-y2".to_string(),
            7 => "fzx2-y2".to_string(),
            _ => format!("f{m_val}"),
        },
        _ => format!("l{l_val}m{m_val}"),
    }
}

pub fn orbital_components(l_letter: &str) -> &'static [&'static str] {
    match l_letter {
        "s" => &["s"],
        "p" => &["pz", "px", "py"],
        "d" => &["dz2", "dxz", "dyz", "dx2-y2", "dxy"],
        "f" => &["fz3", "fxz2", "fyz2", "fxyz", "fx2y2", "fy3x2-y2", "fzx2-y2"],
        _ => &[],
    }
}

// ---------------------------------------------------------------------------
// projwfc / elebanddata 解析
// ---------------------------------------------------------------------------

/// `parse_projwfc_file` 的返回（labels 与展平的投影权重，索引 `k*nbnd+b`）。
pub struct ProjwfcData {
    pub labels: Vec<String>,
    /// `weights[proj][k * nbnd + b]`
    pub weights: Vec<Vec<f64>>,
    pub nk: usize,
    pub nbnd: usize,
}

fn ft_header_line(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() {
        return false;
    }
    let re = Regex::new(r"(?i)^[FT](\s+[FT])*$").unwrap();
    re.is_match(t)
}

/// 解析 projwfc 投影文件；格式不匹配时返回 `Ok(None)`（对齐 Python 返回 None）。
pub fn parse_projwfc_file(path: &str) -> Result<Option<ProjwfcData>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("读取 projwfc 文件失败: {path}"))?;
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end()).collect();

    let mut header_idx = None;
    let mut nproj = 0usize;
    let mut nk = 0usize;
    let mut nbnd = 0usize;
    for index in 0..lines.len() {
        let parts: Vec<&str> = lines[index].split_whitespace().collect();
        if parts.len() == 3
            && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
            && index + 1 < lines.len()
            && ft_header_line(lines[index + 1])
        {
            nproj = parts[0].parse().context("projwfc 头部 nproj 解析失败")?;
            nk = parts[1].parse().context("projwfc 头部 nk 解析失败")?;
            nbnd = parts[2].parse().context("projwfc 头部 nbnd 解析失败")?;
            header_idx = Some(index + 2);
            break;
        }
    }
    let Some(mut idx) = header_idx else {
        return Ok(None);
    };

    let mut labels = Vec::with_capacity(nproj);
    let mut weights = vec![vec![0.0_f64; nk * nbnd]; nproj];
    let shell_re = Regex::new(r"[SPDFspdf]$").unwrap();

    for proj_idx in 0..nproj {
        idx = next_nonempty(&lines, idx);
        if idx >= lines.len() {
            bail!("projwfc 文件结构不完整，缺少投影头部。");
        }
        let tokens: Vec<&str> = lines[idx].split_whitespace().collect();
        idx += 1;
        if tokens.len() < 6 {
            bail!("投影头部解析失败: {}", lines[idx - 1]);
        }

        let atom_index = tokens[1];
        let element = tokens[2];
        let shell_token = tokens
            .iter()
            .find(|t| shell_re.is_match(t) && t.chars().any(|c| c.is_ascii_digit()));
        let trailing_ints: Vec<i64> = tokens
            .iter()
            .filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()))
            .filter_map(|t| t.parse::<i64>().ok())
            .collect();
        if trailing_ints.len() < 2 {
            bail!("无法解析 l/m: {}", lines[idx - 1]);
        }
        let l_val = trailing_ints[trailing_ints.len() - 2];
        let m_val = trailing_ints[trailing_ints.len() - 1];

        let orbital = orbital_from_lm(l_val, m_val);
        let label = match shell_token {
            Some(st) => {
                let shell_label = st.to_lowercase();
                let base_letter = shell_label.chars().last().unwrap_or(' ');
                let suffix = if orbital.starts_with(base_letter) && base_letter != ' ' {
                    orbital[base_letter.len_utf8()..].to_string()
                } else {
                    orbital.clone()
                };
                format!("{element}{atom_index}-{shell_label}{suffix}")
            }
            None => format!("{element}{atom_index}-{orbital}"),
        };
        labels.push(label);

        for _ in 0..(nk * nbnd) {
            if idx >= lines.len() {
                bail!("projwfc 文件数据长度不足。");
            }
            let parts: Vec<&str> = lines[idx].split_whitespace().collect();
            idx += 1;
            if parts.len() < 3 {
                continue;
            }
            let k_idx: usize = parts[0].parse::<usize>().context("projwfc k 索引解析失败")? - 1;
            let b_idx: usize = parts[1].parse::<usize>().context("projwfc 带索引解析失败")? - 1;
            let val: f64 = parts[2].parse().context("projwfc 权重解析失败")?;
            if k_idx < nk && b_idx < nbnd {
                weights[proj_idx][k_idx * nbnd + b_idx] = val;
            }
        }
    }

    Ok(Some(ProjwfcData {
        labels,
        weights,
        nk,
        nbnd,
    }))
}

/// 解析 `elebanddata.dat`（bands.x 输出），返回 (k 坐标, 能量[带][k], nbnd, nks)。
pub fn parse_elebanddata(path: &str) -> Result<(Vec<[f64; 3]>, Vec<Vec<f64>>, usize, usize)> {
    if !Path::new(path).exists() {
        bail!("找不到能带数据文件: {path}");
    }
    let text = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.trim().is_empty())
        .collect();

    let header_re = Regex::new(r"nbnd\s*=\s*(\d+).+nks\s*=\s*(\d+)").unwrap();
    let header = lines.first().copied().unwrap_or("");
    let caps = header_re
        .captures(header)
        .context("无法从 elebanddata.dat 解析 nbnd/nks。")?;
    let nbnd: usize = caps[1].parse().context("nbnd 解析失败")?;
    let nks: usize = caps[2].parse().context("nks 解析失败")?;

    let mut coords = Vec::new();
    let mut energies: Vec<Vec<f64>> = Vec::new();
    let mut idx = 1usize;
    while idx < lines.len() && coords.len() < nks {
        let parts: Vec<&str> = lines[idx].split_whitespace().collect();
        if parts.len() >= 3 && parts[..3].iter().all(|p| is_float(p)) {
            coords.push([
                parts[0].parse().unwrap(),
                parts[1].parse().unwrap(),
                parts[2].parse().unwrap(),
            ]);
            idx += 1;
            let mut band_vals: Vec<f64> = Vec::with_capacity(nbnd);
            while idx < lines.len() && band_vals.len() < nbnd {
                let vals: Vec<f64> = lines[idx]
                    .split_whitespace()
                    .filter(|v| is_float(v))
                    .filter_map(|v| v.parse::<f64>().ok())
                    .collect();
                band_vals.extend(vals);
                idx += 1;
            }
            band_vals.truncate(nbnd);
            energies.push(band_vals);
        } else {
            idx += 1;
        }
    }
    if coords.len() != nks || energies.len() != nks {
        bail!("elebanddata.dat 解析的 k 点数量与头部不一致。");
    }
    Ok((coords, energies, nbnd, nks))
}

// ---------------------------------------------------------------------------
// eband.in 读取（晶胞 / K_POINTS crystal_b）
// ---------------------------------------------------------------------------

/// 从 eband.in 读 CELL_PARAMETERS；缺段时回退同目录 POSCAR（含 scale）。
pub fn read_cell_from_pw_input(path: &str) -> Result<[[f64; 3]; 3]> {
    let text = std::fs::read_to_string(path).map_err(|_| anyhow::anyhow!("找不到 eband.in: {path}"))?;
    let lines: Vec<&str> = text.lines().collect();
    for idx in 0..lines.len() {
        if lines[idx].trim().to_uppercase().starts_with("CELL_PARAMETERS") {
            let mut cell = [[0.0_f64; 3]; 3];
            for (offset, row) in cell.iter_mut().enumerate() {
                if idx + 1 + offset >= lines.len() {
                    bail!("CELL_PARAMETERS 段不完整。");
                }
                let parts: Vec<&str> = lines[idx + 1 + offset].split_whitespace().collect();
                if parts.len() < 3 {
                    bail!("CELL_PARAMETERS 行格式错误。");
                }
                for (c, part) in row.iter_mut().zip(&parts) {
                    *c = part.parse().context("CELL_PARAMETERS 分量解析失败")?;
                }
            }
            return Ok(cell);
        }
    }
    let poscar_path = Path::new(path)
        .parent()
        .map(|p| p.join("POSCAR"))
        .unwrap_or_else(|| Path::new("POSCAR").to_path_buf());
    if poscar_path.exists() {
        let ptext = std::fs::read_to_string(&poscar_path)?;
        let plines: Vec<&str> = ptext.lines().collect();
        if plines.len() < 5 {
            bail!("POSCAR 行数不足: {}", poscar_path.display());
        }
        let scale: f64 = plines[1]
            .trim()
            .parse()
            .context("POSCAR scale 解析失败")?;
        let mut cell = [[0.0_f64; 3]; 3];
        for (row, line) in cell.iter_mut().zip(&plines[2..5]) {
            for (c, part) in row.iter_mut().zip(line.split_whitespace()) {
                *c = part.parse::<f64>().context("POSCAR 晶格解析失败")? * scale;
            }
        }
        return Ok(cell);
    }
    bail!("eband.in 中未找到 CELL_PARAMETERS 段，且同目录没有 POSCAR。")
}

fn read_crystal_b_kpoints(path: &str) -> Result<(Vec<String>, usize)> {
    if !Path::new(path).exists() {
        bail!("找不到 eband.in: {path}");
    }
    let text = std::fs::read_to_string(path)?;
    let mut kline = None;
    for (i, line) in text.lines().enumerate() {
        if line.trim().to_uppercase().starts_with("K_POINTS") {
            if !line.to_uppercase().contains("CRYSTAL_B") {
                bail!("K_POINTS 段不是 crystal_b 格式。");
            }
            kline = Some(i);
            break;
        }
    }
    let Some(kline) = kline else {
        bail!("eband.in 中未找到 K_POINTS 段。");
    };
    let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    Ok((lines, kline))
}

fn read_kpoint_count(lines: &[String], start: usize) -> Result<(usize, usize)> {
    let idx = next_nonempty(lines, start);
    if idx >= lines.len() {
        bail!("K_POINTS 段缺少点数。");
    }
    let first = lines[idx]
        .split_whitespace()
        .next()
        .context("K_POINTS 点数解析失败。")?;
    let n: usize = first.parse().map_err(|_| anyhow::anyhow!("K_POINTS 点数解析失败。"))?;
    Ok((idx + 1, n))
}

/// 读 crystal_b 高对称点（含 `!` 后的标签）。
pub fn read_kpath_points_from_pw_input(path: &str) -> Result<(Vec<[f64; 3]>, Vec<String>)> {
    let (lines, kline) = read_crystal_b_kpoints(path)?;
    let (mut idx, num) = read_kpoint_count(&lines, kline + 1)?;
    let mut points = Vec::with_capacity(num);
    let mut labels = Vec::with_capacity(num);
    for _ in 0..num {
        idx = next_nonempty(&lines, idx);
        if idx >= lines.len() {
            bail!("K_POINTS 段数据不足。");
        }
        let line = lines[idx].as_str();
        idx += 1;
        let (coord_part, label) = match line.split_once('!') {
            Some((c, l)) => (c, l.trim().to_string()),
            None => (line, String::new()),
        };
        let parts: Vec<&str> = coord_part.split_whitespace().collect();
        if parts.len() < 3 {
            bail!("K_POINTS 行格式错误: {line}");
        }
        points.push([
            parts[0].parse().context("K_POINTS 坐标解析失败")?,
            parts[1].parse().context("K_POINTS 坐标解析失败")?,
            parts[2].parse().context("K_POINTS 坐标解析失败")?,
        ]);
        labels.push(label);
    }
    Ok((points, labels))
}

/// 读 crystal_b 点与整数权重。
pub fn read_kpoints_weights_from_pw_input(path: &str) -> Result<(Vec<[f64; 3]>, Vec<i64>)> {
    let (lines, kline) = read_crystal_b_kpoints(path)?;
    let (mut idx, num) = read_kpoint_count(&lines, kline + 1)?;
    let mut points = Vec::with_capacity(num);
    let mut weights = Vec::with_capacity(num);
    for _ in 0..num {
        idx = next_nonempty(&lines, idx);
        if idx >= lines.len() {
            bail!("K_POINTS 段数据不足。");
        }
        let line = lines[idx].as_str();
        idx += 1;
        let coord_part = line.split_once('!').map(|(c, _)| c).unwrap_or(line);
        let parts: Vec<&str> = coord_part.split_whitespace().collect();
        if parts.len() < 4 {
            bail!("K_POINTS 行格式错误: {line}");
        }
        let weight: f64 = parts[3]
            .parse()
            .map_err(|_| anyhow::anyhow!("K_POINTS 权重解析失败: {line}"))?;
        points.push([
            parts[0].parse().context("K_POINTS 坐标解析失败")?,
            parts[1].parse().context("K_POINTS 坐标解析失败")?,
            parts[2].parse().context("K_POINTS 坐标解析失败")?,
        ]);
        weights.push(weight as i64);
    }
    Ok((points, weights))
}

// ---------------------------------------------------------------------------
// k 路径插值与距离
// ---------------------------------------------------------------------------

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// 倒格子（2π × 叉乘公式，与 Python `get_recip_lattice` 一致）。
pub fn recip_lattice_2pi(cell: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let vol = dot3(cell[0], cross(cell[1], cell[2]));
    [
        [
            2.0 * std::f64::consts::PI * cross(cell[1], cell[2])[0] / vol,
            2.0 * std::f64::consts::PI * cross(cell[1], cell[2])[1] / vol,
            2.0 * std::f64::consts::PI * cross(cell[1], cell[2])[2] / vol,
        ],
        [
            2.0 * std::f64::consts::PI * cross(cell[2], cell[0])[0] / vol,
            2.0 * std::f64::consts::PI * cross(cell[2], cell[0])[1] / vol,
            2.0 * std::f64::consts::PI * cross(cell[2], cell[0])[2] / vol,
        ],
        [
            2.0 * std::f64::consts::PI * cross(cell[0], cell[1])[0] / vol,
            2.0 * std::f64::consts::PI * cross(cell[0], cell[1])[1] / vol,
            2.0 * std::f64::consts::PI * cross(cell[0], cell[1])[2] / vol,
        ],
    ]
}

/// 按 crystal_b 权重段内插值生成 k 列表（`generate_klist`）。
pub fn generate_klist(points: &[[f64; 3]], weights: &[i64]) -> Result<Vec<[f64; 3]>> {
    if points.len() != weights.len() {
        bail!("K_POINTS 点与权重数量不一致。");
    }
    let mut klist = Vec::new();
    for i in 0..points.len() - 1 {
        let start = points[i];
        let end = points[i + 1];
        let num = weights[i];
        if num < 1 {
            bail!("K_POINTS 权重必须为正整数。");
        }
        for j in 0..num {
            let frac = j as f64 / num as f64;
            klist.push([
                start[0] + (end[0] - start[0]) * frac,
                start[1] + (end[1] - start[1]) * frac,
                start[2] + (end[2] - start[2]) * frac,
            ]);
        }
    }
    klist.push(*points.last().context("K_POINTS 为空")?);
    Ok(klist)
}

fn metric_times(metric: &[[f64; 3]; 3], delta: [f64; 3]) -> f64 {
    let m0 = [metric[0][0], metric[0][1], metric[0][2]];
    let m1 = [metric[1][0], metric[1][1], metric[1][2]];
    let m2 = [metric[2][0], metric[2][1], metric[2][2]];
    dot3(delta, [dot3(m0, delta), dot3(m1, delta), dot3(m2, delta)])
}

fn median(values: &mut Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len();
    if n == 0 {
        0.0
    } else if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    }
}

/// 累计 k 路径距离（`compute_kpath_dist`）。`weights` 为 Some 时按权重 1 的跳变点处理。
pub fn compute_kpath_dist(
    coords: &[[f64; 3]],
    cell: [[f64; 3]; 3],
    weights: Option<&[i64]>,
) -> Vec<f64> {
    let recip = recip_lattice_2pi(cell);
    // metric = recip @ recip.T
    let mut metric = [[0.0_f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            metric[i][j] = dot3(recip[i], recip[j]);
        }
    }
    let mut dist = vec![0.0_f64];
    let jump_points: std::collections::HashSet<usize> = match weights {
        Some(w) => {
            let mut indices = vec![1usize];
            for i in 1..w.len() {
                indices.push(indices[i - 1] + w[i - 1] as usize);
            }
            (1..indices.len())
                .filter(|&i| w[i - 1] == 1)
                .map(|i| indices[i])
                .collect()
        }
        None => std::collections::HashSet::new(),
    };

    match weights {
        None => {
            let mut steps = Vec::new();
            for i in 1..coords.len() {
                let delta = sub(coords[i], coords[i - 1]);
                steps.push(metric_times(&metric, delta).sqrt());
            }
            let nonzero: Vec<f64> = steps.iter().copied().filter(|s| *s > 1.0e-10).collect();
            let median_step = median(&mut nonzero.clone());
            let jump_threshold = if median_step > 0.0 { median_step * 5.0 } else { f64::INFINITY };
            for i in 1..coords.len() {
                let delta = sub(coords[i], coords[i - 1]);
                let step = metric_times(&metric, delta).sqrt();
                if step > jump_threshold {
                    dist.push(dist[i - 1]);
                    continue;
                }
                dist.push(dist[i - 1] + step);
            }
        }
        Some(_) => {
            for i in 1..coords.len() {
                if jump_points.contains(&(i + 1)) {
                    dist.push(dist[i - 1]);
                    continue;
                }
                let delta = sub(coords[i], coords[i - 1]);
                dist.push(dist[i - 1] + metric_times(&metric, delta).sqrt());
            }
        }
    }
    dist
}

// ---------------------------------------------------------------------------
// 通道（元素/壳层分波）组织：build_channels / build_sum_channels / order_channels
// ---------------------------------------------------------------------------

pub struct ParsedLabel {
    pub elem: String,
    pub atom_index: i64,
    pub shell_num: i64,
    pub base_letter: char,
    pub suffix: String,
}

/// `parse_label`: `^([A-Z][a-z]?)(\d+)?-(\d+)([spdf])(.+)?$`
pub fn parse_label(label: &str) -> Option<ParsedLabel> {
    let re = Regex::new(r"^([A-Z][a-z]?)(\d+)?-(\d+)([spdf])(.+)?$").unwrap();
    let caps = re.captures(label)?;
    Some(ParsedLabel {
        elem: caps[1].to_string(),
        atom_index: caps.get(2).map(|m| m.as_str().parse().unwrap()).unwrap_or(0),
        shell_num: caps[3].parse().ok()?,
        base_letter: caps[4].chars().next()?,
        suffix: caps.get(5).map(|m| m.as_str().to_string()).unwrap_or_default(),
    })
}

fn base_order_index(base: char) -> usize {
    ["s", "p", "d", "f"]
        .iter()
        .position(|&b| b == base.to_string().as_str())
        .unwrap_or(0)
}

fn suffix_orders(base: char) -> &'static [&'static str] {
    match base {
        's' => &[""],
        'p' => &["x", "y", "z"],
        'd' => &["z2", "xz", "yz", "x2-y2", "xy"],
        'f' => &["z3", "xz2", "yz2", "xyz", "x2y2", "y3x2-y2", "zx2-y2"],
        _ => &[],
    }
}

/// `order_channels`：解析得到的标签按 (elem, atom, shell, base) 排序，
/// 后缀按 p/d/f 惯例排序，并在组标签存在时插入 `__sum` 通道。
pub fn order_channels(labels: &[String], group_map: &HashMap<String, Vec<String>>) -> Vec<String> {
    // grouped: 有序去重的 key 列表 + 每个 key 的 (suffix, label)。
    let mut key_order: Vec<Option<(String, i64, i64, char)>> = Vec::new();
    let mut grouped: HashMap<Option<(String, i64, i64, char)>, Vec<(String, String)>> = HashMap::new();
    for label in labels {
        let key = parse_label(label).map(|p| (p.elem, p.atom_index, p.shell_num, p.base_letter));
        let entry = grouped.entry(key.clone()).or_default();
        if !entry.iter().any(|(_, l)| l == label) {
            entry.push((parse_label(label).map(|p| p.suffix).unwrap_or_default(), label.clone()));
        }
        if !key_order.contains(&key) {
            key_order.push(key);
        }
    }

    let mut ordered = Vec::new();
    let mut tuple_keys: Vec<(String, i64, i64, char)> = key_order
        .iter()
        .filter_map(|k| k.clone())
        .collect();
    tuple_keys.sort_by(|a, b| {
        (&a.0, a.1, a.2, base_order_index(a.3)).cmp(&(&b.0, b.1, b.2, base_order_index(b.3)))
    });
    for (elem, atom_index, shell_num, base_letter) in tuple_keys {
        let key = Some((elem.clone(), atom_index, shell_num, base_letter));
        let suffix_labels: HashMap<&str, &String> = grouped
            .get(&key)
            .map(|v| v.iter().map(|(s, l)| (s.as_str(), l)).collect())
            .unwrap_or_default();
        for suffix in suffix_orders(base_letter) {
            if let Some(label) = suffix_labels.get(suffix) {
                ordered.push((*label).clone());
            }
        }
        let group_label = if atom_index == 0 {
            format!("{elem}-{shell_num}{base_letter}")
        } else {
            format!("{elem}{atom_index}-{shell_num}{base_letter}")
        };
        if group_map.contains_key(&group_label) {
            ordered.push(format!("{group_label}__sum"));
        }
    }
    for key in &key_order {
        if key.is_none() {
            if let Some(entries) = grouped.get(&None) {
                for (_, label) in entries {
                    ordered.push(label.clone());
                }
            }
        }
    }
    ordered
}

/// `build_channels`：p/d/f 分量按原子分组。
pub fn build_channels(labels: &[String]) -> (Vec<String>, HashMap<String, Vec<String>>) {
    let mut group_map: HashMap<String, Vec<String>> = HashMap::new();
    for label in labels {
        let Some(p) = parse_label(label) else { continue };
        if !matches!(p.base_letter, 'p' | 'd' | 'f') {
            continue;
        }
        let group = format!("{}{}-{}{}", p.elem, p.atom_index, p.shell_num, p.base_letter);
        group_map.entry(group).or_default().push(label.clone());
    }
    let channels = order_channels(labels, &group_map);
    (channels, group_map)
}

pub struct SumChannels {
    pub channels: Vec<String>,
    /// sum 标签 -> 成员原始标签
    pub sum_map: HashMap<String, Vec<String>>,
    /// sum 标签（去 __sum）-> 成员 sum 标签
    pub group_map: HashMap<String, Vec<String>>,
}

/// `build_sum_channels`：按元素+壳层汇总。
pub fn build_sum_channels(labels: &[String]) -> SumChannels {
    let mut sum_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut ordered: Vec<String> = Vec::new();
    for label in labels {
        let Some(p) = parse_label(label) else { continue };
        let sum_label = format!("{}-{}{}{}", p.elem, p.shell_num, p.base_letter, p.suffix);
        if !sum_map.contains_key(&sum_label) {
            sum_map.insert(sum_label.clone(), Vec::new());
            ordered.push(sum_label.clone());
        }
        sum_map.get_mut(&sum_label).unwrap().push(label.clone());
    }
    let mut group_map: HashMap<String, Vec<String>> = HashMap::new();
    for label in &ordered {
        let Some(p) = parse_label(label) else { continue };
        if !matches!(p.base_letter, 'p' | 'd' | 'f') {
            continue;
        }
        let group = format!("{}-{}{}", p.elem, p.shell_num, p.base_letter);
        group_map.entry(group).or_default().push(label.clone());
    }
    let channels = order_channels(&ordered, &group_map);
    SumChannels {
        channels,
        sum_map,
        group_map,
    }
}

// ---------------------------------------------------------------------------
// eledos 专用：tdos/pdos 解析
// ---------------------------------------------------------------------------

pub struct TdosData {
    pub energy: Vec<f64>,
    pub tdos: Vec<f64>,
    pub itdos: Vec<f64>,
    pub fermi: Option<f64>,
}

/// 解析 dos.x 的 TDOS 文件（含 `# EFermi=` 头）。
pub fn parse_tdos_with_fermi(path: &str) -> Result<TdosData> {
    let text = std::fs::read_to_string(path)?;
    let mut energy = Vec::new();
    let mut tdos = Vec::new();
    let mut itdos = Vec::new();
    let mut fermi = None;
    let re = Regex::new(r"EFermi\s*=\s*([-\d.]+)").unwrap();
    for line in text.lines() {
        if line.trim().starts_with('#') {
            if let Some(c) = re.captures(line) {
                fermi = c[1].parse().ok();
            }
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && is_float(parts[0]) {
            energy.push(parts[0].parse().unwrap());
            tdos.push(parts[1].parse().unwrap());
            itdos.push(parts[2].parse().unwrap());
        }
    }
    if energy.is_empty() {
        bail!("未从 {path} 解析到 TDOS 数据。");
    }
    Ok(TdosData {
        energy,
        tdos,
        itdos,
        fermi,
    })
}

/// `parse_projwfc_shells`: 原子序号 -> 壳层标签（如 "2s"、"2p"）列表。
pub fn parse_projwfc_shells(path: &str) -> HashMap<usize, Vec<String>> {
    let mut mapping: HashMap<usize, Vec<String>> = HashMap::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return mapping;
    };
    let re = Regex::new(r"^\s*\d+\s+(\d+)\s+([A-Za-z]{1,2})\s+(\d+)([SPDF])\s+").unwrap();
    for line in text.lines() {
        let Some(c) = re.captures(line) else { continue };
        let atom_index: usize = c[1].parse().unwrap();
        let shell_label = format!("{}{}", &c[3], c[4].to_lowercase());
        let entry = mapping.entry(atom_index).or_default();
        if !entry.contains(&shell_label) {
            entry.push(shell_label);
        }
    }
    mapping
}

/// `parse_pdos_filename`: `pdos_atm#N(Elem)_wfc#M(l)`。
pub fn parse_pdos_filename(path: &str) -> Option<(usize, String, usize, String)> {
    let name = Path::new(path).file_name()?.to_string_lossy();
    let re = Regex::new(r"pdos_atm#(\d+)\(([A-Za-z]{1,2})\)_wfc#(\d+)\(([spdf])\)").unwrap();
    let c = re.captures(&name)?;
    Some((
        c[1].parse().ok()?,
        c[2].to_string(),
        c[3].parse().ok()?,
        c[4].to_string(),
    ))
}

/// `find_first_file`：按模式顺序找第一个匹配（fullmatch，字典序最小）。
pub fn find_first_file(directory: &str, patterns: &[&str]) -> Option<String> {
    let entries = std::fs::read_dir(directory).ok()?;
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(name) = names.iter().find(|n| re.is_match(n)) {
            return Some(format!("{directory}/{name}"));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// 输出表格式化（与 Python f-string 逐字段一致）
// ---------------------------------------------------------------------------

/// Python `{:.6e}` 风格的科学计数（指数两位、带符号）。
pub fn fmt_e6(x: f64) -> String {
    if x == 0.0 {
        return "0.000000e+00".to_string();
    }
    let s = format!("{:.6e}", x);
    // Rust: "1.234500e4" / "-1.234500e-4" → 补齐为 "1.234500e+04"。
    let (mantissa, exp) = s.split_once('e').unwrap();
    let e: i32 = exp.parse().unwrap();
    format!("{mantissa}e{}{:02}", if e < 0 { '-' } else { '+' }, e.abs())
}

fn write_proj_table(
    path: &str,
    energy: &[f64],
    channels: &[String],
    resolve: &dyn Fn(&str, usize) -> f64,
    total_values: &[f64],
    fermi: Option<f64>,
    total_label: &str,
) -> Result<()> {
    let labels: Vec<String> = std::iter::once("Energy".to_string())
        .chain(channels.iter().map(|c| c.replace("__sum", "")))
        .chain(std::iter::once(total_label.to_string()))
        .collect();
    let width = labels
        .iter()
        .map(|l| l.len())
        .max()
        .unwrap_or(10)
        .max(10)
        + 2;
    let width = width.max(12);
    let mut out = String::new();
    if let Some(f) = fermi {
        out.push_str(&format!("# EFermi = {:.6} eV\n", f));
    }
    out.push('#');
    for label in &labels {
        out.push_str(&format!("{label:>width$}"));
    }
    out.push('\n');
    for (i, e) in energy.iter().enumerate() {
        let mut row = format!("{:>width$}", fmt_e6(*e));
        for c in channels {
            row.push_str(&format!("{:>width$}", fmt_e6(resolve(c, i))));
        }
        row.push_str(&format!("{:>width$}", fmt_e6(total_values[i])));
        out.push_str(&row);
        out.push('\n');
    }
    std::fs::write(path, out)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 命令入口
// ---------------------------------------------------------------------------

fn guess_prefix_from_filename(path: &str) -> String {
    let mut name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    for suffix in [
        "_band.projwfc_up",
        "_band.projwfc_down",
        "_band.projwfc",
        "_band.dat",
    ] {
        if name.ends_with(suffix) {
            name.truncate(name.len() - suffix.len());
            break;
        }
    }
    match name.split_once('.') {
        Some((prefix, _)) => prefix.to_string(),
        None => name,
    }
}

fn normalize_band_prefix(prefix: &str) -> String {
    if let Some(stripped) = prefix.strip_suffix("_band_proj") {
        return stripped.to_string();
    }
    if let Some(stripped) = prefix.strip_suffix("_proj") {
        return stripped.to_string();
    }
    prefix.to_string()
}

/// `qe_process_eband` 主逻辑：生成投影能带三张表。
#[allow(clippy::too_many_arguments)]
pub fn run_process_eband(
    input_path: &str,
    bandfile: &str,
    cell_file: &str,
    output: Option<&str>,
    prefix_override: Option<&str>,
) -> Result<Vec<String>> {
    if !Path::new(input_path).exists() {
        bail!("找不到输入文件: {input_path}");
    }
    let proj = parse_projwfc_file(input_path)?.context("未识别 projwfc 投影文件格式。")?;
    let (coords, energies, nbnd_band, nks_band) = parse_elebanddata(bandfile)?;
    let _ = coords;
    if proj.nk != nks_band || proj.nbnd != nbnd_band {
        bail!(
            "投影文件与能带文件尺寸不一致: projwfc(nk={}, nbnd={}) vs bands(nk={}, nbnd={})",
            proj.nk,
            proj.nbnd,
            nks_band,
            nbnd_band
        );
    }

    let cell = read_cell_from_pw_input(cell_file)?;
    let (_kpath_points, _kpath_labels) = read_kpath_points_from_pw_input(cell_file)?;
    let (kpoints, weights) = read_kpoints_weights_from_pw_input(cell_file)?;
    let klist = generate_klist(&kpoints, &weights)?;
    if klist.len() != nks_band {
        bail!(
            "K_POINTS 生成的 k 点数({})与 bands 文件不一致(nks={})",
            klist.len(),
            nks_band
        );
    }
    let kpath_dist = compute_kpath_dist(&klist, cell, Some(&weights));
    let (channels, group_map) = build_channels(&proj.labels);
    let proj_index: HashMap<&String, usize> =
        proj.labels.iter().enumerate().map(|(i, l)| (l, i)).collect();

    let prefix = prefix_override
        .map(|s| s.to_string())
        .unwrap_or_else(|| guess_prefix_from_filename(input_path));
    let output_path = output
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{prefix}_eleband_proj.dat"));
    let sum_output_path = format!("{prefix}_eleband_proj_sum.dat");
    let band_prefix = normalize_band_prefix(&prefix);
    let band_output_path = format!("{band_prefix}_band.dat");

    let col_width = channels.iter().map(|c| c.len()).max().unwrap_or(8).max(12);
    let resolve = |channel: &str, k: usize, b: usize| -> f64 {
        if let Some(base) = channel.strip_suffix("__sum") {
            group_map
                .get(base)
                .map(|members| {
                    members
                        .iter()
                        .filter_map(|l| proj_index.get(l))
                        .map(|&pi| proj.weights[pi][k * proj.nbnd + b])
                        .sum()
                })
                .unwrap_or(0.0)
        } else {
            proj_index
                .get(&channel.to_string())
                .map(|&pi| proj.weights[pi][k * proj.nbnd + b])
                .unwrap_or(0.0)
        }
    };

    // 表 1：投影通道
    let mut out = String::new();
    out.push_str(&format!("# source = {input_path}\n"));
    out.push_str(&format!("# bands = {bandfile}\n"));
    out.push_str(&format!("# cell = {cell_file}\n"));
    out.push_str(&format!("{:>12} {:>14}", "kpath_dist", "Energy(eV)"));
    for name in &channels {
        out.push_str(&format!(" {:>width$}", name.replace("__sum", ""), width = col_width));
    }
    out.push('\n');
    for band_idx in 0..proj.nbnd {
        for k_idx in 0..proj.nk {
            let energy = energies[k_idx][band_idx];
            out.push_str(&format!("{:>12.6} {:>14.6} ", kpath_dist[k_idx], energy));
            let cols: Vec<String> = channels
                .iter()
                .map(|c| format!("{:>width$.6}", resolve(c, k_idx, band_idx), width = col_width))
                .collect();
            out.push_str(&cols.join(" "));
            out.push('\n');
        }
        out.push_str(&format!("#band={}\n\n", band_idx + 1));
    }
    std::fs::write(&output_path, &out)?;

    // 表 2：元素/壳层汇总
    let sum = build_sum_channels(&proj.labels);
    let sum_col_width = sum.channels.iter().map(|c| c.len()).max().unwrap_or(8).max(12);
    let mut out = String::new();
    out.push_str(&format!("# source = {input_path}\n"));
    out.push_str(&format!("# bands = {bandfile}\n"));
    out.push_str(&format!("# cell = {cell_file}\n"));
    out.push_str("# summed by element and shell (e.g., Y-4s)\n");
    out.push_str(&format!("{:>12} {:>14}", "kpath_dist", "Energy(eV)"));
    for name in &sum.channels {
        out.push_str(&format!(
            " {:>width$}",
            name.replace("__sum", ""),
            width = sum_col_width
        ));
    }
    out.push('\n');
    for band_idx in 0..proj.nbnd {
        for k_idx in 0..proj.nk {
            let energy = energies[k_idx][band_idx];
            out.push_str(&format!("{:>12.6} {:>14.6} ", kpath_dist[k_idx], energy));
            let cols: Vec<String> = sum
                .channels
                .iter()
                .map(|channel| {
                    let weight = if let Some(base) = channel.strip_suffix("__sum") {
                        sum.group_map
                            .get(base)
                            .map(|members| {
                                members
                                    .iter()
                                    .filter_map(|l| sum.sum_map.get(l))
                                    .flatten()
                                    .filter_map(|bl| proj_index.get(bl))
                                    .map(|&pi| proj.weights[pi][k_idx * proj.nbnd + band_idx])
                                    .sum::<f64>()
                            })
                            .unwrap_or(0.0)
                    } else {
                        sum.sum_map
                            .get(channel)
                            .map(|members| {
                                members
                                    .iter()
                                    .filter_map(|bl| proj_index.get(bl))
                                    .map(|&pi| proj.weights[pi][k_idx * proj.nbnd + band_idx])
                                    .sum::<f64>()
                            })
                            .unwrap_or(0.0)
                    };
                    format!("{:>width$.6}", weight, width = sum_col_width)
                })
                .collect();
            out.push_str(&cols.join(" "));
            out.push('\n');
        }
        out.push_str(&format!("#band={}\n\n", band_idx + 1));
    }
    std::fs::write(&sum_output_path, &out)?;

    // 表 3：纯能带
    let mut out = String::new();
    out.push_str(&format!("# source = {bandfile}\n"));
    out.push_str(&format!("# cell = {cell_file}\n"));
    out.push_str("# kpath_dist energy\n");
    for band_idx in 0..proj.nbnd {
        for k_idx in 0..proj.nk {
            let energy = energies[k_idx][band_idx];
            out.push_str(&format!("{:>12.6} {:>14.6}\n", kpath_dist[k_idx], energy));
        }
        out.push_str(&format!("#band={}\n\n", band_idx + 1));
    }
    std::fs::write(&band_output_path, &out)?;

    Ok(vec![
        output_path,
        sum_output_path,
        band_output_path,
    ])
}

/// 梯形累积积分（`cumulative_integral`）。
pub fn cumulative_integral(energy: &[f64], values: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0_f64; values.len()];
    for i in 1..values.len() {
        let de = energy[i] - energy[i - 1];
        out[i] = out[i - 1] + 0.5 * (values[i] + values[i - 1]) * de;
    }
    out
}

/// `qe_process_eledos` 主逻辑：生成 DOS/IDOS 四张表。
pub fn run_process_eledos(input: &str, proj_arg: Option<&str>) -> Result<Vec<String>> {
    let input_abs = input.to_string();
    let (work_dir, tdos_path) = if Path::new(&input_abs).is_dir() {
        let dir = input_abs.trim_end_matches('/').to_string();
        match find_first_file(&dir, &[r".+\.tdos"]) {
            Some(p) => (dir, p),
            None => bail!("未找到 *.tdos 文件。"),
        }
    } else {
        let dir = Path::new(&input_abs)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| ".".to_string());
        if input_abs.ends_with(".tdos") {
            (dir, input_abs.clone())
        } else {
            match find_first_file(&dir, &[r".+\.tdos"]) {
                Some(p) => (dir, p),
                None => bail!("未找到 *.tdos 文件。"),
            }
        }
    };

    let proj_path = match proj_arg {
        Some(p) => p.to_string(),
        None => find_first_file(
            &work_dir,
            &[
                r".+\.projwfc_up",
                r".+\.projwfc_down",
                r".+\.projwfc",
                r".+\.proj",
            ],
        )
        .unwrap_or_default(),
    };

    let tdos_data = parse_tdos_with_fermi(&tdos_path)?;
    let shell_map = parse_projwfc_shells(&proj_path);

    let mut pdos_files: Vec<String> = std::fs::read_dir(&work_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| {
            Regex::new(r"pdos_atm#\d+\([^)]+\)_wfc#\d+\([spdf]\)")
                .unwrap()
                .is_match(n)
        })
        .collect();
    pdos_files.sort();
    if pdos_files.is_empty() {
        bail!("未找到 pdos_atm 文件。");
    }

    // data 用有序容器保持插入顺序（与 Python dict 一致）。
    let mut data: Vec<(String, Vec<f64>)> = Vec::new();
    for filename in &pdos_files {
        let file_path = format!("{work_dir}/{filename}");
        let Some((atom_index, element, wfc_index, l_letter)) = parse_pdos_filename(&file_path)
        else {
            continue;
        };
        let shell_label = shell_map
            .get(&atom_index)
            .and_then(|shells| shells.get(wfc_index.saturating_sub(1)))
            .cloned()
            .unwrap_or_else(|| format!("{wfc_index}{l_letter}"));
        let shell_re = Regex::new(r"^(\d+)([spdf])").unwrap();
        let (shell_num, _base_letter) = match shell_re.captures(&shell_label) {
            Some(c) => (c[1].to_string(), c[2].to_string()),
            None => (wfc_index.to_string(), l_letter.clone()),
        };

        let mut energies: Vec<f64> = Vec::new();
        let mut values: Vec<Vec<f64>> = Vec::new();
        let text = std::fs::read_to_string(&file_path)?;
        for line in text.lines() {
            if line.trim().starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && is_float(parts[0]) {
                energies.push(parts[0].parse().unwrap());
                values.push(parts[2..].iter().map(|p| p.parse().unwrap()).collect());
            }
        }
        if energies.is_empty() {
            continue;
        }
        if energies.len() != tdos_data.energy.len()
            || energies
                .iter()
                .zip(&tdos_data.energy)
                .any(|(a, b)| (a - b).abs() > 1e-6)
        {
            bail!("能量网格与 TDOS 不一致: {filename}");
        }
        let components = orbital_components(&l_letter);
        if !values.is_empty() && values[0].len() < components.len() {
            bail!("{filename} 的 pdos 列数不足。");
        }
        for (idx, comp) in components.iter().enumerate() {
            let label = format!("{element}{atom_index}-{shell_num}{comp}");
            data.push((label, values.iter().map(|row| row[idx]).collect()));
        }
    }
    if data.is_empty() {
        bail!("未解析到 PDOS 数据。");
    }

    let labels: Vec<String> = data.iter().map(|(l, _)| l.clone()).collect();
    let (channels, group_map) = build_channels(&labels);
    let sum = build_sum_channels(&labels);

    let get = |name: &str| -> Vec<f64> {
        data.iter()
            .find(|(l, _)| l == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    // sum_data：保持 sum_map 插入顺序。
    let mut sum_data: Vec<(String, Vec<f64>)> = Vec::new();
    for (sum_label, members) in &sum.sum_map {
        let mut acc = vec![0.0_f64; tdos_data.energy.len()];
        for label in members {
            for (i, v) in get(label).iter().enumerate() {
                acc[i] += v;
            }
        }
        sum_data.push((sum_label.clone(), acc));
    }
    let _sum_get = |name: &str| -> Vec<f64> {
        sum_data
            .iter()
            .find(|(l, _)| l == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };

    let idos_data: Vec<(String, Vec<f64>)> = data
        .iter()
        .map(|(l, v)| (l.clone(), cumulative_integral(&tdos_data.energy, v)))
        .collect();
    let idos_sum_data: Vec<(String, Vec<f64>)> = sum_data
        .iter()
        .map(|(l, v)| (l.clone(), cumulative_integral(&tdos_data.energy, v)))
        .collect();

    let base = Path::new(&tdos_path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let prefix = base.split('.').next().unwrap_or("dos").to_string();

    let resolve_channel = |store: &[(String, Vec<f64>)],
                           channel: &str,
                           group_map: &HashMap<String, Vec<String>>,
                           idx: usize|
     -> f64 {
        if let Some(base) = channel.strip_suffix("__sum") {
            group_map
                .get(base)
                .map(|members| {
                    members
                        .iter()
                        .filter_map(|l| store.iter().find(|(sl, _)| sl == l))
                        .map(|(_, v)| v[idx])
                        .sum()
                })
                .unwrap_or(0.0)
        } else {
            store
                .iter()
                .find(|(l, _)| l == channel)
                .map(|(_, v)| v[idx])
                .unwrap_or(0.0)
        }
    };

    let dos_path = format!("{work_dir}/{prefix}_dos_proj.dat");
    let dos_sum_path = format!("{work_dir}/{prefix}_dos_proj_sum.dat");
    let idos_path = format!("{work_dir}/{prefix}_idos_proj.dat");
    let idos_sum_path = format!("{work_dir}/{prefix}_idos_proj_sum.dat");

    let gm = group_map.clone();
    write_proj_table(
        &dos_path,
        &tdos_data.energy,
        &channels,
        &|c, i| resolve_channel(&data, c, &gm, i),
        &tdos_data.tdos,
        tdos_data.fermi,
        "TDOS",
    )?;
    let sgm = sum.group_map.clone();
    write_proj_table(
        &dos_sum_path,
        &tdos_data.energy,
        &sum.channels,
        &|c, i| resolve_channel(&sum_data, c, &sgm, i),
        &tdos_data.tdos,
        tdos_data.fermi,
        "TDOS",
    )?;
    write_proj_table(
        &idos_path,
        &tdos_data.energy,
        &channels,
        &|c, i| resolve_channel(&idos_data, c, &gm, i),
        &tdos_data.itdos,
        tdos_data.fermi,
        "ITDOS",
    )?;
    write_proj_table(
        &idos_sum_path,
        &tdos_data.energy,
        &sum.channels,
        &|c, i| resolve_channel(&idos_sum_data, c, &sgm, i),
        &tdos_data.itdos,
        tdos_data.fermi,
        "ITDOS",
    )?;

    Ok(vec![dos_path, dos_sum_path, idos_path, idos_sum_path])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbital_names_match_python() {
        assert_eq!(orbital_from_lm(0, 1), "s");
        assert_eq!(orbital_from_lm(1, 2), "px");
        assert_eq!(orbital_from_lm(2, 4), "dx2-y2");
        assert_eq!(orbital_from_lm(3, 6), "fy3x2-y2");
    }

    #[test]
    fn parses_pdos_filename() {
        let (a, e, w, l) = parse_pdos_filename("dir/pdos_atm#3(Mg)_wfc#2(p)").unwrap();
        assert_eq!((a, e.as_str(), w, l.as_str()), (3, "Mg", 2, "p"));
    }

    #[test]
    fn formats_e6_like_python() {
        assert_eq!(fmt_e6(0.0), "0.000000e+00");
        assert_eq!(fmt_e6(123.456), "1.234560e+02");
        assert_eq!(fmt_e6(-0.000123), "-1.230000e-04");
    }

    #[test]
    fn channels_order_p_d_f() {
        let labels: Vec<String> = vec![
            "Mg1-2px".into(),
            "Mg1-2s".into(),
            "B1-3dxy".into(),
            "B1-3dz2".into(),
        ];
        let (channels, group_map) = build_channels(&labels);
        // s 不进组；p 在 d 前；同壳层内按 suffix 惯例排序。
        assert!(channels.contains(&"Mg1-2s".to_string()));
        assert!(channels.contains(&"Mg1-2px".to_string()));
        assert!(channels.contains(&"Mg1-2p__sum".to_string()));
        let pos = |s: &str| channels.iter().position(|c| c == s).unwrap();
        assert!(pos("Mg1-2s") < pos("Mg1-2px"));
        assert!(pos("B1-3dz2") < pos("B1-3dxy"));
        assert!(group_map.contains_key("Mg1-2p"));
        assert!(group_map.contains_key("B1-3d"));
    }

    #[test]
    fn klist_interpolates_segments() {
        let pts = vec![[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.5, 0.5, 0.0]];
        let w = vec![2, 1, 1];
        let klist = generate_klist(&pts, &w).unwrap();
        assert_eq!(klist.len(), 4);
        assert!((klist[1][0] - 0.25).abs() < 1e-12);
        assert_eq!(klist[2], [0.5, 0.0, 0.0]);
    }
}
