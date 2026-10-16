use anyhow::{bail, ensure, Context, Result};
use std::path::Path;
use super::super::foundation::{config, stage::Backend, store};
use super::super::interpolation::manifest::InterpolationManifest;
use super::{
    aniso, attempt, compare, iso, manifest::{AttemptRecord, SuperconductivityManifest, check},
    parse, solver_config::SolverConfig,
};
use super::super::qe_data::manifest::Validation;

/// 由插值清单构建超导谱（FR-003/011）：细网格频率（cm⁻¹→meV）+ 指定谱 a²F。
fn build_spectrum(f3: &InterpolationManifest, cfg: &config::Config) -> Result<parse::Spectrum> {
    let fine = f3
        .fine
        .as_ref()
        .context("data_invalid: 插值清单缺少细网格数据")?;
    let idx = cfg.superconductivity.spectrum_index as usize;
    ensure!(
        idx < f3.spectra.len(),
        "data_invalid: spectrum_index 超出插值谱数量"
    );
    let spectrum = &f3.spectra[idx];
    let cm1_to_mev = 0.123984_f64;
    let frequencies_mev: Vec<f64> = fine.frequencies.iter().map(|w| *w * cm1_to_mev).collect();
    let cutoff_mev = cfg.superconductivity.frequency_cutoff_ev * 1000.0;
    parse::compute_lambda(&frequencies_mev, &spectrum.a2f, cutoff_mev)
}

/// 执行完整超导求解（F4），交付 superconductivity/manifest.json（FR-001~013）。
pub fn run(
    cfg: &config::Config,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<SuperconductivityManifest> {
    let f3_path = work.join("interpolation").join("manifest.json");
    ensure!(
        f3_path.exists(),
        "data_invalid: 缺失插值清单 {}，F4 不得放行",
        f3_path.display()
    );
    let f3 = InterpolationManifest::load(&f3_path)
        .map_err(|e| anyhow::anyhow!("data_invalid: 无法加载插值清单: {e}"))?;
    ensure!(
        f3.validation.all_passed(),
        "data_invalid: 插值本地校验未通过，F4 不得放行"
    );

    let sc = SolverConfig::from_config(cfg)?;
    let spectrum = build_spectrum(&f3, cfg)?;
    let mode = sc.mode();

    let (tc, located, converged, gaps): (Option<f64>, bool, bool, Vec<f64>) = match mode {
        config::Mode::Iso => {
            let r = iso::solve(&spectrum, &sc)?;
            (
                r.tc,
                r.located,
                r.tc.is_some(),
                r.gap_at_t.map(|g| vec![g]).unwrap_or_default(),
            )
        }
        config::Mode::Aniso => {
            let r = aniso::solve(&spectrum, &sc)?;
            (r.tc, r.located, r.converged, vec![r.gap_min, r.gap_max])
        }
        config::Mode::Both => {
            // 同时求解两种模式，以 iso 的 Tc 为准，aniso 提供双初值判据（FR-005）。
            let ri = iso::solve(&spectrum, &sc)?;
            let ra = aniso::solve(&spectrum, &sc)?;
            let gaps = match (ri.gap_at_t, ra.converged) {
                (Some(g), true) => vec![g, ra.gap_min, ra.gap_max],
                _ => Vec::new(),
            };
            (ri.tc, ri.located, ra.converged, gaps)
        }
    };

    // 单次尝试记录（FR-006）：各 gap 温度下的收敛判据。
    let mut attempts: Vec<AttemptRecord> = Vec::new();
    for t in &sc.gap_temperatures_k() {
        let ar = attempt::run_attempt(mode, &spectrum, &sc, *t)?;
        attempts.push(AttemptRecord {
            temperature_k: ar.temperature_k,
            mode: ar.mode,
            converged: ar.converged,
            gap_mev: ar.gap_mev,
        });
    }

    let mode_str = format!("{:?}", mode).to_lowercase();

    let checks = vec![
        check("interpolation_valid", true, "F3 校验通过"),
        check("spectrum_built", true, &format!("λ_eff={:.4} ω_log={:.1}K", spectrum.lambda_eff, spectrum.omega_log_k)),
        check("tc_search", located, &format!("Tc={:?} 定位={}", tc, located)),
    ];
    let all_passed = located && converged && checks.iter().all(|c| c.passed);
    let validation = Validation {
        status: if all_passed { "valid" } else { "partial" }.to_string(),
        checks,
        units: {
            let mut m = std::collections::BTreeMap::new();
            m.insert("tc".to_string(), "K".to_string());
            m.insert("gap".to_string(), "meV".to_string());
            m
        },
        thresholds: {
            let mut m = std::collections::BTreeMap::new();
            m.insert("tc".to_string(), compare::TOL_TC_K);
            m.insert("gap".to_string(), compare::TOL_GAP_MEV);
            m
        },
        evidence: vec![format!("mode={} located={} converged={}", mode_str, located, converged)],
    };

    let manifest = SuperconductivityManifest {
        schema_version: 1,
        run_id: "f4".to_string(),
        interpolation_digest: store::hash_file(&f3_path)?,
        mode: mode_str,
        tc,
        located,
        converged,
        gaps_mev: gaps,
        attempts,
        validation,
    };

    let out = work.join("superconductivity").join("manifest.json");
    let _ = backend;
    let _ = launcher;
    store::write(&out, &manifest)?;
    if !manifest.validation.all_passed() {
        bail!(
            "data_invalid: 超导本地校验未通过，拒绝发布清单: {}",
            manifest.validation.status
        );
    }
    Ok(manifest)
}
