//! EPW 5.9／6.0 原生运行证据与输出解析。谱能量轴独立于细网格声子本征频率。
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// 只读检查现有原生计算，不替代完整执行和求解收敛记录。
pub fn inspect(work: &Path) -> Result<serde_json::Value> {
    let input = fs::read_to_string(work.join("epw.in"))?;
    let prefix_re = regex::Regex::new(r"(?im)^\s*prefix\s*=\s*'([A-Za-z0-9_-]+)'\s*,?\s*$")?;
    let prefix = prefix_re
        .captures(&input)
        .context("data_invalid: Lack of support EPW prefix")?;
    let samples_re = regex::Regex::new(r"(?im)^\s*nqstep\s*=\s*([0-9]+)\s*,?\s*$")?;
    let samples: usize = samples_re
        .captures(&input)
        .context("data_invalid: Native spectroscopy needs to be visible nqstep")?[1]
        .parse()?;
    let exit_code: i32 = fs::read_to_string(work.join("epw.exitcode"))?
        .trim()
        .parse()?;
    checked_log(&fs::read_to_string(work.join("epw.out"))?, exit_code)?;
    let base_name = format!("{}.a2f", &prefix[1]);
    let pattern = format!("{base_name}.");
    let mut spectra = Vec::new();
    for entry in fs::read_dir(work)? {
        let path = entry?.path();
        if path
            .file_name()
            .is_some_and(|n| n == base_name.as_str() || n.to_string_lossy().starts_with(&pattern))
            && path.is_file()
        {
            spectra.push(path);
        }
    }
    spectra.sort();
    ensure!(
        spectra.len() == 1,
        "data_invalid: There's only one of the current originals {} Grandpa",
        spectra.len()
    );
    let spectrum = NativeSpectrum::read(&spectra[0], samples)?;
    let wout = fs::read_to_string(work.join(format!("{}.wout", &prefix[1])))?;
    ensure!(
        wout.contains("<<< Wannierisation convergence criteria satisfied >>>")
            && !regex::Regex::new(
                r"(?i)convergence\s+(?:was\s+)?not\s+(?:reached|achieved)|not\s+converged"
            )?
            .is_match(&wout),
        "data_invalid: Wannier Localization lacks evidence of pre-generic convergence"
    );
    let disentangled = regex::Regex::new(r"Using band disentanglement\s*:\s*T")?.is_match(&wout);
    ensure!(
        !disentangled || wout.contains("<<< Disentanglement convergence criteria satisfied >>>"),
        "data_invalid: Wannier The lack of evidence of abdominal indigence"
    );
    Ok(serde_json::json!({
        "schema_version": 1, "workflow_status": "native_epc_only", "workflow_complete": false,
        "native_epc_smoke_passed": true,
        "eliashberg": "unverified", "automatic_chain": "unverified",
        "samples": samples, "spectrum": spectra[0].file_name().context("data_invalid: Spectrofile name is empty")?.to_string_lossy(),
        "spectrum_sha256": super::super::foundation::store::hash_file(&spectra[0])?,
        "lambda_integral": spectrum.lambda_integral, "lambda_sum": spectrum.lambda_sum,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeSpectrum {
    pub energy_mev: Vec<f64>,
    pub a2f: Vec<Vec<f64>>,
    pub lambda_integral: Vec<f64>,
    pub lambda_sum: f64,
}

fn number(s: &str) -> Result<f64> {
    let value: f64 = s
        .replace(['D', 'd'], "E")
        .parse()
        .with_context(|| format!("data_invalid: Illegal native value {s}"))?;
    ensure!(value.is_finite(), "data_invalid: Native output is not finite");
    Ok(value)
}

/// 只认真实运行头，不认输入标题；支持范围不等于所有科学格式均已验收。
pub fn runtime_version(log: &str) -> Result<&str> {
    let header = regex::Regex::new(r"(?m)^\s*Program EPW v\.([0-9]+(?:\.[0-9]+)*)\s")?;
    let versions: Vec<_> = header.captures_iter(log)
        .map(|c| c.get(1).unwrap().as_str()).collect();
    ensure!(versions.len() == 1, "environment_incompatible: Missing Unique EPW Run Version Record");
    ensure!(matches!(versions[0], "5.9" | "6.0"),
        "environment_incompatible: Unsuited EPW Run Version {}", versions[0]);
    Ok(versions[0])
}

/// EPW 的结束计时和进程退出码均必需；它不输出 PW/PH 的 JOB DONE。
pub fn checked_log(log: &str, exit_code: i32) -> Result<()> {
    ensure!(exit_code == 0, "execution_failed: EPW Exit Code {exit_code}");
    runtime_version(log)?;
    ensure!(
        !regex::Regex::new(r"(?i)error in routine|segmentation fault|MPI_Abort")?.is_match(log),
        "execution_failed: EPW Report native error"
    );
    ensure!(
        log.contains("Total program execution")
            && regex::Regex::new(r"EPW\s*:\s*[^\n]*WALL")?.is_match(log),
        "data_invalid: EPW End timer incomplete"
    );
    Ok(())
}

impl NativeSpectrum {
    pub fn read(path: &Path, samples: usize) -> Result<Self> {
        Self::parse(&fs::read_to_string(path)?, samples)
    }

    pub fn parse(text: &str, samples: usize) -> Result<Self> {
        let (table, footer) = text
            .split_once("Integrated el-ph coupling")
            .context("data_invalid: The native spectro lacked the end of the score coupling")?;
        let header = regex::Regex::new(
            r"^\s*w\[meV\] a2f and integrated 2\*a2f/w for\s+(\d+) smearing values\s*$",
        )?;
        let mut lines = table.lines().filter(|l| !l.trim().is_empty()).peekable();
        let cumulative_columns = lines
            .peek()
            .and_then(|line| header.captures(line))
            .map(|c| c[1].parse::<usize>())
            .transpose()?;
        if cumulative_columns.is_some() {
            lines.next();
        }
        let rows: Vec<Vec<f64>> = lines
            .map(|line| line.split_whitespace().map(number).collect())
            .collect::<Result<_>>()?;
        ensure!(
            samples >= 2 && rows.len() == samples,
            "data_invalid: It doesn't match the native sample"
        );
        let table_columns = rows[0]
            .len()
            .checked_sub(1)
            .filter(|n| *n > 0)
            .context("data_invalid: There's no native α² F Columns")?;
        let columns = cumulative_columns.unwrap_or(table_columns);
        ensure!(
            columns > 0
                && table_columns == columns * if cumulative_columns.is_some() { 2 } else { 1 },
            "data_invalid: The spectrohead does not match the number of columns"
        );
        ensure!(
            rows.iter().all(|r| r.len() == table_columns + 1
                && r[0] > 0.0
                && r[1..].iter().all(|v| *v >= 0.0)),
            "data_invalid: Invalidity of ingenuity column, energy or weight"
        );
        ensure!(
            rows.windows(2).all(|r| r[0][0] < r[1][0]),
            "data_invalid: The native energy axis is not strictly increasing"
        );
        let step = rows[0][0];
        ensure!(
            rows.iter()
                .enumerate()
                .all(|(i, r)| (r[0] - (i + 1) as f64 * step).abs() <= 1e-7 * samples as f64),
            "data_invalid: The native spectra is not a flat grid starting with the first positive frequency"
        );
        let coupling = footer
            .lines()
            .find(|l| l.trim_start().starts_with('#'))
            .context("data_invalid: Lack of native score")?;
        let lambda_integral: Vec<f64> = coupling
            .split_whitespace()
            .skip(1)
            .map(number)
            .collect::<Result<_>>()?;
        ensure!(
            lambda_integral.len() == columns,
            "data_invalid: Numerical points, ⁇ , column numbers don't match"
        );
        let summed = regex::Regex::new(r"Summed el-ph coupling\s+(\S+)")?;
        let value = summed
            .captures(footer)
            .context("data_invalid: There's a lack of direct reconciliation in the native spectra λ")?;
        let lambda_sum = number(&value[1])?;
        ensure!(
            lambda_sum >= 0.0 && lambda_integral.iter().all(|v| *v >= 0.0),
            "data_invalid: Periphery negative"
        );
        let a2f: Vec<Vec<f64>> = (1..=columns)
            .map(|i| rows.iter().map(|r| r[i]).collect())
            .collect();
        for (i, integral) in lambda_integral.iter().enumerate() {
            // spectral.f90 与 supercond.f90 均使用相同的离散矩形求和。
            let reconstructed: f64 = rows.iter().map(|r| 2.0 * step * r[i + 1] / r[0]).sum();
            ensure!(
                (reconstructed - integral).abs() <= 1e-6 + 1e-6 * integral.abs(),
                "data_invalid: I don't think so {} It's not consistent with the native spectra",
                i + 1
            );
            if cumulative_columns.is_some() {
                let mut cumulative = 0.0;
                for row in &rows {
                    cumulative += 2.0 * step * row[i + 1] / row[0];
                    ensure!(
                        (cumulative - row[columns + i + 1]).abs() <= 1e-6 + 1e-6 * cumulative.abs(),
                        "data_invalid: I don't think so {} It's not consistent",
                        i + 1
                    );
                }
            }
        }
        Ok(Self {
            energy_mev: rows.iter().map(|r| r[0]).collect(),
            a2f,
            lambda_integral,
            lambda_sum,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NativeBands {
    /// 原生 band.eig/phband.freq 均写笛卡尔坐标（2π/alat）。
    pub points_cartesian: Vec<[f64; 3]>,
    pub values: Vec<Vec<f64>>,
}

/// 读取 QE/EPW 的 &plot 格式；不猜测空行、频率单位或能量零点。
pub fn parse_bands(text: &str) -> Result<NativeBands> {
    parse_plot(text, 3)
}

/// QE 7.4.1 matdyn 的坐标记录另含第四列 q 权重，不能当成首个频率。
pub fn parse_matdyn(text: &str) -> Result<NativeBands> {
    parse_plot(text, 4)
}

fn plot_header(text: &str) -> Result<(usize, usize, &str)> {
    let header = regex::Regex::new(r"(?s)^\s*&plot\s+nbnd=\s*(\d+)\s*,\s*nks=\s*(\d+)\s*/")?;
    let cap = header
        .captures(text)
        .context("data_invalid: Lack of native capacity to carry a watch")?;
    let bands: usize = cap[1].parse()?;
    let points: usize = cap[2].parse()?;
    ensure!(bands > 0 && points > 0, "data_invalid: It's empty");
    Ok((bands, points, &text[cap.get(0).unwrap().end()..]))
}

/// 显式路径的 wq 未初始化（QE 7.4.1）；只读取坐标和频率，不把占位列用作 DOS 权重。
pub fn parse_matdyn_path(text: &str) -> Result<NativeBands> {
    let (bands, points, body) = plot_header(text)?;
    let mut lines = body.lines().filter(|line| !line.trim().is_empty());
    let mut out = NativeBands { points_cartesian: vec![], values: vec![] };
    for _ in 0..points {
        let line = lines.next().context("data_invalid: matdyn Path Coordinate Interrupted")?;
        let coords = if line.is_ascii() && line.len() >= 50
            && line[..10].trim().is_empty() && line[50..].trim().is_empty() {
            // matdyn.f90: (10x,4f10.6)。第 4 栏可溢出为 **********，不要求空白分隔。
            [number(line[10..20].trim())?, number(line[20..30].trim())?, number(line[30..40].trim())?]
        } else {
            let row: Vec<_> = line.split_whitespace().collect();
            ensure!(row.len() == 4, "data_invalid: matdyn Path Coordinate Malformed");
            [number(row[0])?, number(row[1])?, number(row[2])?]
        };
        let mut values = Vec::new();
        while values.len() < bands {
            let line = lines.next().context("data_invalid: matdyn Path frequency cut")?;
            values.extend(line.split_whitespace().map(number).collect::<Result<Vec<_>>>()?);
            ensure!(values.len() <= bands, "data_invalid: matdyn Path frequency is redundant");
        }
        out.points_cartesian.push(coords);
        out.values.push(values);
    }
    ensure!(lines.next().is_none(), "data_invalid: matdyn Path Override Record");
    Ok(out)
}

fn parse_plot(text: &str, coordinate_columns: usize) -> Result<NativeBands> {
    let (bands, points, body) = plot_header(text)?;
    let tokens: Vec<&str> = body.split_whitespace().collect();
    ensure!(
        bands.checked_add(coordinate_columns).and_then(|stride| points.checked_mul(stride)) == Some(tokens.len()),
        "data_invalid: Natives can cut off or be redundant"
    );
    let mut out = NativeBands {
        points_cartesian: Vec::new(),
        values: Vec::new(),
    };
    for row in tokens.chunks_exact(bands + coordinate_columns) {
        if coordinate_columns == 4 {
            ensure!(number(row[3])? >= 0.0, "data_invalid: matdyn q Weights are negative");
        }
        out.points_cartesian
            .push([number(row[0])?, number(row[1])?, number(row[2])?]);
        out.values
            .push(row[coordinate_columns..].iter().map(|s| number(s)).collect::<Result<_>>()?);
    }
    Ok(out)
}
