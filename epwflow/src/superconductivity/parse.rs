use super::super::interpolation::fine;
use anyhow::{ensure, Context, Result};
use std::path::Path;

/// Fortran D 指数；拒绝 NaN/Inf 和溢出，保留原生物理量。
pub fn numbers(line: &str) -> Result<Vec<f64>> {
    line.split_whitespace()
        .map(|s| {
            let v = s
                .replace(['D', 'd'], "E")
                .parse::<f64>()
                .with_context(|| format!("data_invalid: 原生数值无效 {s}"))?;
            ensure!(v.is_finite(), "data_invalid: 原生输出含非有限数");
            Ok(v)
        })
        .collect()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolveEvidence {
    pub temperature_centikelvin: i64,
    pub nfreq: usize,
    pub iterations: Option<u32>,
    /// LAPACK 不输出非线性残差，不能补零。
    pub relative_residual: Option<f64>,
    pub maximum_eigenvalue: Option<f64>,
}

/// 单次日志必须只有一份相符的温度/模式求解，不接受拼接或多温度日志。
pub fn solve_log(log: &str, request: &super::attempt::Request) -> Result<SolveEvidence> {
    use super::attempt::SolveKind;
    crate::interpolation::native::checked_log(log, 0)?;
    let linear = request.kind == SolveKind::Linear;
    let mode = if request.mode == crate::foundation::config::Mode::Iso {
        "isotropic"
    } else {
        "anisotropic"
    };
    let start = if linear {
        "Start: Solving (isotropic) linearized Eliashberg equation with solver = lapack".to_string()
    } else {
        format!("Solve {mode} Eliashberg equations on imaginary-axis")
    };
    ensure!(
        log.lines().filter(|l| l.trim() == start).count() == 1,
        "data_invalid: 缺失或重复目标求解段"
    );
    ensure!(
        log.matches("Solve isotropic Eliashberg equations on imaginary-axis")
            .count()
            + log
                .matches("Solve anisotropic Eliashberg equations on imaginary-axis")
                .count()
            + log.matches("Start: Solving (isotropic) linearized").count()
            == 1,
        "data_invalid: 多模式/多次求解日志"
    );
    let freq = regex::Regex::new(
        r"(?m)^\s*Total number of frequency points nsiw\(\s*1\)\s*=\s*(\d+)(?: for uniform\s+sampling)?\s*$",
    )?;
    // 线性路径另打印一次网格说明；两份必须一致。
    let counts = freq
        .captures_iter(log)
        .map(|c| c[1].parse::<usize>())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let nf = request.nfreq()?;
    ensure!(
        !counts.is_empty() && counts.len() <= 2 && counts.iter().all(|n| *n == nf),
        "data_invalid: 原生频率数与固定截断不同"
    );
    let temp = if linear {
        regex::Regex::new(r"(?m)^\s*For the first Temp\.\s+([0-9.]+)\s+K\s*$")?
    } else {
        regex::Regex::new(r"(?m)^\s*temp\(\s*1\)\s*=\s*([0-9.]+)\s+K\s*$")?
    };
    let ts = temp.captures_iter(log).collect::<Vec<_>>();
    ensure!(
        ts.len() == 1
            && (ts[0][1].parse::<f64>()? * 100.0 - request.temperature_centikelvin as f64).abs()
                < 1e-5,
        "data_invalid: 求解实际温度不符"
    );
    let mut result = SolveEvidence {
        temperature_centikelvin: request.temperature_centikelvin,
        nfreq: nf,
        iterations: None,
        relative_residual: None,
        maximum_eigenvalue: None,
    };
    if linear {
        let finish = "Finish: Solving (isotropic) linearized Eliashberg equation";
        ensure!(
            log.matches(finish).count() == 1,
            "data_invalid: 线性 LAPACK 未完成"
        );
        let block = log
            .split_once(&start)
            .unwrap()
            .1
            .split_once(finish)
            .unwrap()
            .0;
        let rows = block
            .lines()
            .filter_map(|l| {
                let parts = l.split_whitespace().collect::<Vec<_>>();
                (parts.len() == 5 && parts[0].parse::<f64>().is_ok()).then_some(l)
            })
            .map(numbers)
            .collect::<Result<Vec<_>>>()?;
        ensure!(rows.len() == 1, "data_invalid: 线性本征值行缺失或多温度");
        let row = &rows[0];
        let wcut = (2 * nf + 1) as f64
            * std::f64::consts::PI
            * super::gap::KB_EV_K
            * request.temperature_centikelvin as f64
            / 100.0;
        ensure!(
            (row[0] * 100.0 - request.temperature_centikelvin as f64).abs() < 1e-6
                && row[2] == nf as f64
                && (row[3] - wcut).abs() <= 5.1e-5,
            "data_invalid: 线性温度/频率表冲突"
        );
        // EPW 的直接 LAPACK 路径 numiter=1，不是非线性迭代次数。
        ensure!(row[4] == 1.0, "data_invalid: 非预期 LAPACK 求解记录");
        result.maximum_eigenvalue = Some(row[1]);
    } else {
        ensure!(
            !log.contains("Convergence was not reached"),
            "solver_failed: 原生非线性未收敛"
        );
        let reached =
            regex::Regex::new(r"(?m)^\s*Convergence was reached in nsiter =\s*(\d+)\s*$")?;
        let cs = reached.captures_iter(log).collect::<Vec<_>>();
        ensure!(cs.len() == 1, "solver_failed: 缺少明确收敛证据");
        let iter = cs[0][1].parse::<u32>()?;
        ensure!(
            iter > 0 && iter <= request.max_iterations,
            "solver_failed: 原生迭代次数无效"
        );
        let block = log
            .split_once(&start)
            .unwrap()
            .1
            .split_once("Convergence was reached in nsiter =")
            .unwrap()
            .0;
        let after_header = block
            .split_once("deltai [meV]")
            .context("data_invalid: 缺少 FSR 迭代表头")?
            .1;
        let mut last = None;
        for line in after_header.lines() {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.first().is_some_and(|p| p.parse::<u32>().is_ok()) {
                let row = numbers(line)?;
                ensure!(
                    row.len() == 4 && row[0] == last.map_or(1.0, |(n, _): (f64, f64)| n + 1.0),
                    "data_invalid: 原生迭代缺失/重复/列冲突"
                );
                last = Some((row[0], row[1]));
            }
        }
        let (n, residual) = last.context("data_invalid: 缺少原生相对残差")?;
        ensure!(
            n == iter as f64 && residual >= 0.0 && residual < request.convergence_relative,
            "solver_failed: 最终相对残差不满足固定阈值"
        );
        result.iterations = Some(iter);
        result.relative_residual = Some(residual);
    }
    Ok(result)
}

/// 谱数据：频率（meV）、α²F、有效耦合 λ 与对数平均频率（K）。
#[derive(Debug, Clone)]
pub struct Spectrum {
    pub frequencies_mev: Vec<f64>,
    pub a2f: Vec<f64>,
    pub lambda_eff: f64,
    pub omega_log_k: f64,
    pub cutoff_mev: f64,
}

/// 从插值谱读取 α²F 与频率（FR-003）。频率为 cm⁻¹，转换为 meV。
pub fn parse_spectrum(
    frequency_cm1_path: &Path,
    a2f_mev_path: &Path,
    cutoff_ev: f64,
) -> Result<Spectrum> {
    let freq_cm1 = fine::read_f64_col(frequency_cm1_path).with_context(|| {
        format!(
            "data_invalid: 无法读取频率 {}",
            frequency_cm1_path.display()
        )
    })?;
    let a2f = fine::read_f64_col(a2f_mev_path)
        .with_context(|| format!("data_invalid: 无法读取 α²F {}", a2f_mev_path.display()))?;
    ensure!(
        freq_cm1.len() == a2f.len() && !freq_cm1.is_empty(),
        "data_invalid: 频率与 α²F 长度不一致"
    );
    let cm1_to_mev = 0.123984;
    let frequencies_mev: Vec<f64> = freq_cm1.iter().map(|w| *w * cm1_to_mev).collect();
    compute_lambda(&frequencies_mev, &a2f, cutoff_ev * 1000.0)
}

/// 由频率与 α²F 计算有效耦合 λ 与对数平均频率 ω_log（K）。
pub fn compute_lambda(frequencies_mev: &[f64], a2f: &[f64], cutoff_mev: f64) -> Result<Spectrum> {
    // λ = 2 ∫ α²F(ω)/ω dω；ω_log = exp[(2/λ) ∫ α²F(ω)/ω ln ω dω]。
    let mut lambda = 0.0_f64;
    let mut lognum = 0.0_f64;
    for i in 0..frequencies_mev.len() {
        let w = frequencies_mev[i];
        if w <= 0.0 {
            continue;
        }
        let dw = if i + 1 < frequencies_mev.len() {
            (frequencies_mev[i + 1] - frequencies_mev[i]).abs()
        } else if i > 0 {
            (frequencies_mev[i] - frequencies_mev[i - 1]).abs()
        } else {
            1.0
        };
        let contrib = 2.0 * a2f[i] / w * dw;
        lambda += contrib;
        lognum += contrib * w.ln();
    }
    ensure!(lambda > 0.0, "data_invalid: 有效耦合 λ 非正");
    let omega_log_mev = (lognum / lambda).exp();
    let k2mev = 0.086173_f64;
    Ok(Spectrum {
        frequencies_mev: frequencies_mev.to_vec(),
        a2f: a2f.to_vec(),
        lambda_eff: lambda,
        omega_log_k: omega_log_mev / k2mev,
        cutoff_mev,
    })
}
