use super::super::foundation::stage::Backend;
use super::super::foundation::store;
use super::epw_config::EpwConfig;
use super::manifest::{Artifact, FineData};
use anyhow::{bail, ensure, Context, Result};
use std::path::Path;

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
    std::fs::write(
        dir.join("epw_fine.in"),
        epw_cfg.epw_fine_input(outdir.to_str().unwrap(), smearing_mev)?,
    )?;
    let prog = &epw_cfg.cfg.programs.epw.path;
    let mut cmd: Vec<String> = launcher.iter().cloned().collect();
    cmd.push(prog.to_string_lossy().into_owned());
    cmd.push("-inp".to_string());
    cmd.push(dir.join("epw_fine.in").to_string_lossy().into_owned());
    match backend {
        Backend::Local => {
            let report = super::super::foundation::process::run(&cmd, &dir)?;
            std::fs::copy(&report.stdout_path, dir.join("epw.out"))?;
            std::fs::copy(&report.stderr_path, dir.join("epw.err"))?;
            super::native::checked_log(&std::fs::read_to_string(dir.join("epw.out"))?, 0)?;
        }
        Backend::Slurm => {
            bail!("allocation: 细网格须在 F1 已登记分配内执行，不能异步提交后立即归集");
        }
    }
    let out = dir.join("epw.out");
    ensure!(out.exists(), "data_invalid: 细网格 EPW 未产生 epw.out");
    collect_fine(work, epw_cfg)
}

/// 归集细网格数据；空文件、截断矩阵、索引错位、非有限数均检出（FR-007/SC-003）。
pub fn collect_fine(work: &Path, epw_cfg: &EpwConfig) -> Result<FineResult> {
    let dir = work.join("fine");
    if dir
        .join("out")
        .join(format!("{}.ephmat", epw_cfg.cfg.qe.prefix))
        .exists()
    {
        return collect_native_fine(work, epw_cfg);
    }
    let freq = dir.join("fine_frequencies.dat");
    let eig = dir.join("fine_eigenvalues.dat");
    let kqmap = dir.join("fine_kqmap.dat");
    let shard = dir.join("fine_ephmat_shard.dat");
    for p in [&freq, &eig, &kqmap, &shard] {
        ensure!(
            p.exists() && std::fs::metadata(p)?.len() > 0,
            "data_invalid: 细网格角色缺失或空 {}",
            p.display()
        );
    }
    let frequencies = read_f64_col(&freq)?;
    let energies = read_f64_col(&eig)?;
    ensure!(
        frequencies.iter().all(|x| x.is_finite()),
        "data_invalid: 细网格频率含非有限数"
    );
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
        kpoints: (epw_cfg.cfg.interpolation.k_grid[0]
            * 2
            * epw_cfg.cfg.interpolation.k_grid[1]
            * 2
            * epw_cfg.cfg.interpolation.k_grid[2]
            * 2) as usize,
        qpoints: (epw_cfg.cfg.interpolation.q_grid[0]
            * 2
            * epw_cfg.cfg.interpolation.q_grid[1]
            * 2
            * epw_cfg.cfg.interpolation.q_grid[2]
            * 2) as usize,
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
        v.push(
            tok.parse::<f64>()
                .with_context(|| format!("data_invalid: 非数值 {tok}"))?,
        );
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
        layout: if r
            .artifacts
            .iter()
            .all(|a| a.format_version == super::native_matrices::FORMAT)
        {
            super::native_matrices::FORMAT.to_string()
        } else {
            "fine".to_string()
        },
        artifacts: r.artifacts.clone(),
    }
}

