//! direct 通道（复刻 `scflow/sc_direct.py`）：不经积分直接由 λ/γ 构建 a2F 与累积量。
//! stairs 的输出数组在 Python 中是 float16，这里用 IEEE binary16 舍入精确复刻。

use anyhow::Result;
use crate::constants::{K_RY_GHZ, K_RY_THZ};
use std::f64::consts::PI;

pub const K_RY_K: f64 = 157887.6061;

/// f64 → binary16（IEEE 754 half，round-to-nearest-even）→ f64。
pub fn as_f16(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    if x == 0.0 {
        return x;
    }
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let mag = x.abs();
    const MAX_NORM: f64 = 65504.0;
    const MIN_NORM: f64 = 6.103515625e-05; // 2^-14
    const MIN_SUB: f64 = 5.960464477539063e-08; // 2^-24
    if mag >= MAX_NORM + 16.0 {
        // 半精度上溢（Python float16 溢出为 inf）
        return sign * f64::INFINITY;
    }
    if mag < MIN_SUB / 2.0 {
        return sign * 0.0;
    }
    if mag >= MIN_NORM {
        // 规格数
        let e = mag.log2().floor();
        let spacing = (2.0_f64).powi(e as i32 - 10);
        let q = round_half_even(mag / spacing);
        return sign * q * spacing;
    }
    // 次规格数
    let q = round_half_even(mag / MIN_SUB);
    if q >= 1024.0 {
        return sign * MIN_NORM;
    }
    sign * q * MIN_SUB
}

fn round_half_even(x: f64) -> f64 {
    let floor = x.floor();
    let frac = x - floor;
    if frac > 0.5 {
        floor + 1.0
    } else if frac < 0.5 {
        floor
    } else if (floor as i64) % 2 == 0 {
        floor
    } else {
        floor + 1.0
    }
}

/// `stairs`：**排除自身**的累积前缀和（`arr[:i]` 不含第 i 项），输出按 float16 舍入。
/// 复刻 `scflow/utils.py: stairs`（其文档字符串描述的是包含式，实现为排除式）。
pub fn stairs(arr: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0f64; arr.len()];
    let mut acc = 0.0f64;
    for (i, v) in arr.iter().enumerate() {
        out[i] = as_f16(acc);
        acc += v;
    }
    out
}

/// 排除式前缀和（f64，Python `stairs(..., dtype=np.longdouble)` 的 macOS 语义）。
pub fn stairs_f64(arr: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0f64; arr.len()];
    let mut acc = 0.0f64;
    for (i, v) in arr.iter().enumerate() {
        out[i] = acc;
        acc += v;
    }
    out
}

fn divide_nonzero(numerator: &[f64], denominator: &[f64]) -> Vec<f64> {
    numerator
        .iter()
        .zip(denominator)
        .map(|(n, d)| if *d != 0.0 { n / d } else { 0.0 })
        .collect()
}

/// 单个 q 点的频率（THz，保留虚频符号）与平滑因子；零频且 λ≠0 直接报错。
fn frequencies_and_smoothing(dyn_elph: &crate::epc::DynElph, lambdas: &[f64], smoothing: f64) -> Result<(Vec<f64>, Vec<f64>)> {
    let freqs: Vec<f64> = dyn_elph
        .sqr_freqs
        .iter()
        .map(|s| s.abs().sqrt() * s.signum() * K_RY_THZ)
        .collect();
    let mut smooth = vec![0.0f64; freqs.len()];
    for (i, f) in freqs.iter().enumerate() {
        if *f != 0.0 {
            smooth[i] = (-(smoothing * smoothing) / (f * f)).exp();
        } else if lambdas[i] != 0.0 {
            anyhow::bail!("A zero-frequency mode has nonzero lambda; cannot exclude it");
        }
    }
    Ok((freqs, smooth))
}

