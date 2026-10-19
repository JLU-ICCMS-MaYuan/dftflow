use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;
use super::super::foundation::{budget, config, stage::Backend, store};
use super::{
    adapter, coarse, epw_config, fine, manifest::{InterpolationManifest, QualityReport, Validation},
    native, native_quality, reuse, wannier,
};

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
    let f2_manifest_path = f2_work.join("qe-data").join("manifest.json");
    let binding = adapter::bind_f2(&f2_manifest_path, cfg)?;
    let quality_path = cfg.interpolation.quality.reference_manifest.as_ref().context(
        "data_invalid: 缺少独立插值质量来源清单，不能生成细网格或发布插值交付")?;
    let checked_quality = native_quality::check_current_coarse(quality_path, &work.join("coarse"))?;
    ensure!(checked_quality.passed,
        "data_invalid: 独立插值质量未通过，禁止细网格与超导下游");
    let structure = crate::qe_data::qe_config::QeSettings::from_config(cfg)?;
    let epw_cfg = epw_config::EpwConfig::from_config(cfg)?.with_fermi(binding.fermi_ev)?.with_bands(&binding.eigenvalues)?.with_species(&structure.cell.elements)?;
    epw_cfg.window_selection()?;
    qf_wannier::require_projectability_keywords(&cfg.programs.epw.path)?;
    if let Some(program) = &cfg.programs.wannier90 {
        qf_wannier::require_projectability_keywords(&program.path)?;
    }
    let _token = charge_budget(cfg, "f3")?;

    // Wannier 化（FR-004）。
    let wan = wannier::run_wannier(&epw_cfg, work, launcher, backend)?;
    // 粗网格 EPW（FR-005）。
    let coarse_res = coarse::run_coarse(&epw_cfg, work, launcher, backend)?;
    // 在申请/执行细网格之前再次核对来源；不能先计算昂贵矩阵元再判超差。
    let checked_quality = native_quality::inspect(quality_path)?;
    ensure!(checked_quality.passed, "data_invalid: 插值质量来源改变或检查未通过");
    // 细网格 EPW：每个展宽一列（FR-007/008）。
    let mut spectra = Vec::new();
    let mut fine_data = None;
    for smearing_mev in &cfg.interpolation.phonon_smearings_mev {
        let fr = fine::run_fine(&epw_cfg, work, *smearing_mev, launcher, backend)?;
        let raw = native::NativeSpectrum::read(&work.join("fine").join(format!("{}.a2f", cfg.qe.prefix)),
            cfg.interpolation.spectral_points as usize)?;
        ensure!(raw.a2f.len() == 1, "data_invalid: 本阶段要求单个显式展宽的原生谱");
        let spec = super::manifest::Spectrum {
            smearing_id: format!("mev-{smearing_mev:.16e}"),
            energy_mev: raw.energy_mev, a2f: raw.a2f[0].clone(),
            lambda_integral: raw.lambda_integral[0], lambda_sum: raw.lambda_sum,
            cutoff_ev: cfg.superconductivity.frequency_cutoff_ev,
        };
        spectra.push(spec);
        fine_data = Some(fine::into_fine_data(&fr));
    }
    ensure!(!spectra.is_empty(), "data_invalid: 未产生任何谱函数列");

    let q = &checked_quality.phonon;
    let quality_report = Some(QualityReport {
        points: q.points * q.modes_per_point,
        energy_reference: format!("{:?}", cfg.wannier.energy_reference),
        max_error: q.max_error_cm1, rms_error: q.rms_error_cm1, coverage: 1.0,
        thresholds: super::quality::thresholds(), status: "valid".into(),
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
    validation.units.insert("frequency_error".to_string(), "cm-1".to_string());
    validation.check("f2_valid", true, "F2 来源已绑定且有效");
    validation.check("wannier_converged", wan.converged, &format!("iter={} spread={}", wan.num_iter, wan.max_spread_angstrom2));
    validation.check("coarse_complete", !coarse_data.artifacts.is_empty(), "粗网格角色完整");
    validation.check("fine_complete", fine_data.is_some(), "细网格数据完整");
    validation.check("spectra_present", !spectra.is_empty(), &format!("spectra={}", spectra.len()));
    validation.check("quality_valid", quality_report.as_ref().is_some_and(|q| q.status == "valid"),
        "必须具有通过检查的独立质量证据，缺失不能跳过");

    let mut mode_readiness = BTreeMap::new();
    mode_readiness.insert("wannier".to_string(), wan.converged);
    mode_readiness.insert("coarse".to_string(), !coarse_data.artifacts.is_empty());
    mode_readiness.insert("fine".to_string(), fine_data.is_some());
    mode_readiness.insert("quality".to_string(), quality_report.is_some());

    let manifest = InterpolationManifest {
        schema_version: 1,
        run_id: "f3".to_string(),
        f2_digest: binding.f2_digest.clone(),
        wannier_digest: wan.wannier_digest.clone(),
        coarse: Some(coarse_data),
        fine: fine_data,
        spectra,
        quality: quality_report,
        mode_readiness,
        validation: validation.clone(),
    };

    if manifest.validation.all_passed() {
        let out = work.join("interpolation").join("manifest.json");
        store::write(&out, &manifest)?;
    } else {
        bail!("data_invalid: interpolation 本地校验未通过，拒绝发布清单: {}", validation.status);
    }
    budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |ledger| budget::reconcile(ledger))?;
    Ok(manifest)
}

/// 阶段边界：F3 之后需 F4 接入（FR-001）。
pub fn unsupported_beyond_interpolation() -> Result<()> {
    bail!("not_implemented: 全流程后续阶段（F4–F5）尚未接入，越过 interpolation 不可用");
}

/// 导入可复用阶段登记（FR-010）。
pub fn register_import(source_work: &Path, work: &Path) -> Result<reuse::ImportReport> {
    reuse::import_from_run(source_work, work)
}
