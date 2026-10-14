use std::{fs, path::PathBuf, process::Command};
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_qf-gen-dyn0"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn mgb2_matches_pre_extraction_outputs() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let out = std::env::temp_dir().join(format!("dyn0-tool-{}", std::process::id()));
    let result = run(&[
        "--xml",
        fixture.join("grid.xml").to_str().unwrap(),
        "--nq1",
        "2",
        "--nq2",
        "2",
        "--nq3",
        "1",
        "--prefix",
        "Mg1B2",
        "--out-dir",
        out.to_str().unwrap(),
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for name in ["Mg1B2.dyn0", "q_points.json"] {
        assert_eq!(
            fs::read(out.join(name)).unwrap(),
            fs::read(fixture.join(name)).unwrap(),
            "{name}"
        );
    }
    fs::remove_dir_all(out).unwrap();
}
#[test]
fn invalid_mesh_and_missing_xml_fail() {
    for extra in [
        vec!["--nq1", "0"],
        vec!["--nq2", "-1"],
        vec!["--nq1", "2147483647", "--nq2", "2"],
        vec![],
    ] {
        let mut args = vec!["--xml", "/nonexistent-dyn0-fixture.xml", "--prefix", "MgB2"];
        args.extend(extra);
        assert!(!run(&args).status.success());
    }
    assert!(!run(&["--prefix", "MgB2"]).status.success());
}
