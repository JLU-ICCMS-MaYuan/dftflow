//! System（复刻 `scflow/system.py` + `scflow/sc_a2f.py`）：
//! direct/a2f 两个有序字典（列序 = Python dict 插入序）及 λ/wlog/w2/Tc 累积量。

use anyhow::{Context, Result};
use crate::a2f;
use crate::constants::K_T_THZ;
use crate::direct;
use crate::epc::DynElph;
use crate::interp::interp_lambda;
use crate::integ::{cumulative_sum, simpson};
use crate::tc::{allen_dynes_tc_array, mc_millan_tc_array};

/// 有序字典：键保持 Python dict 插入顺序。
pub type OrderedDict = Vec<(String, Vec<f64>)>;

pub fn get<'a>(dict: &'a OrderedDict, key: &str) -> Option<&'a Vec<f64>> {
    dict.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

pub fn get_mut<'a>(dict: &'a mut OrderedDict, key: &str) -> Option<&'a mut Vec<f64>> {
    dict.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn push(dict: &mut OrderedDict, key: &str, values: Vec<f64>) {
    dict.push((key.to_string(), values));
}

/// f64 → f32 → f64（对齐 Python `dtype=np.float32` 数组存储）。
pub fn as_f32(x: f64) -> f64 {
    (x as f32) as f64
}

pub struct System {
    pub direct: OrderedDict,
    pub a2f: OrderedDict,
    pub q_points: Vec<(DynElph, f64)>,
    pub smoothing: f64,
    pub resolution: usize,
    pub sigma: f64,
    pub integration: String,
    pub mu: f64,
}

/// q 点匹配：`compare_q_points`（前 5 位小数一致）。
fn compare_q_points(a: &[f64; 3], b: &[f64; 3]) -> bool {
    for i in 0..3 {
        let ra = (a[i] * 1e5).round() / 1e5;
        let rb = (b[i] * 1e5).round() / 1e5;
        if ra != rb {
            return false;
        }
    }
    true
}

impl System {
    pub fn new(dyn_elphs: Vec<DynElph>, weights: &[([f64; 3], f64)]) -> Self {
        let mut q_points: Vec<(DynElph, f64)> = Vec::new();
        for dyn_elph in dyn_elphs {
            for (q, w) in weights {
                if compare_q_points(&dyn_elph.q_point, q) {
                    q_points.push((dyn_elph, *w));
                    break;
                }
            }
        }
        Self {
            direct: Vec::new(),
            a2f: Vec::new(),
            q_points,
            smoothing: 0.0,
            resolution: 0,
            sigma: 0.0,
            integration: "sum".to_string(),
            mu: 0.0,
        }
    }

    pub fn get_direct(&mut self, smoothing: f64) -> Result<()> {
        self.smoothing = smoothing;
        let refs: Vec<(&DynElph, f64)> = self.q_points.iter().map(|(d, w)| (d, *w)).collect();
        let mut new_dict = direct::a2f_direct(&refs, smoothing)?;
        new_dict.extend(direct::lambdas_direct(&refs, smoothing)?);
        new_dict.extend(direct::wlogs_direct(&new_dict));
        new_dict.extend(direct::w2s_direct(&new_dict));
        self.direct = new_dict;
        Ok(())
    }

