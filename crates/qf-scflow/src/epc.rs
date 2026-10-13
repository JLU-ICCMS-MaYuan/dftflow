//! QE EPC 输出解析（复刻 `scflow/qe_outputs.py` 的 Folder / Dyn / DynElph / PhOuts）。

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub const K_BOHR_A: f64 = 0.529177249;

fn list_sorted(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    names.sort();
    names
}

fn fullmatch(re: &regex::Regex, name: &str) -> bool {
    re.find(name).map(|m| m.as_str() == name).unwrap_or(false)
}

/// `Folder`：发现 dyn*.elph*、*.dyn* 与 *.ph.out。
pub struct Folder {
    pub path: PathBuf,
    pub dyn_paths: Vec<PathBuf>,
    pub dyn_elph_paths: Vec<PathBuf>,
    pub ph_outs_paths: Vec<PathBuf>,
}

impl Folder {
    pub fn new(path: &Path) -> Result<Self> {
        let dyn_paths = Self::matching(path, r".*dyn[1-9][0-9]*");
        println!("Found {} dynamical matrices", dyn_paths.len());

        let legacy = Self::matching(path, r".*dyn[0-9]+\.elph(?:\.?[0-9]+)?");
        let native = Self::matching(&path.join("elph_dir"), r"elph\.inp_lambda\.[1-9][0-9]*");
        if !legacy.is_empty() && !native.is_empty() {
            bail!("Both legacy and native EPC files found; select one input layout");
        }
        let dyn_elph_paths = if !legacy.is_empty() { legacy } else { native };
        if dyn_elph_paths.is_empty() {
            bail!("No QE EPC files found in {}", path.display());
        }
        println!("Found {} EPC files", dyn_elph_paths.len());

        let ph_outs_paths = Self::matching(path, r"(?:.*\.ph\.out|ph_no_split\.out|split_ph\.out\.[1-9][0-9]*)");
        Ok(Self { path: path.to_path_buf(), dyn_paths, dyn_elph_paths, ph_outs_paths })
    }

    fn matching(directory: &Path, pattern: &str) -> Vec<PathBuf> {
        let re = regex::Regex::new(pattern).unwrap();
        list_sorted(directory)
            .into_iter()
            .filter(|p| p.is_file() && fullmatch(&re, &p.file_name().unwrap_or_default().to_string_lossy()))
            .collect()
    }
}

/// `Dyn`：从 `*.dyn*` 文件取 q 点与星权重。
pub struct Dyn {
    pub path: PathBuf,
    pub q_point: [f64; 3],
    pub weight: usize,
}

impl Dyn {
    pub fn new(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        let lines: Vec<&str> = text.lines().collect();
        let mut q_point = [0.0f64; 3];
        let mut weight = 0usize;
        let mut diagonalizing = false;
        for line in &lines {
            if line.contains("Diagonalizing the dynamical matrix") {
                diagonalizing = true;
            }
            if !diagonalizing {
                if let Some(idx) = line.find("q = (") {
                    if weight == 0 {
                        // 第一处 "q = (" 行取坐标：split 后倒数第 4..1 个 token。
                        let tokens: Vec<&str> = line.split_whitespace().collect();
                        if tokens.len() >= 4 {
                            for k in 0..3 {
                                q_point[k] = parse_f64(tokens[tokens.len() - 4 + k])?;
                            }
                        }
                    }
                    weight += 1;
                    let _ = idx;
                }
            }
        }
        println!(
            "q = ({:.3}, {:.3}, {:.3}) with number of q in the star {}",
            q_point[0], q_point[1], q_point[2], weight
        );
        Ok(Self { path: path.to_path_buf(), q_point, weight })
    }
}

fn parse_f64(token: &str) -> Result<f64> {
    token
        .parse::<f64>()
        .map_err(|e| anyhow::anyhow!("数值解析失败 {token:?}: {e}"))
}

/// `DynElph`：单个 `*.dyn*.elph*` 文件的全部内容（按展宽区段切分）。
#[derive(Debug, Clone)]
pub struct DynElph {
    pub path: PathBuf,
    /// 展宽区段表头：`Gaussian Broadening` 为数值，`Tetrahedron method` 为 None。
    pub headers: Vec<Option<f64>>,
    /// 表头原文（Python `str(header)` 用于结果子目录名；Tetrahedron 为整行）。
    pub headers_text: Vec<String>,
    pub q_point: [f64; 3],
    pub sqr_freqs: Vec<f64>,
    /// 每个展宽区段的 E_F（eV）。
    pub e_f: Vec<f64>,
    /// 每个展宽区段的 DOS（states/spin/Ry）。
    pub dos: Vec<f64>,
    /// lambdas[区段][模式]。
    pub lambdas: Vec<Vec<f64>>,
    /// gammas[区段][模式]（GHz）。
    pub gammas: Vec<Vec<f64>>,
}