/// 原生角色直接来自 EPW，不把简化文本转写成二进制矩阵元。
fn collect_native_fine(work: &Path, epw_cfg: &EpwConfig) -> Result<FineResult> {
    use super::native_matrices::{self, FORMAT};
    use std::{
        collections::{BTreeMap, BTreeSet},
        fs,
    };
    let cfg = epw_cfg.cfg;
    let dir = work.join("fine");
    super::native::checked_log(&fs::read_to_string(dir.join("epw.out"))?, 0)?;
    let matrix = dir.join("out").join(format!("{}.ephmat", cfg.qe.prefix));
    let e = native_matrices::eigenvalues(&fs::read(matrix.join("egnv"))?)?;
    native_matrices::band_offset(&fs::read_to_string(dir.join("epw.out"))?, e.bands)?;
    ensure!(
        e.grid == cfg.interpolation.k_grid,
        "data_invalid: 原生 egnv 网格不符"
    );
    let freq =
        native_matrices::frequencies(&fs::read(matrix.join("freq"))?, cfg.interpolation.q_grid)?;
    let ktotal = cfg
        .interpolation
        .k_grid
        .iter()
        .map(|x| *x as usize)
        .product::<usize>();
    let ikmap = fs::read(matrix.join("ikmap"))?;
    ensure!(
        ikmap.len() == ktotal * 8,
        "data_invalid: 原生 ikmap 长度不符"
    );
    let nkfs = e
        .states
        .iter()
        .map(|s| s.kpoint)
        .collect::<BTreeSet<_>>()
        .len();
    for (i, bytes) in ikmap.chunks_exact(4).enumerate() {
        let n = i32::from_le_bytes(bytes.try_into().unwrap());
        ensure!(
            n >= 0 && n as usize <= if i < ktotal { nkfs } else { e.kpoints },
            "data_invalid: 原生 ikmap 索引越界"
        );
    }
    let npool = fs::read(matrix.join("npool"))?;
    ensure!(npool.len() == 4, "data_invalid: 原生 npool 格式不符");
    let npool = i32::from_le_bytes(npool.try_into().unwrap());
    ensure!(npool > 0 && npool <= 64, "data_invalid: 原生矩阵分片数无效");
    let mut shards = BTreeMap::new();
    for file in fs::read_dir(&matrix)? {
        let path = file?.path();
        let name = path.file_name().unwrap().to_string_lossy();
        if let Some(suffix) = name.strip_prefix("ephmat") {
            let id = suffix
                .parse::<usize>()
                .context("data_invalid: 矩阵分片编号无效")?;
            ensure!(
                shards.insert(id, path).is_none(),
                "data_invalid: 重复矩阵分片"
            );
        }
    }
    ensure!(
        shards.len() == npool as usize && shards.keys().copied().eq(1..=npool as usize),
        "data_invalid: 缺少原生矩阵分片"
    );
    let mut covered = 0usize;
    let mut roles = vec![
        ("fine_egnv".to_string(), matrix.join("egnv")),
        ("fine_freq".into(), matrix.join("freq")),
        ("fine_ikmap".into(), matrix.join("ikmap")),
        ("fine_npool".into(), matrix.join("npool")),
        ("fine_crystal".into(), dir.join("crystal.fmt")),
        ("fine_selecq".into(), dir.join("selecq.fmt")),
        (
            "fine_a2f".into(),
            dir.join(format!("{}.a2f", cfg.qe.prefix)),
        ),
        ("fine_log".into(), dir.join("epw.out")),
    ];
    for (id, path) in shards {
        use std::io::Read;
        let mut header = [0u8; 8];
        fs::File::open(&path)?.read_exact(&mut header)?;
        let n = i32::from_le_bytes(header[4..8].try_into().unwrap());
        ensure!(
            i32::from_le_bytes(header[..4].try_into().unwrap()) == id as i32
                && n >= 0
                && fs::metadata(&path)?.len() % 8 == 0,
            "data_invalid: 矩阵分片头/布局错误"
        );
        covered += n as usize;
        roles.push((format!("fine_ephmat_{id:04}"), path));
    }
    ensure!(covered == nkfs, "data_invalid: 矩阵分片未覆盖全部费米面 k");
    let mut artifacts = vec![];
    for (role, path) in roles {
        let byte_size = fs::metadata(&path)?.len();
        ensure!(byte_size > 0, "data_invalid: 原生角色为空");
        artifacts.push(Artifact {
            role,
            relative_path: path.strip_prefix(work)?.into(),
            byte_size,
            sha256: store::hash_file(&path)?,
            producer_stage: "fine".into(),
            input_digest: store::hash_file(&dir.join("epw_fine.in"))?,
            format_version: FORMAT.into(),
        });
    }
    Ok(FineResult {
        kpoints: e.kpoints,
        qpoints: cfg
            .interpolation
            .q_grid
            .iter()
            .map(|x| *x as usize)
            .product(),
        frequencies: freq.iter().map(|w| w / 0.123984).collect(),
        energies: e.energies_ev,
        weights: e.states.iter().map(|s| s.weight).collect(),
        kqmap: ktotal,
        ephmat_shards: npool as usize,
        artifacts,
    })
}