/// `a2f_direct`（f64，Python dtype 参数未实际使用）。
pub fn a2f_direct(
    q_points: &[(&crate::epc::DynElph, f64)],
    smoothing: f64,
) -> Result<Vec<(String, Vec<f64>)>> {
    let mut freqs: Vec<f64> = Vec::new();
    let mut a2f_lambda: Vec<f64> = Vec::new();
    let mut a2f_gamma: Vec<f64> = Vec::new();
    for (dyn_elph, weight) in q_points {
        let (q_freqs, smooth) = frequencies_and_smoothing(dyn_elph, &dyn_elph.lambdas[0], smoothing)?;
        let _ = &dyn_elph.lambdas[0];
        for i in 0..q_freqs.len() {
            let lambdas_i = dyn_elph.lambdas[0][i];
            let gammas_i = dyn_elph.gammas[0][i];
            let nef = dyn_elph.dos[0];
            let q_a2f_lambda = smooth[i] * lambdas_i * q_freqs[i] / 2.0 * weight;
            let q_a2f_gamma = if q_freqs[i] != 0.0 {
                smooth[i] * gammas_i / q_freqs[i]
            } else {
                0.0
            } / nef
                / K_RY_GHZ
                / PI
                / 2.0
                * weight
                * K_RY_THZ.powi(2);
            freqs.push(q_freqs[i]);
            a2f_lambda.push(q_a2f_lambda);
            a2f_gamma.push(q_a2f_gamma);
        }
    }
    // 按频率排序
    let mut idx: Vec<usize> = (0..freqs.len()).collect();
    idx.sort_by(|a, b| freqs[*a].partial_cmp(&freqs[*b]).unwrap());
    let freqs: Vec<f64> = idx.iter().map(|i| freqs[*i]).collect();
    let mut a2f_lambda: Vec<f64> = idx.iter().map(|i| a2f_lambda[*i]).collect();
    let mut a2f_gamma: Vec<f64> = idx.iter().map(|i| a2f_gamma[*i]).collect();
    for v in a2f_lambda.iter_mut() {
        if *v <= 0.0 {
            *v = 0.0;
        }
    }
    for v in a2f_gamma.iter_mut() {
        if *v <= 0.0 {
            *v = 0.0;
        }
    }
    Ok(vec![
        ("freqs, THz".into(), freqs),
        ("a2f (lambda), THz".into(), a2f_lambda),
        ("a2f (gamma), THz".into(), a2f_gamma),
    ])
}

/// `lambdas_direct`：λ/γ 直接累积（含未平滑 stairs）。
#[allow(clippy::type_complexity)]
pub fn lambdas_direct(
    q_points: &[(&crate::epc::DynElph, f64)],
    smoothing: f64,
) -> Result<Vec<(String, Vec<f64>)>> {
    let mut freqs_smooth: Vec<f64> = Vec::new();
    let mut lambdas_lambda_smooth: Vec<f64> = Vec::new();
    let mut lambdas_gamma_smooth: Vec<f64> = Vec::new();
    let mut lambdas_lambda: Vec<f64> = Vec::new();
    let mut lambdas_gamma: Vec<f64> = Vec::new();
    for (dyn_elph, weight) in q_points {
        let (q_freqs, smooth) = frequencies_and_smoothing(dyn_elph, &dyn_elph.lambdas[0], smoothing)?;
        for i in 0..q_freqs.len() {
            let l = dyn_elph.lambdas[0][i];
            let g = dyn_elph.gammas[0][i];
            let nef = dyn_elph.dos[0];
            lambdas_lambda_smooth.push(smooth[i] * l * weight);
            lambdas_gamma_smooth.push(
                if q_freqs[i] != 0.0 { smooth[i] * g * weight / (q_freqs[i] * q_freqs[i]) } else { 0.0 }
                    / nef
                    * K_RY_THZ
                    / 1000.0
                    / PI,
            );
            lambdas_lambda.push(l * weight);
            lambdas_gamma.push(
                if q_freqs[i] != 0.0 { g * weight / (q_freqs[i] * q_freqs[i]) } else { 0.0 }
                    / nef
                    * K_RY_THZ
                    / 1000.0
                    / PI,
            );
            freqs_smooth.push(q_freqs[i]);
        }
    }
    // smooth 序列按 freqs 排序后 stairs
    let mut idx: Vec<usize> = (0..freqs_smooth.len()).collect();
    idx.sort_by(|a, b| freqs_smooth[*a].partial_cmp(&freqs_smooth[*b]).unwrap());
    let lambdas_lambda_smooth: Vec<f64> = idx.iter().map(|i| lambdas_lambda_smooth[*i]).collect();
    let lambdas_gamma_smooth: Vec<f64> = idx.iter().map(|i| lambdas_gamma_smooth[*i]).collect();
    let lambda_lambda_stairs_smooth = stairs(&lambdas_lambda_smooth);
    let lambda_gamma_stairs_smooth = stairs(&lambdas_gamma_smooth);
    // 未平滑序列同样按 freqs 排序后 stairs（Python: idxs = freqs.argsort()）
    let mut idx2: Vec<usize> = (0..freqs_smooth.len()).collect();
    idx2.sort_by(|a, b| freqs_smooth[*a].partial_cmp(&freqs_smooth[*b]).unwrap());
    let lambdas_lambda: Vec<f64> = idx2.iter().map(|i| lambdas_lambda[*i]).collect();
    let lambdas_gamma: Vec<f64> = idx2.iter().map(|i| lambdas_gamma[*i]).collect();
    let lambda_lambda_stairs = stairs(&lambdas_lambda);
    let lambda_gamma_stairs = stairs(&lambdas_gamma);
    Ok(vec![
        ("lambdas (lambda)".into(), lambdas_lambda_smooth),
        ("lambdas (gamma)".into(), lambdas_gamma_smooth),
        ("lambda (lambda)".into(), lambda_lambda_stairs_smooth),
        ("lambda (gamma)".into(), lambda_gamma_stairs_smooth),
        ("lambda (lambda, not smooth)".into(), lambda_lambda_stairs),
        ("lambda (gamma, not smooth)".into(), lambda_gamma_stairs),
    ])
}

