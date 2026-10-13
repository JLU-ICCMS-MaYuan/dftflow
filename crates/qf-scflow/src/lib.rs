//! QE 超导后处理核心库（复刻 Python `scflow`）：
//! `*.dyn*.elph*` / `elph.in_lambda.*` 读取、direct/interp 双通道、λ/wlog/w2、
//! McMillan / Allen-Dynes / Eliashberg Tc、分展宽 CSV / result.json /
//! spglib 对称化 CIF / 三类 PDF（plotters SVG + svg2pdf）。

pub mod a2f;
pub mod config;
pub mod constants;
pub mod direct;
pub mod epc;
pub mod eig;
pub mod integ;
pub mod interp;
pub mod outputs;
pub mod plots;
pub mod structure_out;
pub mod system;
pub mod tc;

use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

pub use a2f::A2f;
pub use config::{Integration, ScflowConfig};
pub use epc::{Dyn, DynElph, Folder, PhOuts};
pub use structure_out::StructureOut;
pub use tc::{allen_dynes_tc, eliashberg_tc, mc_millan_tc};

/// scflow 单点求解结果（兼容旧单点路径与测试）。
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

/// 执行完整求解：读取 a2F -> λ/wlog/w2 -> 三种 Tc（单点 α²F 输入）。
pub fn solve(config: &ScflowConfig) -> Result<ScflowResult> {
    let a2f_path = config.work_dir.join(&config.a2f_file);
    let a2f = A2f::load(&a2f_path)?;

    let (lam_l, lam_g) = a2f::compute_lambda(&a2f, config.integration);
    let lam_g_total = *lam_g.last().unwrap();

    let (wl_l, wl_g) = a2f::compute_wlog(&a2f, lam_g_total, config.integration);
    let (w2_l, w2_g) = a2f::compute_w2(&a2f, lam_g_total, config.integration);

    let lambda = lam_g_total;
    let wlog = *wl_g.last().unwrap();
    let w2 = *w2_g.last().unwrap();

    let tc_mcm = mc_millan_tc(wlog, lambda, config.mu);
    let tc_ad = allen_dynes_tc(wlog, w2, lambda, config.mu);
    let (tc_e, status) = eliashberg_tc(&a2f, config.mu, config.eliashberg_dim, config.integration);

    let _ = lam_l;
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

/// 权重解析：*.dyn* → ph.out → 均匀（复刻 `main.py` 的三级回退）。
fn resolve_weights(
    folder: &Folder,
    dyns: &[Dyn],
    dyn_elphs: &[DynElph],
    ph_outs: Option<&PhOuts>,
) -> Vec<([f64; 3], f64)> {
    if !dyns.is_empty() && folder.dyn_paths.len() == folder.dyn_elph_paths.len() {
        println!("Everything's ok with the weights. They will be used from *.dyn* files.");
        let sum: usize = dyns.iter().map(|d| d.weight).sum();
        let weights: Vec<([f64; 3], f64)> = dyns
            .iter()
            .map(|d| (d.q_point, d.weight as f64 / sum as f64))
            .collect();
        return weights;
    }
    if !dyns.is_empty() {
        println!(
            "WARNING: The number of *.dyn* files is not equal to the number of *.dyn*.elph* files. Weights from *.dyn* files won't be used."
        );
    }
    if let Some(ph) = ph_outs {
        println!("Trying to get weights out of ph.out ...");
        let weights = ph.weights();
        if weights.len() != folder.dyn_elph_paths.len() {
            println!(
                "WARNING: Number of weights found in ph.out is not equal to the number of *.dyn*.elph* files. Weights from ph.out won't be used."
            );
        } else {
            println!("Everything's ok with the weights. They will be used from ph.out files.");
            // q 点与 dyn_elph 对齐（compare_q_points，前 5 位小数）
            let mut result: Vec<([f64; 3], f64)> = Vec::new();
            for dyn_elph in dyn_elphs {
                for (q, w) in &weights {
                    if (dyn_elph.q_point[0] * 1e5).round() / 1e5 == (q[0] * 1e5).round() / 1e5
                        && (dyn_elph.q_point[1] * 1e5).round() / 1e5 == (q[1] * 1e5).round() / 1e5
                        && (dyn_elph.q_point[2] * 1e5).round() / 1e5 == (q[2] * 1e5).round() / 1e5
                    {
                        result.push((dyn_elph.q_point, *w));
                        break;
                    }
                }
            }
            if result.len() == dyn_elphs.len() {
                return result;
            }
            // q 点对不齐时退回均匀
            let uniform = 1.0 / dyn_elphs.len() as f64;
            return dyn_elphs.iter().map(|d| (d.q_point, uniform)).collect();
        }
    }
    println!("WARNING: Weights are not found. They will be set to equal values for all q-points");
    let uniform = 1.0 / dyn_elphs.len() as f64;
    dyn_elphs.iter().map(|d| (d.q_point, uniform)).collect()
}

/// 完整生产链路（复刻 `scflow/main.py: run`）。
pub fn run(config: &ScflowConfig) -> Result<()> {
    std::fs::create_dir_all(&config.results_dir)?;
    let folder = Folder::new(&config.work_dir)?;
    let mut dyn_elphs: Vec<DynElph> = folder
        .dyn_elph_paths
        .iter()
        .map(|p| DynElph::new(p))
        .collect::<Result<Vec<_>>>()?;
    let dyns: Vec<Dyn> = folder
        .dyn_paths
        .iter()
        .map(|p| Dyn::new(p))
        .collect::<Result<Vec<_>>>()?;

    let ph_outs = if !folder.ph_outs_paths.is_empty() {
        Some(PhOuts::new(&folder.ph_outs_paths))
    } else {
        println!("WARNING: Found no ph.out file(s). The structure will not be read and written.");
        None
    };

    let structure = match &ph_outs {
        Some(ph) => match ph.structure() {
            Ok(s) => {
                outputs::save_structure(&s, config.structure_tolerance, &config.results_dir.to_string_lossy())?;
                Some(s)
            }
            Err(e) => {
                println!("WARNING: 结构解析失败: {e}");
                None
            }
        },
        None => None,
    };

    let weights = resolve_weights(&folder, &dyns, &dyn_elphs, ph_outs.as_ref());

    let smoothing = if config.smoothing_thz.fract() == 0.0 {
        config.smoothing_thz.trunc()
    } else {
        config.smoothing_thz
    };
    let sigma = if config.gaussian_sigma.fract() == 0.0 {
        config.gaussian_sigma.trunc()
    } else {
        config.gaussian_sigma
    };

    let lengths: Vec<usize> = dyn_elphs.iter().map(|d| d.headers.len()).collect();
    if lengths.is_empty() || !lengths.iter().all(|l| *l == lengths[0]) {
        anyhow::bail!("所有 *.dyn*.elph* 文件必须包含相同数量的展宽区段");
    }
    let length = lengths[0];
    let mut headers: Vec<Option<f64>> = Vec::new();
    if length == 1 {
        println!("Assuming a Tetrahedron method");
    } else {
        println!("Assuming a Gaussian Broadening method with broadenings:");
        headers = dyn_elphs[0].headers.clone();
        println!("{headers:?}");
    }

    let mut summary: std::collections::BTreeMap<String, Vec<f64>> = std::collections::BTreeMap::new();
    let formula = structure
        .as_ref()
        .map(|s| s.formula(false).0)
        .unwrap_or_else(|| "Unknown".to_string());

    let dyn_elphs_full = dyn_elphs.clone();
    for index in 0..length {
        let broadening = if length > 1 {
            println!("\nBroadening: {:?}", headers[index]);
            headers[index].context("Gaussian Broadening 表头应为数值")?
        } else {
            0.0
        };

        // 选定展宽区段（对齐 Python 的 deepcopy：每轮从完整区段重新选取）
        let dyn_elphs: Vec<DynElph> = dyn_elphs_full
            .iter()
            .map(|d| {
                let mut c = d.clone();
                c.lambdas = vec![d.lambdas[index].clone()];
                c.gammas = vec![d.gammas[index].clone()];
                c.dos = vec![d.dos[index]];
                c.e_f = vec![d.e_f[index]];
                c
            })
            .collect();

        let zero_modes = outputs::zero_modes(&dyn_elphs, &folder.dyn_elph_paths);
        if !zero_modes.is_empty() {
            println!(
                "WARNING: {} exact-zero modes: excluding only if lambda=0; original gamma is recorded in result.json",
                zero_modes.len()
            );
        }

        let mut sys = system::System::new(dyn_elphs.clone(), &weights);
        sys.get_direct(smoothing)?;
        outputs::print_direct(&sys.direct, sys.smoothing);
        sys.get_a2f(config.resolution, config.gaussian_sigma, config.integration.integration_str())?;
        outputs::print_a2f(&sys.a2f, sys.resolution, sys.sigma);
        sys.get_tc(config.mu);
        outputs::print_tc(&sys.direct, &sys.a2f, sys.mu);

        let result_dir = if length > 1 {
            let dir = config.results_dir.join(dyn_elphs_full[0].headers_text[index].clone());
            std::fs::create_dir_all(&dir)?;
            dir
        } else {
            config.results_dir.clone()
        };
        let result_dir_text = result_dir.to_string_lossy().into_owned();

        outputs::save_dict(&sys.direct, &result_dir_text, &format!("direct_s{}", outputs::format_smoothing_raw(smoothing)), broadening)?;
        outputs::save_dict(
            &sys.a2f,
            &result_dir_text,
            &format!(
                "a2f_s{}_r{}_g{}",
                outputs::format_smoothing_raw(smoothing),
                config.resolution,
                outputs::format_smoothing_raw(config.gaussian_sigma)
            ),
            broadening,
        )?;
        plots::plot_system(&sys.direct, &sys.a2f, &formula, smoothing, config.resolution, config.gaussian_sigma, config.mu, &result_dir_text, broadening)?;
        plots::plot_article_view(&sys.direct, &sys.a2f, &formula, &result_dir_text)?;

        // Eliashberg（用 a2f 的 gamma 通道）
        let a2f = crate::a2f::A2f {
            freqs: system::get(&sys.a2f, "freqs, THz").cloned().context("a2f freqs")?,
            a2f_lambda: system::get(&sys.a2f, "a2f (lambda), THz").cloned().context("a2f lambda")?,
            a2f_gamma: system::get(&sys.a2f, "a2f (gamma), THz").cloned().context("a2f gamma")?,
        };
        let (tc_e, _status) = eliashberg_tc(&a2f, config.mu, config.eliashberg_dim, config.integration);
        let nef = dyn_elphs[0].dos[0];
        println!("nef: {nef}");
        let result = outputs::build_result(
            &sys.direct,
            &sys.a2f,
            tc_e,
            nef,
            structure.as_ref(),
            smoothing,
            config.resolution,
            config.mu,
            broadening,
            zero_modes,
        );
        outputs::save_result(&result, &result_dir_text)?;

        if length > 1 {
            outputs::update_summary(
                &mut summary,
                &result,
                broadening,
                &formula,
                smoothing,
                config.mu,
                &config.results_dir.to_string_lossy(),
            )?;
            plots::plot_summary(
                &summary["Broadening, Ry"],
                &summary,
                &formula,
                smoothing,
                config.gaussian_sigma,
                config.resolution,
                config.mu,
                &config.results_dir.to_string_lossy(),
            )?;
        }
    }
    Ok(())
}

/// 兼容占位（供外部调用探测单点路径）。
pub fn single_point_available(config: &ScflowConfig) -> bool {
    config.work_dir.join(&config.a2f_file).exists()
}

use std::path::Path as _PathAlias;
