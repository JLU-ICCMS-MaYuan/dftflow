//! 实际公开入口的英文帮助、报错和配置检查；不调用科学程序或调度器。
use std::{fs, process::Command};
use epwflow::foundation::store;

const BINS: [&str; 4] = [
    env!("CARGO_BIN_EXE_epw_phonon"), env!("CARGO_BIN_EXE_epw_wannier"),
    env!("CARGO_BIN_EXE_epw_superconductivity"), env!("CARGO_BIN_EXE_epw_clear"),
];
fn assert_english(bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    assert!(!text.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)), "{text}");
}
#[test]
fn public_help_and_argument_errors_are_english() {
    for bin in BINS {
        let help = Command::new(bin).arg("--help").output().unwrap();
        assert!(help.status.success());
        assert_english(&help.stdout); assert_english(&help.stderr);
        assert!(String::from_utf8_lossy(&help.stdout).contains("Usage:"));
        let invalid = Command::new(bin).arg("--unknown-language-check-option").output().unwrap();
        assert!(!invalid.status.success());
        assert_english(&invalid.stdout); assert_english(&invalid.stderr);
    }
}
#[test]
fn public_missing_input_and_invalid_configuration_errors_keep_english_details() {
    let root = std::env::temp_dir().join(format!("english-output-{}", store::token()));
    fs::create_dir_all(&root).unwrap();
    for bin in &BINS[..3] {
        let missing = Command::new(bin).args(["--check", "-c"]).arg(root.join("missing.toml")).output().unwrap();
        assert!(!missing.status.success());
        assert_english(&missing.stdout); assert_english(&missing.stderr);
        assert!(String::from_utf8_lossy(&missing.stderr).contains("missing.toml"));
        let p = root.join("invalid.toml");
        fs::write(&p, "schema_version = 999\n").unwrap();
        let invalid = Command::new(bin).args(["--check", "-c"]).arg(&p).output().unwrap();
        assert!(!invalid.status.success());
        assert_english(&invalid.stdout); assert_english(&invalid.stderr);
        assert!(String::from_utf8_lossy(&invalid.stderr).contains("configuration"));
    }
    fs::remove_dir_all(root).unwrap();
}
