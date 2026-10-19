use anyhow::{ensure, Result};
use std::path::Path;
use super::super::foundation::stage::Backend;
use super::super::foundation::store;
use super::epw_config::EpwConfig;
use super::manifest::{Artifact, CoarseData};

/// 粗网格 EPW 运行结果（FR-005）。
#[derive(Debug, Clone)]
pub struct CoarseResult {
    pub kpoints: usize,
    pub qpoints: usize,
    pub nwann: usize,
    pub artifacts: Vec<Artifact>,
}

fn run_epw(epw_cfg: &EpwConfig, dir: &Path, input_name: &str, launcher: &[String], backend: Backend) -> Result<()> {
    let prog = &epw_cfg.cfg.programs.epw.path;
    let mut cmd: Vec<String> = launcher.iter().cloned().collect();
    cmd.push(prog.to_string_lossy().into_owned());
    cmd.push("-inp".to_string());
    cmd.push(dir.join(input_name).to_string_lossy().into_owned());
    match backend {
        Backend::Local => {
            super::super::foundation::process::run(&cmd, dir)?;
        }
        Backend::Slurm => {
            super::super::foundation::slurm::submit_via_sbatch(launcher, &cmd[launcher.len()..], dir, "epw_coarse")?;
        }
    }
    Ok(())
}

/// 运行粗网格 EPW，写 ephmat（FR-005）。
pub fn run_coarse(
    epw_cfg: &EpwConfig,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<CoarseResult> {
    let dir = work.join("coarse");
    std::fs::create_dir_all(&dir)?;
    qf_wannier::write_selection(&dir.join("window-selection.json"), &epw_cfg.window_selection()?)?;
    qf_wannier::require_projectability_keywords(&epw_cfg.cfg.programs.epw.path)?;
    let outdir = dir.join("out");
    std::fs::create_dir_all(&outdir)?;
    epw_cfg.write_quality_points(&dir)?;
    std::fs::write(dir.join("epw_coarse.in"), epw_cfg.epw_coarse_input(outdir.to_str().unwrap())?)?;
    run_epw(epw_cfg, &dir, "epw_coarse.in", launcher, backend)?;
    let out = dir.join("epw.out");
    ensure!(out.exists(), "data_invalid: 粗网格 EPW 未产生 epw.out");
    collect_coarse(work, epw_cfg)
}

/// 归集粗网格数据并检查完整性；缺任一必需角色即拒绝（FR-005/007）。
pub fn collect_coarse(work: &Path, epw_cfg: &EpwConfig) -> Result<CoarseResult> {
    let dir = work.join("coarse");
    let out = dir.join("epw.out");
    let ephmat = dir.join("coarse.ephmat");
    ensure!(out.exists() && std::fs::metadata(&out)?.len() > 0, "data_invalid: 粗网格 epw.out 缺失或空");
    ensure!(ephmat.exists() && std::fs::metadata(&ephmat)?.len() > 0, "data_invalid: 粗网格 ephmat 缺失或空");
    let mut artifacts = Vec::new();
    for (role, p) in [("epw_coarse_data", out.clone()), ("coarse_ephmat", ephmat.clone())] {
        let size = std::fs::metadata(&p)?.len();
        let sha = store::hash_file(&p)?;
        artifacts.push(Artifact {
            role: role.to_string(),
            relative_path: p.strip_prefix(work).unwrap_or(&p).to_path_buf(),
            byte_size: size,
            sha256: sha,
            producer_stage: "coarse".to_string(),
            input_digest: String::new(),
            format_version: "1".to_string(),
        });
    }
    Ok(CoarseResult {
        kpoints: (epw_cfg.cfg.interpolation.k_grid[0]
            * epw_cfg.cfg.interpolation.k_grid[1]
            * epw_cfg.cfg.interpolation.k_grid[2]) as usize,
        qpoints: (epw_cfg.cfg.interpolation.q_grid[0]
            * epw_cfg.cfg.interpolation.q_grid[1]
            * epw_cfg.cfg.interpolation.q_grid[2]) as usize,
        nwann: epw_cfg.cfg.wannier.num_wann as usize,
        artifacts,
    })
}

/// 由归集结果构造可复用的 CoarseData（FR-010 复用来源）。
pub fn into_coarse_data(r: &CoarseResult) -> CoarseData {
    CoarseData {
        kpoints: r.kpoints,
        qpoints: r.qpoints,
        nwann: r.nwann,
        ephmat_role: "coarse_ephmat".to_string(),
        artifacts: r.artifacts.clone(),
    }
}
