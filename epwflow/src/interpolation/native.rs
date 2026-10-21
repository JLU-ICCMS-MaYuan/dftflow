//! EPW 5.9 原生输出解析。谱能量轴独立于细网格声子本征频率。
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// 只读检查现有原生计算。冒烟产物不能自动取得插值质量或超导认证。
pub fn inspect(work: &Path) -> Result<serde_json::Value> {
    let input = fs::read_to_string(work.join("epw.in"))?;
    let prefix_re = regex::Regex::new(r"(?im)^\s*prefix\s*=\s*'([A-Za-z0-9_-]+)'\s*,?\s*$")?;
    let prefix = prefix_re
        .captures(&input)
        .context("data_invalid: 缺少受支持的 EPW prefix")?;
    let samples_re = regex::Regex::new(r"(?im)^\s*nqstep\s*=\s*([0-9]+)\s*,?\s*$")?;
    let samples: usize = samples_re
        .captures(&input)
        .context("data_invalid: 原生谱检查需要显式 nqstep")?[1]
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
        "data_invalid: 应有且只有一份当前原生谱，发现 {} 份",
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
        "data_invalid: Wannier 局域化缺少原生收敛证据"
    );
    let disentangled = regex::Regex::new(r"Using band disentanglement\s*:\s*T")?.is_match(&wout);
    ensure!(
        !disentangled || wout.contains("<<< Disentanglement convergence criteria satisfied >>>"),
        "data_invalid: Wannier 解纠缠缺少原生收敛证据"
    );
    Ok(serde_json::json!({
        "schema_version": 1, "workflow_status": "native_epc_only", "workflow_complete": false,
        "native_epc_smoke_passed": true, "interpolation_quality": "unverified",
        "eliashberg": "unverified", "automatic_chain": "unverified",
        "samples": samples, "spectrum": spectra[0].file_name().context("data_invalid: 谱文件名为空")?.to_string_lossy(),
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
        .with_context(|| format!("data_invalid: 非法原生数值 {s}"))?;
    ensure!(value.is_finite(), "data_invalid: 原生输出含非有限数");
    Ok(value)
}

/// EPW 的结束计时和进程退出码均必需；它不输出 PW/PH 的 JOB DONE。
pub fn checked_log(log: &str, exit_code: i32) -> Result<()> {
    ensure!(exit_code == 0, "execution_failed: EPW 退出码 {exit_code}");
    ensure!(
        regex::Regex::new(r"Program EPW v\.5\.9\s")?.is_match(log),
        "environment_incompatible: 缺少 EPW 5.9 运行证据"
    );
    ensure!(
        !regex::Regex::new(r"(?i)error in routine|segmentation fault|MPI_Abort")?.is_match(log),
        "execution_failed: EPW 报告原生错误"
    );
    ensure!(
        log.contains("Total program execution")
            && regex::Regex::new(r"EPW\s*:\s*[^\n]*WALL")?.is_match(log),
        "data_invalid: EPW 结束计时不完整"
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
            .context("data_invalid: 原生谱缺少积分耦合结尾")?;
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
            "data_invalid: 原生谱采样数不符"
        );
        let table_columns = rows[0]
            .len()
            .checked_sub(1)
            .filter(|n| *n > 0)
            .context("data_invalid: 原生谱没有 α²F 列")?;
        let columns = cumulative_columns.unwrap_or(table_columns);
        ensure!(
            columns > 0
                && table_columns == columns * if cumulative_columns.is_some() { 2 } else { 1 },
            "data_invalid: 谱表头与列数不一致"
        );
        ensure!(
            rows.iter().all(|r| r.len() == table_columns + 1
                && r[0] > 0.0
                && r[1..].iter().all(|v| *v >= 0.0)),
            "data_invalid: 原生谱列数、能量或权重无效"
        );
        ensure!(
            rows.windows(2).all(|r| r[0][0] < r[1][0]),
            "data_invalid: 原生谱能量轴未严格递增"
        );
        let step = rows[0][0];
        ensure!(
            rows.iter()
                .enumerate()
                .all(|(i, r)| (r[0] - (i + 1) as f64 * step).abs() <= 1e-7 * samples as f64),
            "data_invalid: 原生谱不是从首个正频率开始的均匀网格"
        );
        let coupling = footer
            .lines()
            .find(|l| l.trim_start().starts_with('#'))
            .context("data_invalid: 缺少原生积分 λ 列")?;
        let lambda_integral: Vec<f64> = coupling
            .split_whitespace()
            .skip(1)
            .map(number)
            .collect::<Result<_>>()?;
        ensure!(
            lambda_integral.len() == columns,
            "data_invalid: 原生积分 λ 列数不符"
        );
        let summed = regex::Regex::new(r"Summed el-ph coupling\s+(\S+)")?;
        let value = summed
            .captures(footer)
            .context("data_invalid: 原生谱缺少直接求和 λ")?;
        let lambda_sum = number(&value[1])?;
        ensure!(
            lambda_sum >= 0.0 && lambda_integral.iter().all(|v| *v >= 0.0),
            "data_invalid: 原生 λ 为负"
        );
        let a2f: Vec<Vec<f64>> = (1..=columns)
            .map(|i| rows.iter().map(|r| r[i]).collect())
            .collect();
        for (i, integral) in lambda_integral.iter().enumerate() {
            // spectral.f90 与 supercond.f90 均使用相同的离散矩形求和。
            let reconstructed: f64 = rows.iter().map(|r| 2.0 * step * r[i + 1] / r[0]).sum();
            ensure!(
                (reconstructed - integral).abs() <= 1e-6 + 1e-6 * integral.abs(),
                "data_invalid: 第 {} 列原生谱积分不一致",
                i + 1
            );
            if cumulative_columns.is_some() {
                let mut cumulative = 0.0;
                for row in &rows {
                    cumulative += 2.0 * step * row[i + 1] / row[0];
                    ensure!(
                        (cumulative - row[columns + i + 1]).abs() <= 1e-6 + 1e-6 * cumulative.abs(),
                        "data_invalid: 第 {} 列原生谱累计 λ 不一致",
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
        .context("data_invalid: 缺少原生能带表头")?;
    let bands: usize = cap[1].parse()?;
    let points: usize = cap[2].parse()?;
    ensure!(bands > 0 && points > 0, "data_invalid: 原生能带为空");
    Ok((bands, points, &text[cap.get(0).unwrap().end()..]))
}

/// 显式路径的 wq 未初始化（QE 7.4.1）；只读取坐标和频率，不把占位列用作 DOS 权重。
pub fn parse_matdyn_path(text: &str) -> Result<NativeBands> {
    let (bands, points, body) = plot_header(text)?;
    let mut lines = body.lines().filter(|line| !line.trim().is_empty());
    let mut out = NativeBands { points_cartesian: vec![], values: vec![] };
    for _ in 0..points {
        let line = lines.next().context("data_invalid: matdyn 路径坐标截断")?;
        let coords = if line.is_ascii() && line.len() >= 50
            && line[..10].trim().is_empty() && line[50..].trim().is_empty() {
            // matdyn.f90: (10x,4f10.6)。第 4 栏可溢出为 **********，不要求空白分隔。
            [number(line[10..20].trim())?, number(line[20..30].trim())?, number(line[30..40].trim())?]
        } else {
            let row: Vec<_> = line.split_whitespace().collect();
            ensure!(row.len() == 4, "data_invalid: matdyn 路径坐标布局错误");
            [number(row[0])?, number(row[1])?, number(row[2])?]
        };
        let mut values = Vec::new();
        while values.len() < bands {
            let line = lines.next().context("data_invalid: matdyn 路径频率截断")?;
            values.extend(line.split_whitespace().map(number).collect::<Result<Vec<_>>>()?);
            ensure!(values.len() <= bands, "data_invalid: matdyn 路径频率多余");
        }
        out.points_cartesian.push(coords);
        out.values.push(values);
    }
    ensure!(lines.next().is_none(), "data_invalid: matdyn 路径多余记录");
    Ok(out)
}

fn parse_plot(text: &str, coordinate_columns: usize) -> Result<NativeBands> {
    let (bands, points, body) = plot_header(text)?;
    let tokens: Vec<&str> = body.split_whitespace().collect();
    ensure!(
        bands.checked_add(coordinate_columns).and_then(|stride| points.checked_mul(stride)) == Some(tokens.len()),
        "data_invalid: 原生能带记录截断或多余"
    );
    let mut out = NativeBands {
        points_cartesian: Vec::new(),
        values: Vec::new(),
    };
    for row in tokens.chunks_exact(bands + coordinate_columns) {
        if coordinate_columns == 4 {
            ensure!(number(row[3])? >= 0.0, "data_invalid: matdyn q 权重为负");
        }
        out.points_cartesian
            .push([number(row[0])?, number(row[1])?, number(row[2])?]);
        out.values
            .push(row[coordinate_columns..].iter().map(|s| number(s)).collect::<Result<_>>()?);
    }
    Ok(out)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElectronicQuality {
    pub window_ev: [f64; 2],
    pub points: usize,
    pub reference_states: usize,
    pub max_error_ev: f64,
    pub rms_error_ev: f64,
    pub passed: bool,
}

/// 同一能量零点下覆盖能窗内所有参考态。按能量有序地进行一一匹配，
/// 不允许多个参考态共用同一 Wannier 态，也不拟合能量平移。
pub fn electronic_quality(
    reference: &NativeBands,
    interpolated: &NativeBands,
    window_ev: [f64; 2],
    max_ev: f64,
    rms_ev: f64,
) -> Result<ElectronicQuality> {
    ensure!(
        window_ev.iter().all(|v| v.is_finite())
            && window_ev[0] < window_ev[1]
            && max_ev.is_finite()
            && rms_ev.is_finite()
            && max_ev > 0.0
            && rms_ev > 0.0,
        "configuration: 非法电子质量检查能窗或阈值"
    );
    ensure!(
        !reference.values.is_empty()
            && reference.values.len() == reference.points_cartesian.len()
            && interpolated.values.len() == interpolated.points_cartesian.len()
            && reference.values.len() == interpolated.values.len(),
        "data_invalid: 电子质量检查点不完整"
    );
    let mut errors = Vec::new();
    for i in 0..reference.values.len() {
        ensure!(
            reference.points_cartesian[i]
                .iter()
                .zip(interpolated.points_cartesian[i])
                .all(|(a, b)| a.is_finite() && b.is_finite() && (a - b).abs() <= 1e-6),
            "data_invalid: 第 {} 个质量检查点坐标不一致",
            i + 1
        );
        ensure!(
            !reference.values[i].is_empty()
                && !interpolated.values[i].is_empty()
                && reference.values[i]
                    .iter()
                    .chain(&interpolated.values[i])
                    .all(|v| v.is_finite()),
            "data_invalid: 第 {} 个点的能量数据无效",
            i + 1
        );
        let mut r: Vec<f64> = reference.values[i]
            .iter()
            .copied()
            .filter(|v| window_ev[0] <= *v && *v <= window_ev[1])
            .collect();
        let mut c = interpolated.values[i].clone();
        r.sort_by(f64::total_cmp);
        c.sort_by(f64::total_cmp);
        ensure!(
            r.len() <= c.len(),
            "data_invalid: 第 {} 个点的 Wannier 态不足以覆盖验收能窗",
            i + 1
        );
        // 最小平方误差的有序单射匹配；保存匹配，而非只比较平均能量。
        let mut cost = vec![vec![f64::INFINITY; c.len() + 1]; r.len() + 1];
        let mut take = vec![vec![false; c.len() + 1]; r.len() + 1];
        cost[0].fill(0.0);
        for a in 1..=r.len() {
            for b in 1..=c.len() {
                let matched = cost[a - 1][b - 1] + (r[a - 1] - c[b - 1]).powi(2);
                if matched < cost[a][b - 1] {
                    cost[a][b] = matched;
                    take[a][b] = true;
                } else {
                    cost[a][b] = cost[a][b - 1];
                }
            }
        }
        let (mut a, mut b) = (r.len(), c.len());
        while a > 0 {
            ensure!(b > 0, "data_invalid: 电子态匹配不完整");
            if take[a][b] {
                errors.push((r[a - 1] - c[b - 1]).abs());
                a -= 1;
            }
            b -= 1;
        }
    }
    ensure!(!errors.is_empty(), "data_invalid: 验收能窗内没有电子参考态");
    let max_error_ev = errors.iter().copied().fold(0.0, f64::max);
    let rms_error_ev = (errors.iter().map(|e| e * e).sum::<f64>() / errors.len() as f64).sqrt();
    ensure!(max_error_ev.is_finite() && rms_error_ev.is_finite(), "data_invalid: 电子误差计算溢出");
    Ok(ElectronicQuality {
        window_ev,
        points: reference.values.len(),
        reference_states: errors.len(),
        max_error_ev,
        rms_error_ev,
        passed: max_error_ev <= max_ev && rms_error_ev <= rms_ev,
    })
}
