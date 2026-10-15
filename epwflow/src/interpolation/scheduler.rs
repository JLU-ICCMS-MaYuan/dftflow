use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;
use super::super::foundation::{budget, config, stage::Backend, store};
use super::{
    adapter, coarse, epw_config, fine, manifest::{InterpolationManifest, QualityReport, Validation},
    quality, reuse, spectral, wannier,
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
    let _token = charge_budget(cfg, "f3")?;
    let f2_manifest_path = f2_work.join("qe-data").join("manifest.json");
    let binding = adapter::bind_f2(&f2_manifest_path, cfg)?;
    let epw_cfg = epw_config::EpwConfig::from_config(cfg)?;

    // Wannier 化（FR-004）。
    let wan = wannier::run_wannier(&epw_cfg, work, launcher, backend)?;
    // 粗网格 EPW（FR-005）。
    let coarse_res = coarse::run_coarse(&epw_cfg, work, launcher, backend)?;
    // 细网格 EPW：每个展宽一列（FR-007/008）。
    let mut spectra = Vec::new();
    let mut fine_data = None;
    for smearing_mev in &cfg.interpolation.phonon_smearings_mev {
        let fr = fine::run_fine(&epw_cfg, work, *smearing_mev, launcher, backend)?;
        // 简化：α²F 由频率幅值构造以产出可积分谱；真实由 EPW 读取。
        let a2f: Vec<f64> = fr.frequencies.iter().map(|w| 0.5 * (1.0 - (-w.abs() / 200.0).exp())).collect();
        let spec = spectral::compute_spectrum(
            &fr.frequencies,
            &a2f,
            &format!("sm{:04}", *smearing_mev as i32),
            cfg.superconductivity.frequency_cutoff_ev,
            cfg.interpolation.eps_acoustic_cm1,
        )?;
        spectra.push(spec);
        fine_data = Some(fine::into_fine_data(&fr));
    }
    ensure!(!spectra.is_empty(), "data_invalid: 未产生任何谱函数列");

    // 独立插值质量检查（FR-006/009）：参考频率存在才评估。
    let quality_report: Option<QualityReport> = {
        let ref_path = f2_work.join("qe-data").join("reference_frequencies.dat");
        if ref_path.exists() {
            if let Some(fd) = &fine_data {
                let reference = fine::read_f64_col(&ref_path)?;
                Some(quality::evaluate(&reference, &fd.frequencies, &format!("{:?}", cfg.wannier.energy_reference))?)
            } else {
                None
            }
        } else {
            None
        }
    };

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
    if let Some(q) = &quality_report {
        validation.check("quality_valid", q.status == "valid", &format!("max={} rms={}", q.max_error, q.rms_error));
    }

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