    pub fn get_a2f(&mut self, resolution: usize, sigma: f64, integration: &str) -> Result<()> {
        self.resolution = resolution;
        self.sigma = sigma;
        self.integration = integration.to_string();
        let freqs = get(&self.direct, "freqs, THz").cloned().context("direct 缺少 freqs")?;
        let dl_l = get(&self.direct, "lambda (lambda, not smooth)").cloned().context("缺少 lambda (lambda, not smooth)")?;
        let dl_l_smooth = get(&self.direct, "lambda (lambda)").cloned().context("缺少 lambda (lambda)")?;
        let dl_g = get(&self.direct, "lambda (gamma, not smooth)").cloned().context("缺少 lambda (gamma, not smooth)")?;
        let dl_g_smooth = get(&self.direct, "lambda (gamma)").cloned().context("缺少 lambda (gamma)")?;

        let (x_new, a2f_lambda) =
            interp_lambda(&freqs, &dl_l, &dl_l_smooth, resolution, sigma, self.smoothing);
        let (_, a2f_gamma) =
            interp_lambda(&freqs, &dl_g, &dl_g_smooth, resolution, sigma, self.smoothing);

        let mut dict: OrderedDict = Vec::new();
        push(&mut dict, "freqs, THz", x_new.clone());
        push(&mut dict, "a2f (lambda), THz", a2f_lambda);
        push(&mut dict, "a2f (gamma), THz", a2f_gamma);
        dict.extend(lambdas_a2f(&dict, integration));
        dict.extend(wlogs_a2f(&dict, integration));
        dict.extend(w2s_a2f(&dict, integration));
        self.a2f = dict;
        Ok(())
    }

    pub fn get_tc(&mut self, mu: f64) {
        self.mu = mu;
        let tc = tc_arrays(&mut self.direct, mu);
        self.direct.extend(tc);
        let tc = tc_arrays(&mut self.a2f, mu);
        self.a2f.extend(tc);
    }
}

/// `tc_mcm` + `tc_ad`（`scflow/tc.py`）。
fn tc_arrays(dict: &mut OrderedDict, mu: f64) -> OrderedDict {
    let wlog_l = get(dict, "wlog (lambda), K").cloned().unwrap_or_default();
    let wlog_g = get(dict, "wlog (gamma), K").cloned().unwrap_or_default();
    let lam_l = get(dict, "lambda (lambda)").cloned().unwrap_or_default();
    let lam_g = get(dict, "lambda (gamma)").cloned().unwrap_or_default();
    let w2_l = get(dict, "w2 (lambda), K").cloned().unwrap_or_default();
    let w2_g = get(dict, "w2 (gamma), K").cloned().unwrap_or_default();
    let mut out: OrderedDict = Vec::new();
    push(&mut out, "Tc_McM (lambda), K", mc_millan_tc_array(&wlog_l, &lam_l, mu));
    push(&mut out, "Tc_McM (gamma), K", mc_millan_tc_array(&wlog_g, &lam_g, mu));
    push(&mut out, "Tc_AD (lambda), K", allen_dynes_tc_array(&wlog_l, &w2_l, &lam_l, mu));
    push(&mut out, "Tc_AD (gamma), K", allen_dynes_tc_array(&wlog_g, &w2_g, &lam_g, mu));
    out
}

/// `sc_a2f.lambdas_a2f`：λ(ω) 累积（dtype=float32 存储）。
pub fn lambdas_a2f(a2f: &OrderedDict, integration: &str) -> OrderedDict {
    let freqs = get(a2f, "freqs, THz").cloned().unwrap_or_default();
    let a2f_lambda = get(a2f, "a2f (lambda), THz").cloned().unwrap_or_default();
    let a2f_gamma = get(a2f, "a2f (gamma), THz").cloned().unwrap_or_default();
    let n = freqs.len();
    let mut lambda_lambda = vec![0.0f64; n];
    let mut lambda_gamma = vec![0.0f64; n];
    for i in 1..n {
        if integration == "simps" {
            let int_l: Vec<f64> = (0..=i).map(|j| a2f_lambda[j] / freqs[j]).collect();
            let int_g: Vec<f64> = (0..=i).map(|j| a2f_gamma[j] / freqs[j]).collect();
            lambda_lambda[i] = as_f32(2.0 * simpson(&int_l, &freqs[0..=i]));
            lambda_gamma[i] = as_f32(2.0 * simpson(&int_g, &freqs[0..=i]));
        } else {
            let dw = freqs[2] - freqs[1];
            let sum_l: f64 = (0..=i).map(|j| a2f_lambda[j] / freqs[j]).sum();
            let sum_g: f64 = (0..=i).map(|j| a2f_gamma[j] / freqs[j]).sum();
            lambda_lambda[i] = as_f32(2.0 * sum_l * dw);
            lambda_gamma[i] = as_f32(2.0 * sum_g * dw);
        }
    }
    vec![
        ("lambda (lambda)".into(), lambda_lambda),
        ("lambda (gamma)".into(), lambda_gamma),
    ]
}

