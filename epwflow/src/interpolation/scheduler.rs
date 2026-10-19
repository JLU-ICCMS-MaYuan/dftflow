use super::super::foundation::{budget, config, stage::Backend, store};
use super::{
    adapter, coarse, epw_config, fine,
    manifest::{InterpolationManifest, QualityReport, Validation},
    native, reuse, wannier,
};
use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

/// 在共享预算账本上预留资源（FR-011）。
fn charge_budget(cfg: &config::Config, run_id: &str) -> Result<String> {
    let r = &cfg.resources;
    budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |ledger| {
        budget::reserve(
            ledger,
            &format!("interpolation:{}", run_id),
            r.nodes,
            r.mpi,
            r.wall_seconds,
            r.storage_bytes,
        )
    })
    .context("budget_denied: interpolation 预算预留失败")
}

/// 执行完整 Wannier 化与 EPW 插值（F3），交付 interpolation/manifest.json（FR-001~012）。
pub fn run(
    cfg: &config::Config,
    backend: Backend,
    launcher: &[String],
    work: &Path,
    f2_work: &Path,
) -> Result<InterpolationManifest> {
    run_coarse_stages(cfg, backend, launcher, work, f2_work, true)?;
    let result = run_fine_registered(cfg, backend, launcher, work)?;
    budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, budget::reconcile)?;
    Ok(result)
}

/// 统一入口已登记整份分配，禁止再次按阶段计费。
pub fn run_registered(
    cfg: &config::Config,
    backend: Backend,
    launcher: &[String],
    work: &Path,
    f2_work: &Path,
) -> Result<InterpolationManifest> {
    run_coarse_registered(cfg, backend, launcher, work, f2_work)?;
    run_fine_registered(cfg, backend, launcher, work)
}

pub fn run_coarse_registered(
    cfg: &config::Config, backend: Backend, launcher: &[String], work: &Path, f2_work: &Path,
) -> Result<InterpolationManifest> {
    run_coarse_stages(cfg, backend, launcher, work, f2_work, false)
}

