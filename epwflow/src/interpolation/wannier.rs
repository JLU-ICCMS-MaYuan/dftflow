use super::super::foundation::stage::Backend;
use super::super::foundation::store;
use super::epw_config::EpwConfig;
use anyhow::{ensure, Context, Result};
use regex::Regex;
use std::path::Path;

/// Wannier 化成功结果（FR-004）。
#[derive(Debug, Clone)]
pub struct WannierResult {
    pub num_iter: usize,
    pub converged: bool,
    pub max_spread_angstrom2: f64,
    pub wannier_digest: String,
}

/// 解析 wannier90 .wout：迭代次数、收敛标志与最终最大 spread（FR-004）。
pub fn parse_wannier_log(log: &Path) -> Result<WannierResult> {
    let txt = std::fs::read_to_string(log)
        .with_context(|| format!("data_invalid: 无法读取 {}", log.display()))?;
    let conv_re = Regex::new(r"(?im)^\s*(?:\|\s*)?Convergence (?:was )?reached\b")
        .expect("wannier: 正则编译失败");
    let conv = conv_re.is_match(&txt);
    ensure!(
        !txt.to_lowercase().contains("convergence not reached")
            && !txt.to_lowercase().contains("convergence was not reached"),
        "data_invalid: Wannier 明确未收敛"
    );
    let iter_re =
        Regex::new(r"Number of iterations\s*[:=]\s*(\d+)").expect("wannier: 正则编译失败");
    let num_iter = iter_re
        .captures(&txt)
        .and_then(|c| c[1].parse::<usize>().ok())
        .unwrap_or(0);
    let spread_re = Regex::new(r"Spread\s*[=:]\s*([-\d.eE+]+)").expect("wannier: 正则编译失败");
    let spreads: Vec<f64> = spread_re
        .captures_iter(&txt)
        .filter_map(|c| c[1].parse::<f64>().ok())
        .collect();
    let max_spread = spreads.iter().cloned().fold(f64::NAN, f64::max);
    let finite = max_spread.is_finite() && max_spread > 0.0;
    ensure!(conv, "data_invalid: Wannier 未报告收敛");
    ensure!(finite, "data_invalid: Wannier 最终 spread 非有限或为零");
    Ok(WannierResult {
        num_iter,
        converged: conv && finite,
        max_spread_angstrom2: max_spread,
        wannier_digest: String::new(),
    })
}

fn exec(
    prog: &Path,
    args: &[&str],
    dir: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<()> {
    let mut cmd: Vec<String> = launcher.iter().cloned().collect();
    cmd.push(prog.to_string_lossy().into_owned());
    for a in args {
        cmd.push(a.to_string());
    }
    match backend {
        Backend::Local => {
            super::super::foundation::process::run(&cmd, dir)?;
        }
        Backend::Slurm => {
            super::super::foundation::slurm::submit_via_sbatch(
                launcher,
                &cmd[launcher.len()..],
                dir,
                "wannier",
            )?;
        }
    }
    Ok(())
}

/// 运行 Wannier 化（wannier90 + pw2wannier90），返回成功结果与来源摘要（FR-004）。
pub fn run_wannier(
    epw_cfg: &EpwConfig,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<WannierResult> {
    let dir = work.join("wannier");
    std::fs::create_dir_all(&dir)?;
    qf_wannier::write_selection(&dir.join("window-selection.json"), &epw_cfg.window_selection()?)?;
    qf_wannier::require_projectability_keywords(
        &epw_cfg.cfg.programs.wannier90.as_ref().context("configuration: 缺少 programs.wannier90")?.path,
    )?;
    let outdir = dir.join("out");
    std::fs::create_dir_all(&outdir)?;
    std::fs::write(dir.join("scf.win"), epw_cfg.wannier90_input()?)?;
    let pw2 = epw_cfg.pw2wannier90_input(outdir.to_str().unwrap())?;
    std::fs::write(dir.join("pw2wan.in"), &pw2)?;

    let w90 = &epw_cfg
        .cfg
        .programs
        .wannier90
        .as_ref()
        .context("configuration: 缺少 programs.wannier90")?
        .path;
    // 先 pw2wannier90 生成 .nnkp，再 wannier90 主计算。
    let pw2 = &epw_cfg
        .cfg
        .programs
        .pw2wannier90
        .as_ref()
        .context("configuration: 缺少 programs.pw2wannier90")?
        .path;
    exec(pw2, &["scf"], &dir, launcher, backend)?;
    exec(w90, &["scf"], &dir, launcher, backend)?;

    let log = dir.join("scf.wout");
    ensure!(
        log.exists(),
        "data_invalid: wannier90 未产生 scf.wout，禁止后续插值"
    );
    let mut res = parse_wannier_log(&log)?;
    // 来源摘要：对 wannier 输出目录做整体哈希。
    res.wannier_digest = store::hash_file(&log)?;
    Ok(res)
}
