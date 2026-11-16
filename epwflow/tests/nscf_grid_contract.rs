//! 0060：经过真实生产输入／执行／XML 边界，外部科学程序使用隔离替身。
#![cfg(unix)]
#[path = "support/native_fixture.rs"]
mod native_fixture;
use epwflow::{
    foundation::{
        config::Config,
        layout,
        native_config::{NSCF, SCF},
        stage::Backend,
        store,
    },
    qe_data::{nscf, qe_config::QeSettings, scf, xml},
};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

struct Case {
    root: PathBuf,
    cfg: Config,
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Case {
    fn new(mode: &str) -> Self {
        let root = std::env::temp_dir().join(format!("epw-grid-contract-{}", store::token()));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("POSCAR"),
            "Si\n1\n3 0 0\n-1.5 2.598 0\n0 0 3.5\nSi\n1\nDirect\n0 0 0\n",
        )
        .unwrap();
        fs::write(root.join("Si.upf"), "test pseudo").unwrap();
        fs::write(
            root.join("qe-native.py"),
            include_str!("fixtures/qe_native_io.py"),
        )
        .unwrap();
        fs::write(root.join("fixture-failure"), mode).unwrap();
        fs::write(root.join("scf-k-count"), "16").unwrap();
        let script = root.join("pw.x");
        fs::write(
            &script,
            format!(
                "#!/bin/sh\nexec /usr/bin/python3 '{}' pw \"$@\"\n",
                root.join("qe-native.py").display()
            ),
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let mut cfg: Config = toml::from_str(include_str!("fixtures/input.toml")).unwrap();
        cfg.work_dir = root.join("run");
        cfg.outdir = root.join("scratch");
        cfg.structure = root.join("POSCAR");
        cfg.pseudos = [("Si".into(), root.join("Si.upf"))].into();
        cfg.atomic_masses.clear();
        cfg.programs.pw.path = script.clone();
        cfg.programs.ph.path = script.clone();
        cfg.programs.epw.path = script;
        cfg.programs.launcher.clear();
        cfg.programs.environment_script = None;
        cfg.qe.scf.k_grid = [2, 3, 6];
        cfg.qe.scf.k_shift = [0; 3];
        cfg.qe.nscf.k_grid = [2, 3, 6];
        cfg.phonon.q_grid = [1; 3];
        cfg.budget.ledger = root.join("ledger.json");
        native_fixture::matrix(&mut cfg);
        cfg.validate().unwrap();
        Self { root, cfg }
    }
    fn run(&self) -> anyhow::Result<()> {
        let qe = QeSettings::from_config(&self.cfg)?;
        scf::run_scf(&qe, &self.cfg.work_dir, &[], Backend::Local)?;
        nscf::run_nscf(&qe, &self.cfg.work_dir, &[], Backend::Local)?;
        Ok(())
    }
    fn input(&self) -> PathBuf {
        layout::path(&self.cfg.work_dir, "nscf/nscf.in")
    }
    fn output(&self) -> PathBuf {
        layout::path(&self.cfg.work_dir, "nscf/out")
            .join(format!("{}.save/data-file-schema.xml", self.cfg.qe.prefix))
    }
}

#[test]
fn compact_mesh_roundtrips_without_expanded_table() {
    let c = Case::new("");
    let text = toml::to_string_pretty(&c.cfg).unwrap();
    let value: toml::Value = toml::from_str(&text).unwrap();
    assert_eq!(
        value["stages"][NSCF]["K_POINTS"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let input = QeSettings::from_config(&c.cfg)
        .unwrap()
        .nscf_input("./out")
        .unwrap();
    let expected = qf_input::pw::uniform_kpoints_text([2, 3, 6], true).unwrap();
    assert!(input.ends_with(&expected));
}

#[test]
fn invalid_or_conflicting_meshes_fail_before_execution() {
    let c = Case::new("");
    for value in [
        json!([0, 3, 6]),
        json!([-1, 3, 6]),
        json!([2, 3]),
        json!([2.0, 3, 6]),
        json!([65536, 65536, 1]),
        json!("crystal\n1\n0 0 0 1"),
    ] {
        assert!(c
            .cfg
            .clone()
            .set_native(&[NSCF, "pw", "K_POINTS"], value)
            .is_err());
    }
    for card in ["automatic\n2 3 6 1 0 0", "automatic\n4 3 6 0 0 0"] {
        let mut cfg = c.cfg.clone();
        cfg.set_native(&[SCF, "pw", "K_POINTS"], json!(card))
            .unwrap();
        assert!(format!("{:#}", cfg.validate().unwrap_err()).contains("K_POINTS"));
    }
    assert!(!c.root.join("scientific-calls").exists());
}

#[test]
fn scf_sixteen_and_nscf_thirty_six_pass_in_both_pw_modes() {
    for mode in ["nscf", "bands"] {
        let mut c = Case::new("");
        c.cfg
            .set_native(&[NSCF, "pw", "CONTROL", "calculation"], json!(mode))
            .unwrap();
        c.cfg.validate().unwrap();
        c.run().unwrap();
        let states = nscf::verified_states(&c.cfg, &c.cfg.work_dir).unwrap();
        assert_eq!(states.kpoints.len(), 36);
        assert!(fs::read_to_string(c.input())
            .unwrap()
            .contains(&format!("calculation = '{mode}'")));
        assert_eq!(
            fs::read_to_string(c.root.join("scientific-calls")).unwrap(),
            "pw\npw\n"
        );
    }
}

#[test]
fn malformed_outputs_stop_at_nscf_with_actionable_message() {
    for failure in [
        "extra_k",
        "missing_k",
        "inconsistent_nks",
        "duplicate_k",
        "reorder_k",
        "unequal_weight",
        "off_grid",
    ] {
        let c = Case::new(failure);
        let error = format!("{:#}", c.run().unwrap_err());
        assert!(
            error.contains("calculation='bands'")
                && error.contains("pw.x")
                && error.contains("nscf.in"),
            "{failure}: {error}"
        );
        if failure == "extra_k" {
            assert!(error.contains("36") && error.contains("40"), "{error}");
        }
        if failure == "missing_k" {
            assert!(error.contains("36") && error.contains("35"), "{error}");
        }
        assert!(!layout::path(&c.cfg.work_dir, "nscf/completion.json").exists());
        assert_eq!(
            fs::read_to_string(c.root.join("scientific-calls")).unwrap(),
            "pw\npw\n"
        );
    }
}

#[test]
fn actual_input_and_reused_outputs_are_rechecked() {
    let c = Case::new("");
    c.run().unwrap();
    let input = fs::read_to_string(c.input()).unwrap();
    fs::write(c.input(), input.replace("crystal\n36", "crystal\n35")).unwrap();
    assert!(format!(
        "{:#}",
        nscf::verified_states(&c.cfg, &c.cfg.work_dir).unwrap_err()
    )
    .contains("Actual number of rows=36"));
    fs::write(c.input(), &input).unwrap();
    let xml = fs::read_to_string(c.output()).unwrap();
    fs::write(c.output(), xml.replace("<nks>36</nks>", "<nks>40</nks>")).unwrap();
    assert!(nscf::verified_states(&c.cfg, &c.cfg.work_dir).is_err());
}

#[test]
fn dense_and_smooth_fft_are_independently_checked() {
    for failure in ["fft_dense", "fft_smooth", "missing_fft_smooth"] {
        let c = Case::new(failure);
        let error = format!("{:#}", c.run().unwrap_err());
        assert!(
            error.contains("FFT") || error.contains("fft_smooth"),
            "{error}"
        );
        assert!(!layout::path(&c.cfg.work_dir, "nscf/completion.json").exists());
    }
}

#[test]
fn fft_configuration_requires_complete_positive_triplets_and_preserves_native_values() {
    let c = Case::new("");
    for stage in [SCF, NSCF] {
        for key in ["nr1", "nr1s"] {
            let mut cfg = c.cfg.clone();
            cfg.set_native(&[stage, "pw", "SYSTEM", key], json!(216))
                .unwrap();
            assert!(format!("{:#}", cfg.validate().unwrap_err()).contains("FFT"));
        }
    }
    let mut cfg = c.cfg.clone();
    for (key, n) in [
        ("nr1", 216),
        ("nr2", 128),
        ("nr3", 60),
        ("nr1s", 120),
        ("nr2s", 72),
        ("nr3s", 32),
    ] {
        cfg.set_native(&[NSCF, "pw", "SYSTEM", key], json!(n))
            .unwrap();
    }
    cfg.validate().unwrap();
    let input = QeSettings::from_config(&cfg)
        .unwrap()
        .nscf_input("./out")
        .unwrap();
    assert!(input.contains("nr2 = 128") && input.contains("nr2s = 72"));
    for value in [json!(0), json!(-2), json!(2.5)] {
        let mut bad = cfg.clone();
        bad.set_native(&[NSCF, "pw", "SYSTEM", "nr2"], value)
            .unwrap();
        assert!(bad.validate().is_err());
    }
    assert!(c
        .cfg
        .native_value::<serde_json::Value>(&[SCF, "pw", "SYSTEM"])
        .unwrap()
        .get("nr1")
        .is_none());
}

#[test]
fn historical_fft_mismatch_reports_both_xml_sources_and_dimensions() {
    let c = Case::new("");
    let a = c.root.join("scf.xml");
    let b = c.root.join("nscf.xml");
    for (path, n) in [(&a, 128), (&b, 125)] {
        fs::write(path,format!("<espresso><output><basis_set><fft_grid nr1='216' nr2='{n}' nr3='60'/><fft_smooth nr1='120' nr2='72' nr3='32'/></basis_set></output></espresso>")).unwrap();
    }
    let error = format!("{:#}", xml::verify_fft(&a, &b).unwrap_err());
    for needle in [
        "scf.xml",
        "nscf.xml",
        "[216, 128, 60]",
        "[216, 125, 60]",
        "[120, 72, 32]",
    ] {
        assert!(error.contains(needle), "{error}");
    }
    fs::copy(&a, &b).unwrap();
    xml::verify_fft(&a, &b).unwrap();
}

#[test]
#[ignore = "Remarkable specification required EPWFLOW_SIP2_EVIDENCE_ROOT ;History only XML ,Do not perform calculations"]
fn actual_sip2_fft_evidence_is_read_only_and_rejected() {
    let root = PathBuf::from(std::env::var_os("EPWFLOW_SIP2_EVIDENCE_ROOT").expect("Require to specify a catalogue of historical evidence"));
    let scf = root.join("000_scf_nk236/tmp/Si8P16.save/data-file-schema.xml");
    let nscf = root.join("001_nscf_nk236/tmp/Si8P16.save/data-file-schema.xml");
    let before = [store::hash_file(&scf).unwrap(), store::hash_file(&nscf).unwrap()];
    assert_eq!(xml::fft_grids(&scf).unwrap(), xml::FftGrids { dense: [216,128,60], smooth: [120,72,32] });
    assert_eq!(xml::fft_grids(&nscf).unwrap(), xml::FftGrids { dense: [216,125,60], smooth: [120,72,32] });
    let error = format!("{:#}", xml::verify_fft(&scf, &nscf).unwrap_err());
    assert!(error.contains("[216, 128, 60]") && error.contains("[216, 125, 60]"), "{error}");
    let after = [store::hash_file(&scf).unwrap(), store::hash_file(&nscf).unwrap()];
    assert_eq!(before, after);
    println!("{error}\n Two XML  It's SHA256  No change:{before:?}");
}
