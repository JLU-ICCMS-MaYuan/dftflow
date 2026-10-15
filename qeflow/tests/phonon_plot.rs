//! 真正 CLI 仅重绘：跳段、虚频、数据校验及 CSV 与图片生产路径。
use std::{fs, process::Command};
#[test]
fn redraw_preserves_all_samples_and_rejects_mismatched_labels() {
    let dir = std::env::temp_dir().join(format!("phonon-redraw-{}", std::process::id()));
    fs::create_dir_all(dir.join("phband")).unwrap();
    fs::create_dir_all(dir.join("phdos")).unwrap();
    fs::write(
        dir.join("selected.vasp"),
        "Si\n1\n2 0 0\n1 3 0\n0 0 5\nSi\n1\nDirect\n0 0 0\n",
    )
    .unwrap();
    fs::write(
        dir.join("input.toml"),
        "[qe_phonon]\nwork_dir='.'\nprefix='Si'\n",
    )
    .unwrap();
    let labels = "GAMMA 1 0 0 0 0\nA 3 2 0.5 0 0\nL 4 2 0 0.5 0\nX 6 5 0 0.5 0.5\n";
    fs::write(dir.join("phband/Si_band.labelinfo.dat"), labels).unwrap();
    let freq="&plot nbnd=3, nks=6 /\n0 0 0\n-5 10 20\n0.25 0 0\n-2 11 21\n0.5 0 0\n0 12 22\n0 0.5 0\n3 8 19\n0 0.5 0.25\n4 9 20\n0 0.5 0.5\n5 10 21\n";
    fs::write(dir.join("phband/Si.phband.freq"), freq).unwrap();
    fs::write(
        dir.join("phdos/Si.phdos.dat"),
        "-10 0 0\n0 0.2 0.2\n10 0.4 0.4\n30 0 0\n",
    )
    .unwrap();
    let invoke = || {
        Command::new(env!("CARGO_BIN_EXE_qeflow"))
            .args([
                "qe-process-phonon",
                "-c",
                "input.toml",
                "-i",
                "selected.vasp",
                "--plot",
            ])
            .current_dir(&dir)
            .output()
            .unwrap()
    };
    let out = invoke();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let csv = fs::read_to_string(dir.join("phband/phonon_band.csv")).unwrap();
    let rows: Vec<_> = csv
        .lines()
        .skip(1)
        .map(|r| r.split(',').collect::<Vec<_>>())
        .collect();
    assert_eq!(rows.len(), 6);
    assert_eq!(
        rows.iter().map(|r| r[1]).collect::<Vec<_>>(),
        vec!["0.000000", "1.000000", "2.000000", "2.000000", "3.500000", "5.000000"]
    );
    assert_eq!(rows[0][5], "-5.000000");
    for name in [
        "phband/phonon_band.png",
        "phdos/phonon_dos.png",
        "phonon_combined.jpg",
    ] {
        assert!(fs::metadata(dir.join(name)).unwrap().len() > 10000);
    }
    assert_eq!(
        fs::read_to_string(dir.join("phband/Si.phband.freq")).unwrap(),
        freq
    );
    let original_csv=fs::read(dir.join("phdos/phonon_dos_elements.csv")).unwrap();
    let smoothed=fs::read(dir.join("phdos/phonon_dos.png")).unwrap();
    fs::write(dir.join("input.toml"), "[qe_phonon]\nwork_dir='.'\nprefix='Si'\n[qe_phonon.phdos]\nplot_sigma_cm1=0\n").unwrap();
    assert!(invoke().status.success());
    assert_ne!(smoothed,fs::read(dir.join("phdos/phonon_dos.png")).unwrap());
    assert_eq!(original_csv,fs::read(dir.join("phdos/phonon_dos_elements.csv")).unwrap());
    fs::write(dir.join("input.toml"), "[qe_phonon]\nwork_dir='.'\nprefix='Si'\n[qe_phonon.phdos]\nplot_sigma_cm1=-1\n").unwrap();
    let invalid=invoke();assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("plot_sigma_cm1"));
    fs::write(dir.join("input.toml"), "[qe_phonon]\nwork_dir='.'\nprefix='Si'\n").unwrap();
    fs::write(
        dir.join("phband/Si_band.labelinfo.dat"),
        labels.replace("X 6", "X 7"),
    )
    .unwrap();
    let bad = invoke();
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("索引与频率点数不一致"));
    fs::remove_dir_all(dir).unwrap();
}
