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

/// 将独立求解展宽与原生矩阵路线固定在现有输入视图中，不改科学源文件。
fn configured_stages(cfg: &Config) -> Result<serde_json::Value> {
    use crate::foundation::{config::Mode, native_config::SUPER};
    let mut stage: serde_json::Value = cfg.native_value(&[SUPER])?;
    for mode in [Mode::Iso, Mode::Aniso] {
        if cfg.superconductivity.mode != Mode::Both && cfg.superconductivity.mode != mode { continue; }
        let name = if mode == Mode::Iso { "iso" } else { "aniso" };
        let input = &mut stage[name]["epw"]["INPUTEPW"];
        let explicit = input.get("degaussq").is_some();
        input["degaussq"] = serde_json::json!(super::batch::smearing(cfg, mode)?);
        if explicit {
            ensure!(input.get("fila2f").is_none_or(|v| v.as_str().is_some_and(|s|s.trim().is_empty())),
                "configuration: independent superconductivity degaussq requires matrix input; set fila2f='' instead of reusing a fixed spectrum");
            input["fila2f"] = serde_json::json!("");
        }
        // 回填后的原生值不再同时声明为运行时派生，避免输入所有权冲突。
        if let Some(derived) = stage["workflow"]["derived"].as_object_mut() {
            derived.remove(&format!("{name}.epw.INPUTEPW.degaussq"));
            if explicit { derived.remove(&format!("{name}.epw.INPUTEPW.fila2f")); }
        }
    }
    Ok(serde_json::json!({SUPER: stage}))
}

pub fn artifact_path(work: &Path, a: &Artifact) -> Result<std::path::PathBuf> {
    ensure!(
        !a.relative_path.is_absolute()
            && a.relative_path
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "data_invalid: artifact path escapes its root"
    );
    let path = work.join(&a.relative_path);
    let target = fs::canonicalize(&path)?;
    ensure!(
        target.starts_with(fs::canonicalize(work)?),
        "data_invalid: artifact symlink escapes its root"
    );
    if crate::foundation::layout::enabled(work) {return Ok(path);}
    ensure!(
        fs::metadata(&path)?.len() == a.byte_size && store::hash_file(&path)? == a.sha256,
        "input_conflict: F3 artifact changed {}",
        path.display()
    );
    Ok(path)
}

