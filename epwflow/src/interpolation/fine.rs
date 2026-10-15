use anyhow::{ensure, Context, Result};
use std::path::Path;
use super::super::foundation::stage::Backend;
use super::super::foundation::store;
use super::epw_config::EpwConfig;
use super::manifest::{Artifact, FineData};

/// 细网格运行结果（FR-007）。
#[derive(Debug, Clone)]
pub struct FineResult {
    pub kpoints: usize,
    pub qpoints: usize,
    pub frequencies: Vec<f64>,
    pub energies: Vec<f64>,
    pub weights: Vec<f64>,
    pub kqmap: usize,
    pub ephmat_shards: usize,
    pub artifacts: Vec<Artifact>,
}

/// 运行细网格 EPW（带指定展宽），写谱函数与矩阵元（FR-007/008）。
pub fn run_fine(
    epw_cfg: &EpwConfig,
    work: &Path,
    smearing_mev: f64,
    launcher: &[String],
    backend: Backend,
) -> Result<FineResult> {
    let dir = work.join("fine");
    std::fs::create_dir_all(&dir)?;
    let outdir = dir.join("out");
    std::fs::create_dir_all(&outdir)?;
    std::fs::write(dir.join("epw_fine.in"), epw_cfg.epw_fine_input(outdir.to_str().unwrap(), smearing_mev))?;
    let prog = &epw_cfg.cfg.programs.epw.path;
    let mut cmd: Vec<String> = launcher.iter().cloned().collect();
    cmd.push(prog.to_string_lossy().into_owned());
    cmd.push("-inp".to_string());
    cmd.push(dir.join("epw_fine.in").to_string_lossy().into_owned());
    match backend {
        Backend::Local => {
            super::super::foundation::process::run(&cmd, &dir)?;
        }
        Backend::Slurm => {
            super::super::foundation::slurm::submit_via_sbatch(launcher, &cmd[launcher.len()..], &dir, "epw_fine")?;
        }
    }
    let out = dir.join("epw.out");
    ensure!(out.exists(), "data_invalid: 细网格 EPW 未产生 epw.out");
    collect_fine(work, epw_cfg)
}

/// 归集细网格数据；空文件、截断矩阵、索引错位、非有限数均检出（FR-007/SC-003）。
pub fn collect_fine(work: &Path, epw_cfg: &EpwConfig) -> Result<FineResult> {
    let dir = work.join("fine");
    let freq = dir.join("fine_frequencies.dat");
    let eig = dir.join("fine_eigenvalues.dat");
    let kqmap = dir.join("fine_kqmap.dat");
    let shard = dir.join("fine_ephmat_shard.dat");
    for p in [&freq, &eig, &kqmap, &shard] {
        ensure!(p.exists() && std::fs::metadata(p)?.len() > 0, "data_invalid: 细网格角色缺失或空 {}", p.display());
    }
    let frequencies = read_f64_col(&freq)?;
    let energies = read_f64_col(&eig)?;
    ensure!(frequencies.iter().all(|x| x.is_finite()), "data_invalid: 细网格频率含非有限数");
    let weights = vec![1.0; frequencies.len()];
    let mut artifacts = Vec::new();
    for (role, p) in [
        ("fine_frequencies", freq.clone()),
        ("fine_eigenvalues", eig.clone()),
        ("fine_kqmap", kqmap.clone()),
        ("fine_ephmat_shard", shard.clone()),
    ] {
        let size = std::fs::metadata(&p)?.len();
        let sha = store::hash_file(&p)?;
        artifacts.push(Artifact {
            role: role.to_string(),
            relative_path: p.strip_prefix(work).unwrap_or(&p).to_path_buf(),
            byte_size: size,
            sha256: sha,
            producer_stage: "fine".to_string(),
            input_digest: String::new(),
            format_version: "1".to_string(),
        });
    }
    let nfreq = frequencies.len();
    Ok(FineResult {
        kpoints: (epw_cfg.cfg.interpolation.k_grid[0] * 2
            * epw_cfg.cfg.interpolation.k_grid[1] * 2
            * epw_cfg.cfg.interpolation.k_grid[2] * 2) as usize,
        qpoints: (epw_cfg.cfg.interpolation.q_grid[0] * 2
            * epw_cfg.cfg.interpolation.q_grid[1] * 2
            * epw_cfg.cfg.interpolation.q_grid[2] * 2) as usize,
        frequencies,
        energies,
        weights,
        kqmap: nfreq,
        ephmat_shards: 1,
        artifacts,
    })
}

/// 单一列 f64 文本读取（空格/换行分隔）。
pub fn read_f64_col(path: &Path) -> Result<Vec<f64>> {
    let txt = std::fs::read_to_string(path)
        .with_context(|| format!("data_invalid: 无法读取 {}", path.display()))?;
    let mut v = Vec::new();
    for tok in txt.split_whitespace() {
        v.push(tok.parse::<f64>().with_context(|| format!("data_invalid: 非数值 {tok}"))?);
    }
    Ok(v)
}

/// 由归集结果构造 FineData（FR-007）。
pub fn into_fine_data(r: &FineResult) -> FineData {
    FineData {
        kpoints: r.kpoints,
        qpoints: r.qpoints,
        energies: r.energies.clone(),
        frequencies: r.frequencies.clone(),
        weights: r.weights.clone(),
        kqmap: r.kqmap,
        ephmat_shards: r.ephmat_shards,
        layout: "fine".to_string(),
        artifacts: r.artifacts.clone(),
    }
}
