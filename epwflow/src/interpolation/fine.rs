use super::super::foundation::stage::Backend;
use super::super::foundation::store;
use super::epw_config::EpwConfig;
use super::manifest::{Artifact, FineData};
use anyhow::{ensure, Context, Result};
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

/// 默认单展宽路径；生产调度使用 run_fine_at 为各展宽隔离目录。
pub fn run_fine(
    epw_cfg: &EpwConfig,
    work: &Path,
    smearing_mev: f64,
    launcher: &[String],
    backend: Backend,
) -> Result<FineResult> {
    run_fine_at(
        epw_cfg,
        work,
        &work.join("fine"),
        smearing_mev,
        launcher,
        backend,
    )
}

pub fn run_fine_at(
    epw_cfg: &EpwConfig,
    work: &Path,
    dir: &Path,
    smearing_mev: f64,
    launcher: &[String],
    backend: Backend,
) -> Result<FineResult> {
    let structure = crate::qe_data::qe_config::QeSettings::from_config(epw_cfg.cfg)?;
    let coarse = work.join("coarse");
    super::restart::stage(
        &coarse,
        &coarse.join("out"),
        &epw_cfg.cfg.qe.prefix,
        epw_cfg.cfg.wannier.num_wann as usize,
        structure.cell.positions.len() * 3,
        dir,
    )?;
    std::fs::write(
        dir.join("epw_fine.in"),
        epw_cfg.epw_fine_input("./out", smearing_mev)?,
    )?;
    super::coarse::run_epw(epw_cfg, dir, "epw_fine.in", launcher, backend)?;
    collect_native_fine_at(work, dir, epw_cfg)
}

/// 归集细网格数据；空文件、截断矩阵、索引错位、非有限数均检出（FR-007/SC-003）。
pub fn collect_fine(work: &Path, epw_cfg: &EpwConfig) -> Result<FineResult> {
    collect_native_fine_at(work, &work.join("fine"), epw_cfg)
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
pub fn collect_native_fine_at(work: &Path, dir: &Path, epw_cfg: &EpwConfig) -> Result<FineResult> {
    use super::native_matrices::{self, FORMAT};
    use std::{
        collections::{BTreeMap, BTreeSet},
        fs,
    };
    let cfg = epw_cfg.cfg;
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
    // io_supercond.f90 的最终写入计数可检出完整头部后截断的矩阵。
    let restart_path = dir.join("restart.fmt");
    let restart = fs::read_to_string(&restart_path)?
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let selection = fs::read_to_string(dir.join("selecq.fmt"))?
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let selected = *selection.first().context("data_invalid: 缺少细网格选择数量")?;
    let qtotal = cfg.interpolation.q_grid.iter().map(|n| *n as u64).product::<u64>();
    ensure!(selected > 0 && selected <= qtotal
        && selection.get(1) == Some(&qtotal)
        && selection.len() as u64 == selected + 2
        && selection[2..].iter().all(|n| *n > 0 && *n <= qtotal)
        && selection[2..].iter().collect::<BTreeSet<_>>().len() as u64 == selected,
        "data_invalid: 细网格选择数量、完整 q 数或索引不符");
    ensure!(
        restart.len() == 4 + 2 * npool as usize
            && restart[0] == selected
            && selected > 0
            && restart[1] == 0
            && restart[2] == 0
            && restart[3] == npool as u64,
        "data_invalid: 细网格重启计数未完成或布局不符"
    );
    let mut covered = 0usize;
    let mut roles = vec![
        ("fine_restart".into(), restart_path),
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
        let expected_bytes = restart[3 + id]
            .checked_mul(8)
            .and_then(|x| x.checked_add(8))
            .context("data_invalid: 细矩阵尺寸溢出")?;
        ensure!(
            fs::metadata(&path)?.len() == expected_bytes,
            "data_invalid: 矩阵分片负载与最终写入计数不同"
        );
        let mut matrix_file = std::io::BufReader::new(fs::File::open(&path)?);
        matrix_file.read_exact(&mut header)?;
        let mut remaining = expected_bytes - 8;
        let mut buffer = [0u8; 65536];
        while remaining > 0 {
            let size = remaining.min(buffer.len() as u64) as usize;
            matrix_file.read_exact(&mut buffer[..size])?;
            ensure!(
                buffer[..size]
                    .chunks_exact(8)
                    .all(|b| f64::from_le_bytes(b.try_into().unwrap()).is_finite()),
                "data_invalid: 细矩阵含非有限数"
            );
            remaining -= size as u64;
        }
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
