use super::*;
use epwflow::foundation::{
    native_config::NSCF,
    stages::{self, Step},
};

fn mesh_case(mode: &str) -> Case {
    let c = Case::numbered();
    c.numbered_change(|cfg| {
        cfg.qe.scf.k_grid = [2, 3, 6];
        cfg.qe.nscf.k_grid = [2, 3, 6];
        cfg.phonon.q_grid = [1; 3];
        cfg.set_native(&[NSCF, "pw", "CONTROL", "calculation"], json!(mode))
            .unwrap();
    });
    c.put("scf-k-count", "16");
    c.accounting(Some(201));
    c
}

#[test]
fn public_nscf_grid_modes_and_valid_symmetry_reduction() {
    for mode in ["nscf", "bands"] {
        let c = mesh_case(mode);
        success(c.numbered_cli(0, &["--stage", "nscf", "-r"], Some(201), "grid-success"));
        let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
        assert!(stages::reusable(&cfg, Step::Nscf).unwrap());
        let states = epwflow::qe_data::nscf::verified_states(&cfg, &cfg.work_dir).unwrap();
        assert_eq!(states.kpoints.len(), 36);
        let xml = fs::read_to_string(
            c.root
                .join("run/1.qe-scf/out/scf.save/data-file-schema.xml"),
        )
        .unwrap();
        assert!(xml.contains("<nks>16</nks>"));
        let before = c.calls();
        c.accounting(Some(202));
        success(c.numbered_cli(0, &["--stage", "nscf", "-r"], Some(202), "grid-reuse"));
        assert_eq!(before, c.calls());
    }
}

#[test]
fn public_nscf_grid_failure_has_no_completion_or_downstream_calls() {
    for failure in [
        "extra_k",
        "missing_k",
        "reorder_k",
        "fft_dense",
        "fft_smooth",
    ] {
        let c = mesh_case("nscf");
        c.put("fixture-failure", failure);
        let result = c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(201), "grid-failed");
        assert!(!result.status.success(), "{failure}");
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(
            error.contains(if failure.starts_with("fft") {
                "FFT"
            } else {
                "calculation='bands'"
            }),
            "{failure}: {error}"
        );
        assert!(!c.root.join("run/2.qe-nscf/completion.json").exists());
        assert!(!c.root.join("run/5.epw-wannier/completion.json").exists());
        assert_eq!(c.calls().0, "pw\npw\n");
    }
}

#[test]
fn public_nscf_grid_reuse_rejects_previously_unchecked_output() {
    let c = mesh_case("nscf");
    success(c.numbered_cli(
        0,
        &["--stage", "nscf", "-r"],
        Some(201),
        "grid-before-forged-receipt",
    ));
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let xml = c
        .root
        .join("run/2.qe-nscf/out/scf.save/data-file-schema.xml");
    let text = fs::read_to_string(&xml)
        .unwrap()
        .replace("<nks>36</nks>", "<nks>40</nks>");
    fs::write(&xml, text).unwrap();
    let path = stages::receipt(&cfg.work_dir, Step::Nscf);
    let mut completion: stages::Completion = store::read(&path).unwrap();
    for identity in &mut completion.outputs {
        if identity.path == xml {
            *identity = store::Identity::capture(&xml).unwrap();
        }
    }
    store::write(&path, &completion).unwrap();
    let error = format!("{:#}", stages::reusable(&cfg, Step::Nscf).unwrap_err());
    assert!(
        error.contains("nks=40") && error.contains("calculation='bands'"),
        "{error}"
    );
}
