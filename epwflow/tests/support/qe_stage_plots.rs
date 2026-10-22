//! 0047：公开执行、图像与补图边界；科学程序为受控替身。
use super::*;

fn png(c: &Case, relative: &str) {
    assert!(
        fs::read(c.root.join(relative))
            .unwrap()
            .starts_with(b"\x89PNG"),
        "{relative}"
    );
}

#[test]
fn electronic_leaf_plot_survives_other_leaf_failure_without_phonons() {
    let c = Case::numbered();
    for name in ["bands", "dos"] {
        let file = format!("{name}.x");
        let source = fs::read_to_string(c.root.join(&file)).unwrap();
        let marker = format!("kind=\"{name}\"");
        assert!(source.contains(&marker));
        c.script(&file, &source.replace(&marker, &format!(
            "{marker}\nwith Path(__file__).with_name('electronic-calls').open('a') as f: f.write(kind+'\\n')"
        )));
    }
    let dos = fs::read_to_string(c.root.join("dos.x")).unwrap();
    c.script("dos.x", "#!/bin/bash\nexit 17\n");
    c.accounting(Some(201));
    assert!(!c
        .numbered_cli(
            0,
            &["--stage", "electronic", "-r"],
            Some(201),
            "qe-plot-dos-failed"
        )
        .status
        .success());
    png(
        &c,
        "run/3.qe-electronic/qe_eband/plots/electronic-bands.png",
    );
    assert!(!c.root.join("run/4.qe-phonon-nosplit/ph.out").exists());
    assert!(!c.root.join("run/5.epw-wannier").exists());
    let calls = c.calls();
    c.script("dos.x", &dos);
    c.accounting(Some(202));
    success(c.numbered_cli(
        0,
        &["--stage", "electronic", "--resume", "-r"],
        Some(202),
        "qe-plot-dos-resume",
    ));
    png(&c, "run/3.qe-electronic/qe_eledos/plots/electronic-dos.png");
    assert_eq!(calls, c.calls(), "成功 PW 不得重跑");
    let bands = c
        .root
        .join("run/3.qe-electronic/qe_eband/plots/electronic-bands.png");
    let original = store::hash_file(&bands).unwrap();
    let electronic_calls = fs::read(c.root.join("electronic-calls")).unwrap();
    fs::remove_file(&bands).unwrap();
    c.accounting(Some(203));
    success(c.numbered_cli(
        0,
        &["--stage", "electronic", "-r"],
        Some(203),
        "qe-plot-only",
    ));
    assert_eq!(original, store::hash_file(&bands).unwrap());
    assert_eq!(calls, c.calls());
    assert_eq!(electronic_calls, fs::read(c.root.join("electronic-calls")).unwrap());
}

#[test]
fn direct_electronic_verification_draws_before_next_scientific_job() {
    let c = Case::numbered();
    c.direct_setup();
    let mut stems = Vec::new();
    for n in 0..12 {
        let o = c.direct_advance(
            0,
            &["--stage", "electronic", "-s"],
            &format!("qe-plot-direct-{n}"),
        );
        let complete = String::from_utf8_lossy(&o.stdout).contains("workflow_complete: true");
        success(o);
        if complete {
            break;
        }
        let task = c.direct_task();
        let stem = task["stem"].as_str().unwrap();
        stems.push(stem.to_string());
        if stem == "dos" {
            png(
                &c,
                "run/3.qe-electronic/qe_eband/plots/electronic-bands.png",
            );
            assert!(!c
                .root
                .join("run/3.qe-electronic/qe_eledos/plots/electronic-dos.png")
                .exists());
        }
        assert_eq!(c.direct_execute(), 0);
    }
    assert_eq!(stems, ["scf", "nscf", "eband", "bands", "dos"]);
    png(&c, "run/3.qe-electronic/qe_eledos/plots/electronic-dos.png");
    let before = fs::read(c.root.join("submissions")).unwrap();
    fs::remove_file(
        c.root
            .join("run/3.qe-electronic/qe_eledos/plots/electronic-dos.png"),
    )
    .unwrap();
    success(c.direct_advance(0, &["--stage", "electronic", "-s"], "qe-plot-direct-repair"));
    png(&c, "run/3.qe-electronic/qe_eledos/plots/electronic-dos.png");
    assert_eq!(before, fs::read(c.root.join("submissions")).unwrap());
}