/// F3 清单必须交付原生角色；旧的简化 fine 文本不能伪装成 EPW 二进制数据。
pub fn bind(work: &Path, cfg: &Config, f3: &InterpolationManifest) -> Result<NativeInputs> {
    if crate::foundation::layout::enabled(work) {return native_selected(cfg);}

    let fine = f3
        .fine
        .as_ref()
        .context("data_invalid: F3 fine-grid data missing")?;
    let evidence = if crate::foundation::layout::enabled(work) {
        let file = fine
            .artifacts
            .iter()
            .find(|a| a.role == "fine_log")
            .context("data_invalid: native fine-grid log missing")?;
        Identity::reference(&artifact_path(work, file)?)
    } else {
        Identity::capture(&crate::foundation::layout::path(
            work,
            "interpolation/manifest.json",
        ))?
    };
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
        "data_invalid: F3 native Eliashberg fine-grid layout not delivered"
    );
    let mut roles = BTreeSet::new();
    for a in &fine.artifacts {
        ensure!(
            a.format_version == FORMAT && roles.insert(a.role.clone()),
            "data_invalid: invalid native format or duplicate role"
        );
        artifact_path(work, a)?;
    }
    let get = |role: &str| -> Result<std::path::PathBuf> {
        artifact_path(
            work,
            fine.artifacts
                .iter()
                .find(|a| a.role == role)
                .with_context(|| format!("data_invalid: F3 role {role} missing"))?,
        )
    };
    let ep = get("fine_egnv")?;
    let mut data = eigenvalues(&fs::read(&ep)?)?;
    let log = fs::read_to_string(get("fine_log")?)?;
    crate::interpolation::native::checked_log(&log, 0)?;
    let offset = crate::interpolation::native_matrices::band_offset(&log, data.bands)?;
    ensure!(
        offset + data.bands <= cfg.wannier.num_wann as usize,
        "data_invalid: native band range exceeds the Wannier subspace"
    );
    for s in &mut data.states {
        s.band += offset;
    }
    ensure!(
        data.grid == cfg.interpolation.k_grid
            && (data.fsthick_ev - cfg.interpolation.fsthick_ev).abs() < 1e-8
            && (data.degaussw_ev - cfg.interpolation.degaussw_ev).abs() < 1e-8,
        "input_conflict: egnv differs from fixed grid/Fermi-surface settings"
    );
    let fp = get("fine_freq")?;
    frequencies(&fs::read(&fp)?, cfg.interpolation.q_grid)?;
    let a2f = get("fine_a2f")?;
    let spectrum = crate::interpolation::native::NativeSpectrum::read(
        &a2f,
        cfg.interpolation.spectral_points as usize,
    )?;
    let selected = if crate::foundation::layout::enabled(work) {0} else {cfg.superconductivity.spectrum_index as usize};
    ensure!(
        selected < spectra.len() && spectrum.a2f.len() == 1,
        "data_invalid: selected F3 spectrum out of range or native spectrum column count invalid"
    );
    let declared = &spectra[selected];
    ensure!(
        declared.energy_mev == spectrum.energy_mev && declared.a2f == spectrum.a2f[0],
        "input_conflict: F3 spectrum differs from native source"
    );
    let matrix_dir = ep.parent().context("data_invalid: egnv parent directory missing")?;
    ensure!(
        matrix_dir
            .file_name()
            .is_some_and(|n| n == format!("{}.ephmat", cfg.qe.prefix).as_str()),
        "data_invalid: native matrix directory name mismatch"
    );
    let mut files = vec![ep.clone(), fp, get("fine_ikmap")?, get("fine_npool")?];
    for a in &fine.artifacts {
        if a.role.starts_with("fine_ephmat_") {
            files.push(artifact_path(work, a)?);
        }
    }
    ensure!(
        files.len() >= 5 && files.iter().all(|p| p.parent() == Some(matrix_dir)),
        "data_invalid: native matrix shards missing or directories mixed"
    );
    let ids = files
        .iter()
        .map(|p| if crate::foundation::layout::enabled(work) {Ok(Identity::reference(p))}else{Identity::capture(p)})
        .collect::<Result<Vec<_>>>()?;
    evidence.verify()?;
    Ok(NativeInputs {
        configured_stages: Some(configured_stages(cfg)?),
        interpolation_digest: if crate::foundation::layout::enabled(work) {"native".into()}else{evidence.sha256.clone()},
        prefix: cfg.qe.prefix.clone(),
        k_grid: cfg.interpolation.k_grid,
        q_grid: cfg.interpolation.q_grid,
        spectral_points: cfg.interpolation.spectral_points as usize,
        eps_acoustic_cm1: cfg.interpolation.eps_acoustic_cm1,
        a2f: if crate::foundation::layout::enabled(work) {Identity::reference(&a2f)}else{Identity::capture(&a2f)?},
        crystal: if crate::foundation::layout::enabled(work) {Identity::reference(&get("fine_crystal")?)}else{Identity::capture(&get("fine_crystal")?)?},
        selecq: Some(if crate::foundation::layout::enabled(work) {Identity::reference(&get("fine_selecq")?)}else{Identity::capture(&get("fine_selecq")?)?}),
        matrix_outdir: matrix_dir
            .parent()
            .context("data_invalid: outdir missing")?
            .into(),
        matrix_files: ids,
        states: data.states,
    })
}

/// 仅各向同性谱路线不读取细网格矩阵、原生日志或更早阶段。
pub fn selected(cfg:&Config)->Result<NativeInputs> {
    let configured = configured_stages(cfg)?;
    let matrix_iso=configured[crate::foundation::native_config::SUPER]["iso"]["epw"]["INPUTEPW"]["fila2f"].as_str().is_some_and(|s|s.trim().is_empty());
    if cfg.superconductivity.mode!=crate::foundation::config::Mode::Iso || matrix_iso {
        return native_selected(cfg);
    }
    let dir=crate::interpolation::fine::directory(cfg)?;
    let explicit=cfg.native_value::<String>(&[crate::foundation::native_config::SUPER,"iso","epw","INPUTEPW","fila2f"]).ok().filter(|s|!s.trim().is_empty());
    let path=explicit.map(|s| {let p=std::path::PathBuf::from(s);if p.is_absolute(){p}else{cfg.work_dir.join(p)}}).unwrap_or_else(||dir.join(format!("{}.a2f",cfg.qe.prefix)));
    let samples=cfg.native_value::<usize>(&[crate::foundation::native_config::SUPER,"iso","epw","INPUTEPW","nqstep"]).unwrap_or(cfg.interpolation.spectral_points as usize);
    crate::interpolation::native::NativeSpectrum::read(&path,samples)?;
    Ok(NativeInputs {
        configured_stages:Some(configured),
        interpolation_digest:"native-spectrum".into(),prefix:cfg.qe.prefix.clone(),k_grid:cfg.interpolation.k_grid,q_grid:cfg.interpolation.q_grid,
        spectral_points:samples,eps_acoustic_cm1:cfg.interpolation.eps_acoustic_cm1,
        a2f:Identity::reference(&path),crystal:Identity::reference(&dir.join("crystal.fmt")),selecq:None,
        matrix_outdir:dir.join("out"),matrix_files:Vec::new(),states:Vec::new(),
    })
}