impl DynElph {
    pub fn new(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let lines: Vec<&str> = text.lines().collect();

        let mut headers_idx: Vec<usize> = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if line.contains("Gaussian Broadening") || line.contains("Tetrahedron method") {
                headers_idx.push(i);
            }
        }
        headers_idx.push(lines.len());

        let mut headers: Vec<Option<f64>> = Vec::new();
        let mut headers_text: Vec<String> = Vec::new();
        let mut dos_lines: Vec<&str> = Vec::new();
        let mut lambda_gamma_lines: Vec<Vec<&str>> = Vec::new();
        for w in headers_idx.windows(2) {
            let (i1, i2) = (w[0], w[1]);
            let section = &lines[i1..i2];
            let dos_line = *section
                .iter()
                .find(|l| l.contains("DOS"))
                .ok_or_else(|| anyhow::anyhow!("{} 缺少 DOS 行", path.display()))?;
            dos_lines.push(dos_line);
            lambda_gamma_lines.push(
                section
                    .iter()
                    .filter(|l| l.contains("lambda") && l.contains("gamma"))
                    .copied()
                    .collect(),
            );
            if lines[i1].contains("Gaussian Broadening") {
                let tokens: Vec<&str> = lines[i1].split_whitespace().collect();
                let value = tokens.get(2).context("Gaussian Broadening 表头缺少数值")?;
                let parsed = parse_f64(value)?;
                headers_text.push(format!("{}", parsed));
                headers.push(Some(parsed));
            } else {
                headers_text.push(lines[i1].to_string());
                headers.push(None);
            }
        }
        let length = headers.len();

        // q 点：首行前三个数值。
        let first_tokens: Vec<&str> = lines[0].split_whitespace().collect();
        if first_tokens.len() < 3 {
            bail!("{} 首行不是 q 点", path.display());
        }
        let mut q_point = [0.0f64; 3];
        for k in 0..3 {
            q_point[k] = parse_f64(first_tokens[k])?;
        }

        // 平方频率：第 2 行到第一个表头之间的全部数值。
        let mut sqr_freqs: Vec<f64> = Vec::new();
        for line in &lines[1..headers_idx[0]] {
            for token in line.split_whitespace() {
                if let Ok(v) = token.parse::<f64>() {
                    sqr_freqs.push(v);
                }
            }
        }

        let re_ef = regex::Regex::new(r"Ef(.*)eV").unwrap();
        let re_dos = regex::Regex::new(r"DOS(.*)states").unwrap();
        let mut e_f = Vec::with_capacity(length);
        let mut dos = Vec::with_capacity(length);
        for dl in &dos_lines {
            let ef = re_ef
                .captures(dl)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().replace('=', ""))
                .ok_or_else(|| anyhow::anyhow!("{} Ef 解析失败", path.display()))?;
            e_f.push(parse_f64(ef.trim())?);
            let d = re_dos
                .captures(dl)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().replace('=', ""))
                .ok_or_else(|| anyhow::anyhow!("{} DOS 解析失败", path.display()))?;
            dos.push(parse_f64(d.trim())?);
        }

        let mut lambdas: Vec<Vec<f64>> = Vec::with_capacity(length);
        let mut gammas: Vec<Vec<f64>> = Vec::with_capacity(length);
        for lg in &lambda_gamma_lines {
            let mut l = Vec::with_capacity(lg.len());
            let mut g = Vec::with_capacity(lg.len());
            for line in lg {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() < 3 {
                    bail!("{} lambda/gamma 行格式异常: {line:?}", path.display());
                }
                l.push(parse_f64(parts[1].split_whitespace().next().unwrap_or(""))?);
                g.push(parse_f64(parts[2].split_whitespace().next().unwrap_or(""))?);
            }
            lambdas.push(l);
            gammas.push(g);
        }

        // 负 λ（及其 γ）置零。
        for (i, lambdas_i) in lambdas.iter_mut().enumerate() {
            if lambdas_i.iter().any(|v| *v < 0.0) {
                if length > 1 {
                    println!(
                        "WARNING: found negative lambdas in {} (Broadening = {:?}). They and their gammas will be zeroed.",
                        path.display(),
                        headers[i]
                    );
                } else {
                    println!(
                        "WARNING: found negative lambdas in {}. They and their gammas will be zeroed.",
                        path.display()
                    );
                }
                for (l, g) in lambdas_i.iter_mut().zip(gammas[i].iter_mut()) {
                    if *l < 0.0 {
                        *l = 0.0;
                        *g = 0.0;
                    }
                }
            }
        }

        Ok(Self {
            path: path.to_path_buf(),
            headers,
            headers_text,
            q_point,
            sqr_freqs,
            e_f,
            dos,
            lambdas,
            gammas,
        })
    }
}

/// `PhOuts`：ph.out 权重与结构。
pub struct PhOuts {
    lines: Vec<Vec<String>>,
    paths: Vec<PathBuf>,
}

