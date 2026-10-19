use serde_json::json;
use std::{path::PathBuf, process::Command};

fn dir(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("wannier-policy-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn xml(path: &std::path::Path) {
    let ev = [-0.0002, 0.0002, 4., 6.]
        .map(|v| format!("{:.17e}", v / 27.211386245988))
        .join(" ");
    std::fs::write(path, format!("<espresso><output><band_structure><nbnd>4</nbnd><nks>1</nks><fermi_energy>0</fermi_energy><ks_energies><eigenvalues>{ev}</eigenvalues></ks_energies></band_structure></output></espresso>")).unwrap();
}

#[test]
fn generated_win_uses_default_combination_and_saves_selection() {
    let p = dir("input");
    xml(&p.join("bands.xml"));
    std::fs::write(
        p.join("POSCAR"),
        "Si\n1\n1 0 0\n0 1 0\n0 0 1\nSi\n1\nDirect\n0 0 0\n",
    )
    .unwrap();
    let structure =
        wannier90flow::poscar::parse_poscar(p.join("POSCAR").to_str().unwrap()).unwrap();
    let mut cfg = wannier90flow::config::wannier_init_template();
    cfg["wannier_init"]["num_wann"] = json!(3);
    cfg["wannier_init"]["bands_file"] = json!(p.join("bands.xml"));
    cfg["wannier_init"]["projections"]["list"] = json!(["Si:p"]);
    cfg["wannier_init"]["k_points"]["mp_grid"] = json!([1, 1, 1]);
    let output = p.join("seed.win");
    wannier90flow::init::write_win(output.to_str().unwrap(), &structure, &cfg, &[[0.; 3]], &[])
        .unwrap();
    let win = std::fs::read_to_string(output).unwrap();
    assert!(win.contains("dis_froz_proj = true"));
    assert!(win.contains("dis_proj_min = 1.0000000000000000e-2"));
    assert!(win.contains("dis_proj_max = 9.4999999999999996e-1"));
    assert!(win.contains("dis_froz_max = 2.0000000000000000e0"));
    assert!(!win.contains("dis_win_min") && !win.contains("dis_froz_min"));
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(p.join("seed.window-selection.json")).unwrap())
            .unwrap();
    assert_eq!(report["mode"], "fixed_plus_projectability");
    assert_eq!(report["parameters"]["dis_froz_max"], 2.);
    cfg["wannier_init"]["dis_froz_max"] = json!("invalid");
    assert!(wannier90flow::init::write_win(
        p.join("bad-type.win").to_str().unwrap(), &structure, &cfg, &[[0.; 3]], &[]
    ).is_err());
    cfg["wannier_init"]["dis_froz_max"] = serde_json::Value::Null;
    cfg["wannier_init"]["bands_file"] = serde_json::Value::Null;
    assert!(wannier90flow::init::write_win(
        p.join("bad.win").to_str().unwrap(),
        &structure,
        &cfg,
        &[[0.; 3]],
        &[]
    )
    .is_err());
    std::fs::remove_dir_all(p).unwrap();
}

#[test]
fn cli_default_and_legacy_rejection_are_observable() {
    let p = dir("cli");
    xml(&p.join("bands.xml"));
    let out = Command::new(env!("CARGO_BIN_EXE_wannier90"))
        .args([
            "window",
            p.join("bands.xml").to_str().unwrap(),
            "--num-wann",
            "3",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["parameters"]["dis_proj_max"], 0.95);
    let out = Command::new(env!("CARGO_BIN_EXE_wannier90"))
        .args([
            "window",
            p.join("bands.xml").to_str().unwrap(),
            "--auto",
            "1",
            "3",
            "1",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("已弃用"));
    std::fs::remove_dir_all(p).unwrap();
}

#[cfg(unix)]
#[test]
fn generated_script_stops_before_pw2_after_preprocessing_failure() {
    use std::os::unix::fs::PermissionsExt;
    let p = dir("script");
    let w90 = p.join("w90");
    std::fs::write(&w90, "#!/bin/bash\nexit 17\n").unwrap();
    std::fs::set_permissions(&w90, std::fs::Permissions::from_mode(0o755)).unwrap();
    let pw2 = p.join("pw2");
    std::fs::write(&pw2, "#!/bin/bash\ntouch called-pw2\n").unwrap();
    std::fs::set_permissions(&pw2, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut cfg = wannier90flow::config::wannier_init_template();
    cfg["wannier90"]["executable_path"] = json!(w90);
    cfg["pw2wannier90"]["executable_path"] = json!(pw2);
    wannier90flow::init::create_run_script(p.to_str().unwrap(), "seed", &cfg).unwrap();
    let out = Command::new("bash")
        .arg(p.join("run_wannier90.sh"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(17));
    assert!(!p.join("called-pw2").exists());
    let input = std::fs::read_to_string(p.join("seed.pw2wan")).unwrap();
    assert!(!input.contains("None"));
    assert!(input.contains("prefix = 'seed'") && input.contains("seedname = 'seed'"));
    std::fs::remove_dir_all(p).unwrap();
}