fn run_coarse_stages(
    cfg: &config::Config, backend: Backend, launcher: &[String], work: &Path, f2_work: &Path, charge: bool,
) -> Result<InterpolationManifest> {
    let f2_manifest_path = f2_work.join("qe-data").join("manifest.json");
    let binding = adapter::bind_f2(&f2_manifest_path, cfg)?;
    let frozen = super::quality_binding::freeze(cfg, binding.fermi_ev)?;
    let structure = crate::qe_data::qe_config::QeSettings::from_config(cfg)?;
    let epw_cfg = epw_config::EpwConfig::from_config(cfg)?
        .with_fermi(binding.fermi_ev)?
        .with_bands(&binding.eigenvalues)?
        .with_species(&structure.cell.elements)?;
    epw_cfg.window_selection()?;
    qf_wannier::require_projectability_keywords(&cfg.programs.epw.path)?;
    if charge { charge_budget(cfg, "f3")?; }
    // EPW 在一次粗网格执行中完成内置 Wannier 化，不另启动不完整的独立链。
    let f2 = crate::qe_data::manifest::QeManifest::load(&f2_manifest_path)?;
    coarse::stage_f2(&f2, work)?;
    let mut coarse_res = coarse::run_coarse(&epw_cfg, work, launcher, backend)?;
    let wan = wannier::check_native(
        &work.join("coarse").join(format!("{}.wout", cfg.qe.prefix)),
        cfg.wannier.num_wann as usize,
    )?;
    let checked_quality = frozen.pair(work)?;
    ensure!(
        checked_quality.passed,
        "data_invalid: 当前粗网格质量未通过，禁止细网格与超导"
    );
    for (role, name) in [
        ("coarse_quality_sources", "quality-reference.json"),
        ("coarse_quality_binding", "quality-binding.json"),
        ("coarse_quality_report", "quality-report.json"),
    ] {
        let path = work.join(name);
        coarse_res.artifacts.push(super::manifest::Artifact {
            role: role.into(),
            relative_path: name.into(),
            byte_size: std::fs::metadata(&path)?.len(),
            sha256: store::hash_file(&path)?,
            producer_stage: "quality".into(),
            input_digest: store::hash_file(
                cfg.interpolation
                    .quality
                    .reference_manifest
                    .as_ref()
                    .unwrap(),
            )?,
            format_version: "1".into(),
        });
    }
    let q = &checked_quality.phonon;
    let quality_report = Some(QualityReport {
        points: q.points * q.modes_per_point,
        energy_reference: format!("{:?}", cfg.wannier.energy_reference),
        max_error: q.max_error_cm1,
        rms_error: q.rms_error_cm1,
        coverage: 1.0,
        thresholds: super::quality::thresholds(),
        status: "valid".into(),
    });

    let coarse_data = coarse::into_coarse_data(&coarse_res);

    // 本地校验（FR-009/012）。
    let mut validation = Validation {
        status: "valid".to_string(),
        checks: Vec::new(),
        units: BTreeMap::new(),
        thresholds: BTreeMap::new(),
        evidence: Vec::new(),
    };
    validation
        .units
        .insert("frequency_error".to_string(), "cm-1".to_string());
    validation.check("f2_valid", true, "F2 来源已绑定且有效");
    validation.check(
        "wannier_converged",
        wan.converged,
        &format!("iter={} spread={}", wan.num_iter, wan.max_spread_angstrom2),
    );
    validation.check(
        "coarse_complete",
        !coarse_data.artifacts.is_empty(),
        "粗网格角色完整",
    );
    validation.check(
        "quality_valid",
        quality_report.as_ref().is_some_and(|q| q.status == "valid"),
        "必须具有通过检查的独立质量证据，缺失不能跳过",
    );

    let mut mode_readiness = BTreeMap::new();
    mode_readiness.insert("wannier".to_string(), wan.converged);
    mode_readiness.insert("coarse".to_string(), !coarse_data.artifacts.is_empty());
    mode_readiness.insert("fine".to_string(), false);
    mode_readiness.insert("quality".to_string(), quality_report.is_some());

    let manifest = InterpolationManifest {
        schema_version: 1,
        run_id: crate::qe_data::manifest::QeManifest::load(&f2_manifest_path)?.run_id,
        f2_digest: binding.f2_digest.clone(),
        wannier_digest: wan.wannier_digest.clone(),
        coarse: Some(coarse_data),
        fine: None,
        spectra: vec![],
        quality: quality_report,
        mode_readiness,
        validation: validation.clone(),
    };

    if manifest.validation.all_passed() {
        let out = work.join("wannier").join("manifest.json");
        store::write(&out, &manifest)?;
    } else {
        bail!(
            "data_invalid: interpolation 本地校验未通过，拒绝发布清单: {}",
            validation.status
        );
    }
    Ok(manifest)
}