/// `wlogs_direct`：ω_log 直接累积（复刻分段/前缀语义）。
pub fn wlogs_direct(direct: &[(String, Vec<f64>)]) -> Vec<(String, Vec<f64>)> {
    let get = |name: &str| -> Vec<f64> {
        direct
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    let freqs = get("freqs, THz");
    let lambdas_lambda = get("lambdas (lambda)");
    let lambdas_gamma = get("lambdas (gamma)");
    let lambda_lambda: f64 = lambdas_lambda.iter().sum();
    let lambda_gamma: f64 = lambdas_gamma.iter().sum();
    // freqs>0 子集
    let mut wlogs_lambda: Vec<f64> = Vec::new();
    let mut wlogs_gamma: Vec<f64> = Vec::new();
    for (f, l) in freqs.iter().zip(&lambdas_lambda) {
        if *f > 0.0 {
            wlogs_lambda.push(l * (f / K_RY_THZ * K_RY_K).ln() / lambda_lambda);
        }
    }
    for (f, l) in freqs.iter().zip(&lambdas_gamma) {
        if *f > 0.0 {
            wlogs_gamma.push(l * (f / K_RY_THZ * K_RY_K).ln() / lambda_gamma);
        }
    }
    // <=0 截零
    for v in wlogs_lambda.iter_mut() {
        if *v <= 0.0 {
            *v = 0.0;
        }
    }
    for v in wlogs_gamma.iter_mut() {
        if *v <= 0.0 {
            *v = 0.0;
        }
    }
    let n_nonpos = freqs.iter().filter(|f| **f <= 0.0).count();
    let zeros_freq: Vec<f64> = vec![0.0; n_nonpos];
    // stairs 部分：zeros(非正子集计数) ++ exp(f16 stairs(正子集))（np.exp 作用在
    // float16 数组上，结果也是 float16）
    let build_stairs = |wlogs: &[f64]| -> Vec<f64> {
        let n_le0 = wlogs.iter().filter(|v| **v <= 0.0).count();
        let positives: Vec<f64> = wlogs.iter().filter(|v| **v > 0.0).copied().collect();
        let mut out: Vec<f64> = vec![0.0; n_le0];
        out.extend(stairs(&positives).iter().map(|v| as_f16(v.exp())));
        out
    };
    let mut wlogs_lambda_stairs: Vec<f64> = zeros_freq.clone();
    wlogs_lambda_stairs.extend(build_stairs(&wlogs_lambda));
    let mut wlogs_gamma_stairs: Vec<f64> = zeros_freq.clone();
    wlogs_gamma_stairs.extend(build_stairs(&wlogs_gamma));
    let mut wlogs_lambda_padded = zeros_freq.clone();
    wlogs_lambda_padded.extend(wlogs_lambda.iter().copied());
    let mut wlogs_gamma_padded = zeros_freq.clone();
    wlogs_gamma_padded.extend(wlogs_gamma.iter().copied());
    vec![
        ("wlogs (lambda), K".into(), wlogs_lambda_padded),
        ("wlogs (gamma), K".into(), wlogs_gamma_padded),
        ("wlog (lambda), K".into(), wlogs_lambda_stairs),
        ("wlog (gamma), K".into(), wlogs_gamma_stairs),
    ]
}

/// `w2s_direct`（Python 用 longdouble stairs；本机约定下等价 f64）。
pub fn w2s_direct(direct: &[(String, Vec<f64>)]) -> Vec<(String, Vec<f64>)> {
    let get = |name: &str| -> Vec<f64> {
        direct
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    let freqs = get("freqs, THz");
    let lambdas_lambda = get("lambdas (lambda)");
    let lambdas_gamma = get("lambdas (gamma)");
    let lambda_lambda: f64 = lambdas_lambda.iter().sum();
    let lambda_gamma: f64 = lambdas_gamma.iter().sum();
    let w2s_lambda: Vec<f64> = lambdas_lambda
        .iter()
        .zip(&freqs)
        .map(|(l, f)| l * (f / K_RY_THZ * K_RY_K).powi(2) / lambda_lambda)
        .collect();
    let w2s_gamma: Vec<f64> = lambdas_gamma
        .iter()
        .zip(&freqs)
        .map(|(l, f)| l * (f / K_RY_THZ * K_RY_K).powi(2) / lambda_gamma)
        .collect();
    let w2_lambda_stairs: Vec<f64> = stairs_f64(&w2s_lambda).iter().map(|v| v.sqrt()).collect();
    let w2_gamma_stairs: Vec<f64> = stairs_f64(&w2s_gamma).iter().map(|v| v.sqrt()).collect();
    vec![
        ("w2s (lambda), K".into(), w2s_lambda),
        ("w2s (gamma), K".into(), w2s_gamma),
        ("w2 (lambda), K".into(), w2_lambda_stairs),
        ("w2 (gamma), K".into(), w2_gamma_stairs),
    ]
}