/// 原生矩阵按实际名称读取并交接，不构建 F2/F3 产物登记。
fn native_selected(cfg:&Config)->Result<NativeInputs> {
    let dir=crate::interpolation::fine::directory(cfg)?;
    let qe=crate::qe_data::qe_config::QeSettings::from_config(cfg)?;
    let epw=crate::interpolation::epw_config::EpwConfig::from_config(cfg)?.with_species(&qe.cell.elements)?;
    let result=crate::interpolation::fine::collect_native_fine_at(&cfg.work_dir,&dir,&epw)?;
    let out=dir.join("out");let matrix=out.join(format!("{}.ephmat",cfg.qe.prefix));
    let mut data=eigenvalues(&fs::read(matrix.join("egnv"))?)?;
    ensure!((data.fsthick_ev-cfg.interpolation.fsthick_ev).abs()<1e-8 && (data.degaussw_ev-cfg.interpolation.degaussw_ev).abs()<1e-8,"input_conflict: selected fine data differs from Fermi-window or electron-smearing settings");
    let log=fs::read_to_string(crate::foundation::scientific_logs::output(&dir,"epw_fine"))?;
    let offset=crate::interpolation::native_matrices::band_offset(&log,data.bands)?;
    ensure!(offset+data.bands<=cfg.wannier.num_wann as usize,"data_invalid: fine band range exceeds Wannier subspace");
    for state in &mut data.states {state.band+=offset;}
    let mut paths=["egnv","freq","ikmap","npool"].iter().map(|n|matrix.join(n)).collect::<Vec<_>>();
    let mut shards=fs::read_dir(&matrix)?.filter_map(|e|e.ok()).map(|e|e.path()).filter(|p|p.file_name().is_some_and(|n|n.to_string_lossy().starts_with("ephmat"))).collect::<Vec<_>>();shards.sort();paths.extend(shards);
    ensure!(paths.len()==result.ephmat_shards+4,"data_invalid: inconsistent matrix shard count");
    let a2f=dir.join(format!("{}.a2f",cfg.qe.prefix));

    Ok(NativeInputs{configured_stages:Some(configured_stages(cfg)?),
        interpolation_digest:"native".into(),prefix:cfg.qe.prefix.clone(),k_grid:cfg.interpolation.k_grid,q_grid:cfg.interpolation.q_grid,
        spectral_points:cfg.interpolation.spectral_points as usize,eps_acoustic_cm1:cfg.interpolation.eps_acoustic_cm1,
        a2f:Identity::reference(&a2f),crystal:Identity::reference(&dir.join("crystal.fmt")),selecq:Some(Identity::reference(&dir.join("selecq.fmt"))),
        matrix_outdir:out,matrix_files:paths.iter().map(|p|Identity::reference(p)).collect(),states:data.states})
}

#[cfg(test)]
mod smearing_tests {
    use super::*;
    use crate::foundation::{config::Mode, native_config::SUPER};
    #[test]
    fn independent_method_widths_choose_matrix_route_and_fallback_is_local() {
        let mut cfg: Config=toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        cfg.superconductivity.mode=Mode::Both;
        cfg.set_native(&[SUPER,"aniso","epw","INPUTEPW","degaussq"],serde_json::json!(2.)).unwrap();
        let values=configured_stages(&cfg).unwrap();
        assert_eq!(values[SUPER]["aniso"]["epw"]["INPUTEPW"]["degaussq"],2.);
        assert_eq!(values[SUPER]["aniso"]["epw"]["INPUTEPW"]["fila2f"],"");
        assert_eq!(values[SUPER]["iso"]["epw"]["INPUTEPW"]["degaussq"],0.5);
        assert!(values[SUPER]["iso"]["epw"]["INPUTEPW"].get("fila2f").is_none());
        cfg.set_native(&[SUPER,"aniso","epw","INPUTEPW","fila2f"],serde_json::json!("old.a2f")).unwrap();
        assert!(configured_stages(&cfg).is_err());
    }
}
