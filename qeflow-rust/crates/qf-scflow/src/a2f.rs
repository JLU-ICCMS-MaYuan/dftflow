//! α²F（Eliashberg 函数）数据模型与 λ/wlog/w2 累积量计算（复刻 `scflow/sc_a2f.py`）。
//!
//! 约定：频率单位为 THz，α²F 单位为 THz（与 Python 端 `a2f['freqs, THz']` 等保持一致）。

use crate::constants::K_T_THZ;
use crate::integ::{cumulative_simpson, cumulative_sum, simpson};
use crate::Integration;
use anyhow::{Context, Result};
use std::path::Path;

/// 一条 α²F 曲线：频率网格 `freqs` 及 lambda/gamma 两个通道的 α²F 值。
#[derive(Debug, Clone)]
pub struct A2f {
    pub freqs: Vec<f64>,
    pub a2f_lambda: Vec<f64>,
    pub a2f_gamma: Vec<f64>,
}

impl A2f {
    /// 读取 3 列空格分隔文件：`freq(THz)  a2f_lambda(THz)  a2f_gamma(THz)`，
    /// 以 `#` 开头的行视为注释。该格式即 Python 端 `a2f['freqs, THz']` 等字段的内存等价物，
    /// 可作为 scflow 单点垂直切片的最小输入。
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let mut freqs = Vec::new();
        let mut a2f_lambda = Vec::new();
        let mut a2f_gamma = Vec::new();
        for (lineno, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<f64> = line
                .split_whitespace()
                .filter_map(|s| s.parse::<f64>().ok())
                .collect();
            if cols.len() < 3 {
                anyhow::bail!(
                    "a2F 行 {} 至少需要 3 列 (freq, a2f_lambda, a2f_gamma)，实际: {:?}",
                    lineno + 1,
                    cols
                );
            }
            freqs.push(cols[0]);
            a2f_lambda.push(cols[1]);
            a2f_gamma.push(cols[2]);
        }
        if freqs.len() < 2 {
            anyhow::bail!("a2F 至少需要 2 个频率点");
        }
        if !(freqs.windows(2).all(|w| w[1] >= w[0])) {
            anyhow::bail!("a2F 频率必须单调不减");
        }
        Ok(A2f {
            freqs,
            a2f_lambda,
            a2f_gamma,
        })
    }
}

/// 计算累积 λ(ω) = 2 ∫ α²F(ω')/ω' dω'（lambda 与 gamma 两个通道）。
/// 复刻 `scflow/sc_a2f.py: lambdas_a2f`，对零频点做安全跳过（物理上 α²F(0)=0）。
pub fn compute_lambda(a2f: &A2f, integration: Integration) -> (Vec<f64>, Vec<f64>) {
    let n = a2f.freqs.len();
    let mut int_l = vec![0.0; n];
    let mut int_g = vec![0.0; n];
    for i in 0..n {
        if a2f.freqs[i] > 0.0 {
            int_l[i] = a2f.a2f_lambda[i] / a2f.freqs[i];
            int_g[i] = a2f.a2f_gamma[i] / a2f.freqs[i];
        }
    }
    match integration {
        Integration::Sum => {
            let dw = a2f.freqs[1] - a2f.freqs[0];
            let cl = cumulative_sum(&int_l, dw);
            let cg = cumulative_sum(&int_g, dw);
            let lam_l: Vec<f64> = cl.iter().map(|v| 2.0 * v).collect();
            let lam_g: Vec<f64> = cg.iter().map(|v| 2.0 * v).collect();
            (lam_l, lam_g)
        }
        Integration::Simps => {
            let cl = cumulative_simpson(&int_l, &a2f.freqs);
            let cg = cumulative_simpson(&int_g, &a2f.freqs);
            let lam_l: Vec<f64> = cl.iter().map(|v| 2.0 * v).collect();
            let lam_g: Vec<f64> = cg.iter().map(|v| 2.0 * v).collect();
            (lam_l, lam_g)
        }
    }
}

/// 计算累积 wlog(ω) = exp( (2/λ) ∫ α²F(ω')/ω'·ln(ω'/k_K_THz) dω' )。
/// 复刻 `scflow/sc_a2f.py: wlogs_a2f`，`λ` 取总 λ。
pub fn compute_wlog(
    a2f: &A2f,
    lambda_total: f64,
    integration: Integration,
) -> (Vec<f64>, Vec<f64>) {
    let n = a2f.freqs.len();
    let mut int_l = vec![0.0; n];
    let mut int_g = vec![0.0; n];
    for i in 0..n {
        if a2f.freqs[i] > 0.0 {
            let lnw = (a2f.freqs[i] / K_T_THZ).ln();
            int_l[i] = a2f.a2f_lambda[i] * lnw / a2f.freqs[i];
            int_g[i] = a2f.a2f_gamma[i] * lnw / a2f.freqs[i];
        }
    }
    let kinv2 = 1.0 / (K_T_THZ * K_T_THZ);
    let build = |int: &[f64]| -> Vec<f64> {
        if lambda_total <= 0.0 {
            return vec![0.0; n];
        }
        let cum = match integration {
            Integration::Sum => {
                let dw = a2f.freqs[1] - a2f.freqs[0];
                cumulative_sum(int, dw)
            }
            Integration::Simps => cumulative_simpson(int, &a2f.freqs),
        };
        cum.iter().map(|v| (2.0 / lambda_total * v).exp()).collect()
    };
    let _ = kinv2;
    (build(&int_l), build(&int_g))
}

/// 计算累积 w2(ω) = sqrt( (2/λ) ∫ α²F(ω')·ω' / k_K_THz² dω' )。
/// 复刻 `scflow/sc_a2f.py: w2s_a2f`。
pub fn compute_w2(
    a2f: &A2f,
    lambda_total: f64,
    integration: Integration,
) -> (Vec<f64>, Vec<f64>) {
    let n = a2f.freqs.len();
    let kinv2 = 1.0 / (K_T_THZ * K_T_THZ);
    let mut int_l = vec![0.0; n];
    let mut int_g = vec![0.0; n];
    for i in 0..n {
        int_l[i] = a2f.a2f_lambda[i] * a2f.freqs[i] * kinv2;
        int_g[i] = a2f.a2f_gamma[i] * a2f.freqs[i] * kinv2;
    }
    let build = |int: &[f64]| -> Vec<f64> {
        if lambda_total <= 0.0 {
            return vec![0.0; n];
        }
        let cum = match integration {
            Integration::Sum => {
                let dw = a2f.freqs[1] - a2f.freqs[0];
                cumulative_sum(int, dw)
            }
            Integration::Simps => cumulative_simpson(int, &a2f.freqs),
        };
        cum.iter().map(|v| (2.0 / lambda_total * v).sqrt()).collect()
    };
    (build(&int_l), build(&int_g))
}
