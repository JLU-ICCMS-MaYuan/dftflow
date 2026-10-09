//! QE 超导后处理核心库（复刻 Python `scflow`）：
//! α²F 读取、λ/wlog/w2 累积量、McMillan / Allen-Dynes / Eliashberg Tc 求解。
//!
//! 设计取舍：数值内核用纯 Rust（含手写的 Jacobi 本征值求解）实现，避免引入 OpenBLAS
//! 静态链接的构建复杂度；完整的 dyn*.elph 解析与插值流水线在后续阶段接入，
//! 本 crate 当前接受"单点" α²F 曲线（freq/λ/γ 三列，单位 THz）作为垂直切片输入。

pub mod a2f;
pub mod config;
pub mod constants;
pub mod eig;
pub mod integ;
pub mod tc;

use anyhow::Result;
use serde::Serialize;

pub use a2f::A2f;
pub use config::{Integration, ScflowConfig};
pub use tc::{allen_dynes_tc, eliashberg_tc, mc_millan_tc};

/// scflow 单点求解结果。
#[derive(Debug, Clone, Serialize)]
pub struct ScflowResult {
    pub lambda_total: f64,
    pub wlog_total: f64,
    pub w2_total: f64,
    pub tc_mcmillan: f64,
    pub tc_allen_dynes: f64,
    pub tc_eliashberg: f64,
    pub eliashberg_dim: usize,
    pub solver_status: String,
}

/// 执行完整求解：读取 a2F -> λ/wlog/w2 -> 三种 Tc。
pub fn solve(config: &ScflowConfig) -> Result<ScflowResult> {
    let a2f_path = config.work_dir.join(&config.a2f_file);
    let a2f = A2f::load(&a2f_path)?;

    let (lam_l, lam_g) = a2f::compute_lambda(&a2f, config.integration);
    let lam_l_total = *lam_l.last().unwrap();
    let lam_g_total = *lam_g.last().unwrap();

    let (wl_l, wl_g) = a2f::compute_wlog(&a2f, lam_g_total, config.integration);
    let (w2_l, w2_g) = a2f::compute_w2(&a2f, lam_g_total, config.integration);

    let lambda = lam_g_total; // gamma 通道对应 Python 的 "Tc (gamma)" 报告
    let wlog = *wl_g.last().unwrap();
    let w2 = *w2_g.last().unwrap();

    let tc_mcm = mc_millan_tc(wlog, lambda, config.mu);
    let tc_ad = allen_dynes_tc(wlog, w2, lambda, config.mu);
    let (tc_e, status) = eliashberg_tc(&a2f, config.mu, config.eliashberg_dim, config.integration);

    Ok(ScflowResult {
        lambda_total: lambda,
        wlog_total: wlog,
        w2_total: w2,
        tc_mcmillan: tc_mcm,
        tc_allen_dynes: tc_ad,
        tc_eliashberg: tc_e,
        eliashberg_dim: config.eliashberg_dim,
        solver_status: status,
    })
}

/// 读取配置、求解并打印 + 写出 `result.json`（CLI 入口）。
pub fn run(config: &ScflowConfig) -> Result<()> {
    let result = solve(config)?;
    println!(
        "[scflow] a2F 读取自 {}",
        config.work_dir.join(&config.a2f_file).display()
    );
    println!("[scflow] lambda(gamma) = {:.4}", result.lambda_total);
    println!("[scflow] wlog  = {:.2} K", result.wlog_total);
    println!("[scflow] w2    = {:.2} K", result.w2_total);
    println!("[scflow] McMillan Tc     = {:.2} K", result.tc_mcmillan);
    println!("[scflow] Allen-Dynes Tc = {:.2} K", result.tc_allen_dynes);
    println!(
        "[scflow] Eliashberg Tc  = {:.3} K (dim={}, status={})",
        result.tc_eliashberg, result.eliashberg_dim, result.solver_status
    );
    let json = serde_json::to_string_pretty(&result)?;
    let out = config.work_dir.join("result.json");
    std::fs::write(&out, json).with_context(|| format!("write {}", out.display()))?;
    println!("[scflow] 结果已写入 {}", out.display());
    Ok(())
}