#[test]
fn phonon_plots_include_elements_and_repair_without_scientific_calls() {
    let c = Case::numbered();
    // 两种元素、同种元素两个原子，校验结构聚合，而非按列冒充元素。
    c.put(
        "POSCAR",
        "Si C\n1\n1 0 0\n0 1 0\n0 0 1\nSi C\n1 2\nDirect\n0 0 0\n0.25 0.25 0.25\n0.5 0.5 0.5\n",
    );
    c.numbered_change(|cfg| {
        cfg.pseudos.insert("C".into(), c.root.join("POSCAR"));
    });
    c.accounting(Some(201));
    success(c.numbered_cli(0, &["-r"], Some(201), "qe-plot-phonon"));
    png(&c, "run/4.qe-phonon-nosplit/plots/phband/phonon_band.png");
    png(&c, "run/4.qe-phonon-nosplit/plots/phdos/phonon_dos.png");
    let csv = c
        .root
        .join("run/4.qe-phonon-nosplit/plots/phdos/phonon_dos_elements.csv");
    let text = fs::read_to_string(csv).unwrap();
    assert!(text.starts_with("frequency_cm-1,TDOS,Si,C\n"));
    for line in text.lines().skip(1) {
        let row: Vec<f64> = line.split(',').map(|v| v.parse().unwrap()).collect();
        assert!((row[1] - row[2] - row[3]).abs() < 1e-6);
        assert!((row[3] - 2.0 * row[2]).abs() < 1e-6);
    }
    assert!(!c.root.join("run/5.epw-wannier").exists());
    let calls = c.calls();
    let science = fs::read(c.root.join("run/4.qe-phonon-nosplit/completion.json")).unwrap();
    fs::remove_file(
        c.root
            .join("run/4.qe-phonon-nosplit/plots/phdos/phonon_dos.png"),
    )
    .unwrap();
    c.accounting(Some(202));
    success(c.numbered_cli(0, &["-r"], Some(202), "qe-plot-ph-repair"));
    png(&c, "run/4.qe-phonon-nosplit/plots/phdos/phonon_dos.png");
    assert_eq!(calls, c.calls());
    assert_eq!(
        science,
        fs::read(c.root.join("run/4.qe-phonon-nosplit/completion.json")).unwrap()
    );
}

#[test]
fn incomplete_atomic_projection_is_rejected_without_fake_element_plots() {
    let c = Case::numbered();
    let file = c.root.join("qe_native_io.py");
    let source = fs::read_to_string(&file).unwrap();
    assert!(source.contains("['0.1']*nat"));
    fs::write(file, source.replace("['0.1']*nat", "['0.1']*(nat-1)")).unwrap();
    c.accounting(Some(201));
    let o = c.numbered_cli(0, &["-r"], Some(201), "qe-plot-ph-incomplete");
    assert!(!o.status.success());
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("原子投影列"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(c
        .root
        .join("run/4.qe-phonon-nosplit/qe-post-completion.json")
        .is_file());
    assert!(!c
        .root
        .join("run/4.qe-phonon-nosplit/plots/completion.json")
        .exists());
    assert!(!c
        .root
        .join("run/4.qe-phonon-nosplit/plots/phdos/phonon_dos.png")
        .exists());
}

#[test]
fn phonon_plot_failure_resumes_from_numeric_checkpoint_without_rerunning_matdyn() {
    let c = Case::numbered();
    let blocked = c
        .root
        .join("run/4.qe-phonon-nosplit/plots/phband/phonon_band.png");
    // 新阶段归档会移动旧文件，因此在 matdyn 替身结束时注入真实图片写入失败。
    let file = c.root.join("qe_native_io.py");
    let source = fs::read_to_string(&file).unwrap();
    fs::write(&file, source.replace("if value('fldos'):", "if value('fldos'):\n            if (ROOT/'block-plot').exists(): Path('plots/phband/phonon_band.png').mkdir(parents=True,exist_ok=True)")).unwrap();
    c.put("block-plot", "fail bitmap write");
    c.accounting(Some(201));
    let o = c.numbered_cli(0, &["-r"], Some(201), "qe-plot-ph-write-failed");
    assert!(!o.status.success());
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("plot_failed"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let calls = c.calls();
    fs::remove_dir(&blocked).unwrap();
    fs::remove_file(c.root.join("block-plot")).unwrap();
    c.accounting(Some(202));
    success(c.numbered_cli(0, &["--resume", "-r"], Some(202), "qe-plot-ph-write-resume"));
    png(&c, "run/4.qe-phonon-nosplit/plots/phband/phonon_band.png");
    assert_eq!(calls, c.calls(), "PH、q2r、matdyn 均不得重跑");
}
