//! F5 行为测试：FR-001~FR-010。隔离临时目录，复用上游清单结构。

use std::path::PathBuf;

use epwflow::delivery::{
    acceptance, collect, compare, plot, regression, report_store::{ComparisonCell, ResultsBundle},
};
use epwflow::foundation::store;

fn write_json(path: &PathBuf, value: serde_json::Value) {
    std::fs::write(path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
}

#[test]
fn collect_assembles_bundle_from_upstream_manifests() {
    let root = std::env::temp_dir().join(format!("epwflow-f5-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let work = root.join("work");
    std::fs::create_dir_all(&work).unwrap();

    // 构造合法插值清单。
    let f3 = work.join("interpolation");
    std::fs::create_dir_all(&f3).unwrap();
    write_json(
        &f3.join("manifest.json"),
        serde_json::json!({
            "schema_version": 2, "run_id": "f3", "f2_digest": "x", "wannier_digest": "y",
            "coarse": null,
            "fine": {"kpoints": 8, "qpoints": 8, "energies": [1.0], "frequencies": [80.6, 161.2], "weights": [0.5,0.5], "kqmap": 2, "layout": "epw", "ephmat_shards": 1, "artifacts": []},
            "spectra": [{"smearing_id": "sm0001", "energy_mev": [10.0,20.0], "a2f": [0.1,0.1], "lambda_integral": 0.8, "lambda_sum": 0.8, "cutoff_ev": 0.2}],
            "mode_readiness": {"iso": true},
            "validation": {"status": "valid", "checks": [{"name":"x","passed":true,"detail":"ok"}], "units": {}, "thresholds": {}, "evidence": []}
        }),
    );
    // 构造合法超导清单。
    let f4 = work.join("superconductivity");
    std::fs::create_dir_all(&f4).unwrap();
    write_json(
        &f4.join("manifest.json"),
        serde_json::json!({
            "schema_version": 1, "run_id": "f4", "interpolation_digest": "z", "mode": "iso",
            "tc": 6.0, "located": true, "converged": true, "gaps_mev": [1.5],
            "attempts": [],
            "validation": {"status": "valid", "checks": [{"name":"x","passed":true,"detail":"ok"}], "units": {}, "thresholds": {}, "evidence": []}
        }),
    );

    let b = collect::collect(&work).unwrap();
    assert_eq!(b.tc, Some(6.0));
    assert_eq!(b.gaps_mev, vec![1.5]);
    assert!((b.lambda_eff - 0.8).abs() < 1e-9);
    assert!(!b.cells.is_empty());
}

#[test]
fn acceptance_table_has_six_combinations() {
    let cells = vec![ComparisonCell {
        ph_mode: "index".to_string(),
        solver_mode: "iso".to_string(),
        reference: "epwpy".to_string(),
        input_hashes: Default::default(),
        metrics: Default::default(),
        tolerances: Default::default(),
        status: "ok".to_string(),
        evidence: vec![],
    }];
    let bundle = ResultsBundle {
        schema_version: 1,
        run_id: "f4".to_string(),
        cells,
        tc: Some(6.0),
        gaps_mev: vec![1.5],
        lambda_eff: 0.8,
        omega_log_k: 0.0,
        validation: epwflow::qe_data::manifest::Validation {
            status: "valid".to_string(),
            checks: vec![],
            units: Default::default(),
            thresholds: Default::default(),
            evidence: vec![],
        },
    };
    let table = acceptance::acceptance_table(&bundle);
    assert_eq!(table.len(), 6, "六组合（3 PH × 2 求解）");
    let run = table.iter().filter(|c| c.status != "not_run").count();
    assert_eq!(run, 1, "仅实际运行组合非 not_run");
}

#[test]
fn compare_tolerances_and_regression() {
    assert!(compare::tc_close(Some(6.0), Some(6.05)));
    assert!(!compare::tc_close(Some(6.0), Some(6.5)));
    assert!(compare::lambda_close(0.8, 0.801));
    assert!(!compare::lambda_close(0.8, 0.85));
    let cur = ResultsBundle {
        schema_version: 1, run_id: "c".to_string(), cells: vec![], tc: Some(6.0),
        gaps_mev: vec![], lambda_eff: 0.8, omega_log_k: 0.0,
        validation: epwflow::qe_data::manifest::Validation { status: "valid".to_string(), checks: vec![], units: Default::default(), thresholds: Default::default(), evidence: vec![] },
    };
    let base = ResultsBundle {
        schema_version: 1, run_id: "b".to_string(), cells: vec![], tc: Some(6.0),
        gaps_mev: vec![], lambda_eff: 0.8, omega_log_k: 0.0,
        validation: epwflow::qe_data::manifest::Validation { status: "valid".to_string(), checks: vec![], units: Default::default(), thresholds: Default::default(), evidence: vec![] },
    };
    let rep = regression::against_baseline(&cur, Some(&base)).unwrap();
    assert!(rep.all_close);
}

#[test]
fn plot_writes_valid_svg_and_csv() {
    let root = std::env::temp_dir().join(format!("epwflow-f5p-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let svg = root.join("a.svg");
    plot::plot_alpha2f(&[1.0, 2.0, 3.0], &[0.1, 0.2, 0.15], &svg).unwrap();
    let txt = std::fs::read_to_string(&svg).unwrap();
    assert!(txt.contains("<svg") && txt.contains("</svg>"));

    let bundle = ResultsBundle {
        schema_version: 1, run_id: "c".to_string(), cells: vec![], tc: Some(6.0),
        gaps_mev: vec![1.5], lambda_eff: 0.8, omega_log_k: 0.0,
        validation: epwflow::qe_data::manifest::Validation { status: "valid".to_string(), checks: vec![], units: Default::default(), thresholds: Default::default(), evidence: vec![] },
    };
    let csv = root.join("r.csv");
    plot::export_csv(&bundle, &csv).unwrap();
    let c = std::fs::read_to_string(&csv).unwrap();
    assert!(c.contains("tc_k,6"));
}