impl PhOuts {
    pub fn new(paths: &[PathBuf]) -> Self {
        let mut lines = Vec::new();
        if paths.len() > 1 || (paths.len() == 1 && !paths[0].is_dir()) {
            for path in paths {
                if let Ok(text) = std::fs::read_to_string(path) {
                    lines.push(text.lines().map(|l| l.to_string()).collect());
                }
            }
        }
        Self { lines, paths: paths.to_vec() }
    }

    /// (q 点, 归一化权重) 列表。
    pub fn weights(&self) -> Vec<([f64; 3], f64)> {
        let mut result = Vec::new();
        if !self.lines.is_empty() {
            for file_lines in &self.lines {
                let occupancies: Vec<usize> = file_lines
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| l.contains("Number of q in the star"))
                    .map(|(i, _)| i)
                    .collect();
                for occupancy in occupancies {
                    let q_line = &file_lines[occupancy + 2];
                    let tokens: Vec<&str> = q_line.split_whitespace().collect();
                    if tokens.len() < 3 {
                        continue;
                    }
                    let mut q = [0.0f64; 3];
                    let mut ok = true;
                    for (k, token) in tokens.iter().rev().take(3).enumerate() {
                        match token.parse::<f64>() {
                            Ok(v) => q[2 - k] = v,
                            Err(_) => ok = false,
                        }
                    }
                    if !ok {
                        continue;
                    }
                    let weight: usize = file_lines[occupancy]
                        .split_whitespace()
                        .last()
                        .and_then(|t| t.parse().ok())
                        .unwrap_or(0);
                    println!(
                        "q = ({:.3}, {:.3}, {:.3}) with number of q in the star {}",
                        q[0], q[1], q[2], weight
                    );
                    result.push((q, weight as f64));
                }
            }
        } else {
            println!("Assuming no-symmetry calculation, consequently setting uniform weights");
            let folder = Folder::new(&self.paths[0]).ok();
            let count = folder.map(|f| f.dyn_elph_paths.len()).unwrap_or(0);
            for _ in 0..count {
                result.push(([0.0; 3], 1.0));
            }
            // 无对称性情形 q 点以 dyn_elph 为准，由调用方覆盖。
            return result;
        }
        let sum: f64 = result.iter().map(|(_, w)| *w).sum();
        for (_, w) in result.iter_mut() {
            *w /= sum;
        }
        result
    }

    /// 从 ph.out 解析结构（笛卡尔坐标，Å）。
    pub fn structure(&self) -> Result<crate::structure_out::StructureOut> {
        if self.lines.is_empty() {
            bail!("没有可解析的 ph.out 文件");
        }
        let keys = [
            "lattice parameter (alat)",
            "a(1)",
            "a(2)",
            "a(3)",
            "site n.",
            "number of atoms/cell",
        ];
        let mut idxs: Vec<Option<(usize, usize)>> = vec![None; keys.len()];
        'outer: for (i, file_lines) in self.lines.iter().enumerate() {
            for (j, line) in file_lines.iter().enumerate() {
                for (k, key) in keys.iter().enumerate() {
                    if line.contains(key) && idxs[k].is_none() {
                        idxs[k] = Some((i, j));
                    }
                }
            }
            if idxs.iter().all(|x| x.is_some()) {
                break 'outer;
            }
        }
        if idxs.iter().any(|x| x.is_none()) {
            bail!("ph.out 缺少结构段关键字");
        }
        let get = |k: usize| idxs[k].unwrap();
        let (ai, aj) = get(0);
        let alat = parse_f64(
            self.lines[ai][aj]
                .split_whitespace()
                .nth(4)
                .context("alat 解析失败")?,
        )?;
        let mut matrix = [[0.0f64; 3]; 3];
        for (i, key_idx) in [get(1), get(2), get(3)].iter().enumerate() {
            let (i2, j2) = *key_idx;
            let tokens: Vec<&str> = self.lines[i2][j2].split_whitespace().collect();
            for k in 0..3 {
                matrix[i][k] = parse_f64(tokens[3 + k])? * alat;
            }
        }
        let (ni, nj) = get(5);
        let n_atoms: usize = self.lines[ni][nj]
            .split_whitespace()
            .last()
            .and_then(|t| t.parse().ok())
            .context("number of atoms/cell 解析失败")?;
        let (si, sj) = get(4);
        let mut species = Vec::with_capacity(n_atoms);
        let mut coords = Vec::with_capacity(n_atoms);
        for line in &self.lines[si][sj + 1..sj + 1 + n_atoms] {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() < 4 {
                bail!("site 行格式异常: {line}");
            }
            species.push(tokens[1].to_string());
            coords.push([
                parse_f64(tokens[tokens.len() - 4])? * alat,
                parse_f64(tokens[tokens.len() - 3])? * alat,
                parse_f64(tokens[tokens.len() - 2])? * alat,
            ]);
        }
        for p in coords.iter_mut() {
            for v in p.iter_mut() {
                *v *= K_BOHR_A;
            }
        }
        for row in matrix.iter_mut() {
            for v in row.iter_mut() {
                *v *= K_BOHR_A;
            }
        }
        Ok(crate::structure_out::StructureOut { lattice: matrix, species, coords })
    }
}