/// `sc_a2f.wlogs_a2f`：wlog(ω) 累积（dtype=float16 存储）。
///
/// 注意：Python 源码中 `wlog_lambda = wlog_gamma = np.zeros(...)` 是同一数组的别名，
/// 循环内 gamma 通道后写覆盖 lambda 通道，最终两个键都是 **gamma 通道**的值。
/// 这里按可观察行为复刻（两个键返回相同数组）。
pub fn wlogs_a2f(a2f: &OrderedDict, integration: &str) -> OrderedDict {
    let freqs = get(a2f, "freqs, THz").cloned().unwrap_or_default();
    let a2f_gamma = get(a2f, "a2f (gamma), THz").cloned().unwrap_or_default();
    let lambda_gamma = get(a2f, "lambda (gamma)").and_then(|v| v.last()).copied().unwrap_or(0.0);
    let n = freqs.len();
    let mut wlog = vec![0.0f64; n];
    for i in 1..n {
        if lambda_gamma > 0.0 {
            if integration == "simps" {
                let int_g: Vec<f64> = (0..=i)
                    .map(|j| a2f_gamma[j] * (freqs[j] / K_T_THZ).ln() / freqs[j])
                    .collect();
                wlog[i] = as_f16((2.0 / lambda_gamma * simpson(&int_g, &freqs[0..=i])).exp());
            } else {
                let dw = freqs[1] - freqs[0];
                let sum_g: f64 = (0..=i)
                    .map(|j| a2f_gamma[j] * (freqs[j] / K_T_THZ).ln() / freqs[j])
                    .sum();
                wlog[i] = as_f16((2.0 / lambda_gamma * sum_g * dw).exp());
            }
        }
    }
    // Python 别名行为：两个键共享同一数组（gamma 通道）
    vec![
        ("wlog (lambda), K".into(), wlog.clone()),
        ("wlog (gamma), K".into(), wlog),
    ]
}

/// `sc_a2f.w2s_a2f`：w2(ω) 累积（dtype=float16 存储；同 wlogs_a2f 的别名行为，
/// 两个键都是 gamma 通道的值）。
pub fn w2s_a2f(a2f: &OrderedDict, integration: &str) -> OrderedDict {
    let freqs = get(a2f, "freqs, THz").cloned().unwrap_or_default();
    let a2f_gamma = get(a2f, "a2f (gamma), THz").cloned().unwrap_or_default();
    let lambda_gamma = get(a2f, "lambda (gamma)").and_then(|v| v.last()).copied().unwrap_or(0.0);
    let n = freqs.len();
    let mut w2 = vec![0.0f64; n];
    for i in 0..n {
        if lambda_gamma > 0.0 {
            if integration == "simps" {
                let int_g: Vec<f64> =
                    (0..=i).map(|j| a2f_gamma[j] * freqs[j] / (K_T_THZ * K_T_THZ)).collect();
                w2[i] = as_f16((2.0 / lambda_gamma * simpson(&int_g, &freqs[0..=i])).sqrt());
            } else {
                let dw = freqs[1] - freqs[0];
                let sum_g: f64 =
                    (0..=i).map(|j| a2f_gamma[j] * freqs[j] / (K_T_THZ * K_T_THZ)).sum();
                w2[i] = as_f16((2.0 / lambda_gamma * sum_g * dw).sqrt());
            }
        }
    }
    vec![
        ("w2 (lambda), K".into(), w2.clone()),
        ("w2 (gamma), K".into(), w2),
    ]
}

// 引入 f16 舍入（direct.rs 提供）
pub use crate::direct::as_f16;

// cumulative_sum 保留引用避免 dead_code（sum 分支已内联）
#[allow(unused)]
fn _keep(y: &[f64], dx: f64) -> Vec<f64> {
    cumulative_sum(y, dx)
}

