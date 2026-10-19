//! 目标 EPW 5.9 little-endian / INTEGER(4) / REAL(8) 源适配。
//! 不从 Δ 文件行号猜测原始 k/band；按 read_eigenvalues 的压缩规则恢复。
use super::attempt::NativeInputs;
use crate::foundation::{
    config::Config,
    store::{self, Identity},
};
use crate::interpolation::manifest::{Artifact, FineData, InterpolationManifest, Spectrum};
pub use crate::interpolation::native_matrices::{eigenvalues, frequencies, FORMAT, RY_EV};
use anyhow::{ensure, Context, Result};
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path},
};

pub fn artifact_path(work: &Path, a: &Artifact) -> Result<std::path::PathBuf> {
    ensure!(
        !a.relative_path.is_absolute()
            && a.relative_path
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "data_invalid: 产物路径逃逸"
    );
    let path = work.join(&a.relative_path);
    let target = fs::canonicalize(&path)?;
    ensure!(
        target.starts_with(fs::canonicalize(work)?),
        "data_invalid: 产物链接逃逸"
    );
    ensure!(
        fs::metadata(&path)?.len() == a.byte_size && store::hash_file(&path)? == a.sha256,
        "input_conflict: F3 产物改变 {}",
        path.display()
    );
    Ok(path)
}

/// F3 清单必须交付原生角色；旧的简化 fine 文本不能伪装成 EPW 二进制数据。
pub fn bind(work: &Path, cfg: &Config, f3: &InterpolationManifest) -> Result<NativeInputs> {
    let fine = f3
        .fine
        .as_ref()
        .context("data_invalid: F3 缺少细网格数据")?;
    let evidence = Identity::capture(&work.join("interpolation/manifest.json"))?;
    bind_native_roles(work, cfg, fine, &f3.spectra, &evidence)
}

/// 原生阶段验收与完整 F3 共用角色校验。调用方负责验证质量门和来源物理参数；
/// 证据可以是阶段报告，不能因此声称完整自动链已经验收。
pub fn bind_native_roles(
    work: &Path,
    cfg: &Config,
    fine: &FineData,
    spectra: &[Spectrum],
    evidence: &Identity,
) -> Result<NativeInputs> {
    evidence.verify()?;
    ensure!(
        fine.layout == FORMAT,
        "data_invalid: F3 尚未交付原生 Eliashberg 细网格布局"
    );
    let mut roles = BTreeSet::new();
    for a in &fine.artifacts {
        ensure!(
            a.format_version == FORMAT && roles.insert(a.role.clone()),
            "data_invalid: 原生格式或重复角色"
        );
        artifact_path(work, a)?;
    }
    let get = |role: &str| -> Result<std::path::PathBuf> {
        artifact_path(
            work,
            fine.artifacts
                .iter()
                .find(|a| a.role == role)
                .with_context(|| format!("data_invalid: 缺少 F3 角色 {role}"))?,
        )
    };
    let ep = get("fine_egnv")?;
    let mut data = eigenvalues(&fs::read(&ep)?)?;
    let log = fs::read_to_string(get("fine_log")?)?;
    crate::interpolation::native::checked_log(&log, 0)?;
    let offset = crate::interpolation::native_matrices::band_offset(&log, data.bands)?;
    ensure!(
        offset + data.bands <= cfg.wannier.num_wann as usize,
        "data_invalid: 原生带范围超出 Wannier 子空间"
    );
    for s in &mut data.states {
        s.band += offset;
    }
    ensure!(
        data.grid == cfg.interpolation.k_grid
            && (data.fsthick_ev - cfg.interpolation.fsthick_ev).abs() < 1e-8
            && (data.degaussw_ev - cfg.interpolation.degaussw_ev).abs() < 1e-8,
        "input_conflict: egnv 与固定网格/费米面设置不同"
    );
    let fp = get("fine_freq")?;
    frequencies(&fs::read(&fp)?, cfg.interpolation.q_grid)?;
    let a2f = get("fine_a2f")?;
    let spectrum = crate::interpolation::native::NativeSpectrum::read(
        &a2f,
        cfg.interpolation.spectral_points as usize,
    )?;
    let selected = cfg.superconductivity.spectrum_index as usize;
    ensure!(
        selected < spectra.len() && spectrum.a2f.len() == 1,
        "data_invalid: F3 所选谱越界或原生谱列数无效"
    );
    let declared = &spectra[selected];
    ensure!(
        declared.energy_mev == spectrum.energy_mev && declared.a2f == spectrum.a2f[0],
        "input_conflict: F3 谱与原生来源不同"
    );
    let matrix_dir = ep.parent().context("data_invalid: egnv 父目录")?;
    ensure!(
        matrix_dir
            .file_name()
            .is_some_and(|n| n == format!("{}.ephmat", cfg.qe.prefix).as_str()),
        "data_invalid: 原生矩阵目录名不符"
    );
    let mut files = vec![ep.clone(), fp, get("fine_ikmap")?, get("fine_npool")?];
    for a in &fine.artifacts {
        if a.role.starts_with("fine_ephmat_") {
            files.push(artifact_path(work, a)?);
        }
    }
    ensure!(
        files.len() >= 5 && files.iter().all(|p| p.parent() == Some(matrix_dir)),
        "data_invalid: 原生矩阵分片缺失或目录混用"
    );
    let ids = files
        .iter()
        .map(|p| Identity::capture(p))
        .collect::<Result<Vec<_>>>()?;
    evidence.verify()?;
    Ok(NativeInputs {
        interpolation_digest: evidence.sha256.clone(),
        prefix: cfg.qe.prefix.clone(),
        k_grid: cfg.interpolation.k_grid,
        q_grid: cfg.interpolation.q_grid,
        spectral_points: cfg.interpolation.spectral_points as usize,
        eps_acoustic_cm1: cfg.interpolation.eps_acoustic_cm1,
        a2f: Identity::capture(&a2f)?,
        crystal: Identity::capture(&get("fine_crystal")?)?,
        selecq: Some(Identity::capture(&get("fine_selecq")?)?),
        matrix_outdir: matrix_dir
            .parent()
            .context("data_invalid: 缺少 outdir")?
            .into(),
        matrix_files: ids,
        states: data.states,
    })
}