/// 第三阶段只消费已经发布的粗网格，不重新执行 Wannier 化。
pub fn run_fine_registered(
    cfg: &config::Config,
    backend: Backend,
    launcher: &[String],
    work: &Path,
) -> Result<InterpolationManifest> {
    let identity = store::Identity::capture(&work.join("wannier/manifest.json"))?;
    let mut manifest = verify_coarse(cfg, work)?;
    ensure!(!work.join("interpolation/manifest.json").exists(),
        "input_conflict: 细网格清单已存在，保留现场");
    let f2 = adapter::bind_f2(&work.join("qe-data/manifest.json"), cfg)?;
    ensure!(f2.f2_digest == manifest.f2_digest, "input_conflict: 粗网格 QE 来源变化");
    let structure = crate::qe_data::qe_config::QeSettings::from_config(cfg)?;
    let epw_cfg = epw_config::EpwConfig::from_config(cfg)?
        .with_fermi(f2.fermi_ev)?
        .with_bands(&f2.eigenvalues)?
        .with_species(&structure.cell.elements)?;
    let coarse = manifest.coarse.as_ref().context("data_invalid: 缺粗网格")?;
    // 细网格 EPW：每个展宽一列（FR-007/008）。
    let mut spectra = Vec::new();
    let mut fine_data = None;
    let mut other_smearings = vec![];
    for (index, smearing_mev) in cfg.interpolation.phonon_smearings_mev.iter().enumerate() {
        ensure!(
            super::quality_binding::check(cfg, work)?.passed,
            "data_invalid: 粗网格质量来源变化"
        );
        for artifact in &coarse.artifacts {
            let path = work.join(&artifact.relative_path);
            ensure!(
                std::fs::metadata(&path)?.len() == artifact.byte_size
                    && store::hash_file(&path)? == artifact.sha256,
                "input_conflict: 粗网格或质量来源在细网格执行前改变"
            );
        }
        let dir = work.join("fine").join(format!("smearing-{:04}", index + 1));
        let fr = fine::run_fine_at(&epw_cfg, work, &dir, *smearing_mev, launcher, backend)?;
        let raw = native::NativeSpectrum::read(
            &dir.join(format!("{}.a2f", cfg.qe.prefix)),
            cfg.interpolation.spectral_points as usize,
        )?;
        ensure!(
            raw.a2f.len() == 1,
            "data_invalid: 本阶段要求单个显式展宽的原生谱"
        );
        let spec = super::manifest::Spectrum {
            smearing_id: format!("mev-{smearing_mev:.16e}"),
            energy_mev: raw.energy_mev,
            a2f: raw.a2f[0].clone(),
            lambda_integral: raw.lambda_integral[0],
            lambda_sum: raw.lambda_sum,
            cutoff_ev: cfg.superconductivity.frequency_cutoff_ev,
        };
        spectra.push(spec);
        if index == cfg.superconductivity.spectrum_index as usize {
            fine_data = Some(fine::into_fine_data(&fr));
        } else {
            for mut a in fr.artifacts {
                a.role = format!("smearing_{:04}_{}", index + 1, a.role);
                other_smearings.push(a);
            }
        }
    }
    ensure!(!spectra.is_empty(), "data_invalid: 未产生任何谱函数列");
    if let Some(data) = &mut fine_data {
        data.artifacts.extend(other_smearings);
    }
    ensure!(
        super::quality_binding::check(cfg, work)?.passed,
        "data_invalid: 发布前质量来源改变"
    );


    identity.verify()?;
    manifest.coarse.as_mut().unwrap().artifacts.push(super::manifest::Artifact {
        role: "coarse_stage_manifest".into(),
        relative_path: "wannier/manifest.json".into(),
        byte_size: identity.bytes,
        sha256: identity.sha256,
        producer_stage: "wannier".into(),
        input_digest: manifest.f2_digest.clone(),
        format_version: "1".into(),
    });
    manifest.fine = fine_data;
    manifest.spectra = spectra;
    manifest.mode_readiness.insert("fine".into(), true);
    manifest.validation.check("fine_complete", manifest.fine.is_some(), "细网格数据完整");
    manifest.validation.check("spectra_present", !manifest.spectra.is_empty(), "原生谱已核验");
    ensure!(manifest.validation.all_passed(), "data_invalid: 细网格校验失败");
    store::write(&work.join("interpolation/manifest.json"), &manifest)?;
    Ok(manifest)
}

/// 清单、质量来源、每个粗网格文件均须仍然有效，且不得混入细网格结果。
pub fn verify_coarse(cfg: &config::Config, work: &Path) -> Result<InterpolationManifest> {
    let m = InterpolationManifest::load(&work.join("wannier/manifest.json"))?;
    ensure!(m.validation.all_passed()
        && m.quality.as_ref().is_some_and(|q| q.status == "valid")
        && m.fine.is_none() && m.spectra.is_empty(), "data_invalid: 粗网格阶段清单无效");
    ensure!(super::quality_binding::check(cfg, work)?.passed, "data_invalid: 粗网格质量失败");
    let coarse = m.coarse.as_ref().context("data_invalid: 缺粗网格")?;
    ensure!(!coarse.artifacts.is_empty(), "data_invalid: 粗网格证据为空");
    for artifact in &coarse.artifacts {
        crate::superconductivity::source::artifact_path(work, artifact)?;
    }
    Ok(m)
}

/// 导入可复用阶段登记（FR-010）。
pub fn register_import(source_work: &Path, work: &Path) -> Result<reuse::ImportReport> {
    reuse::import_from_run(source_work, work)
}
