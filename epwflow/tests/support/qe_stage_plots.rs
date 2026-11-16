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
    assert!(!c.root.join("run/3.qe-electronic/plot/electronic.png").exists());
    assert!(c.root.join("run/results/3.qe-electronic/band-ranges").is_dir());
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
    png(&c, "run/3.qe-electronic/plot/electronic.png");
    assert_eq!(calls, c.calls(), "Success PW Don't run again");
    let bands = c
        .root
        .join("run/3.qe-electronic/plot/electronic.png");
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
fn electronic_allocation_publishes_both_leaves_and_repairs_without_science() {
    let c=Case::numbered();c.stage_setup();
    success(c.stage_submit("electronic",false));success(c.stage_execute());
    png(&c,"run/results/3.qe-electronic/plot/electronic.png");
    png(&c,"run/results/3.qe-electronic/plot/electronic.png");
    let calls=c.calls();
    fs::remove_file(c.root.join("run/3.qe-electronic/plot/electronic.png")).unwrap();
    success(c.stage_submit("electronic",false));success(c.stage_execute());
    assert_eq!(calls,c.calls());
    png(&c,"run/results/3.qe-electronic/plot/electronic.png");
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
    png(&c, "run/4.qe-phonon-nosplit/plot/phonon.png");
    png(&c, "run/4.qe-phonon-nosplit/plot/phonon.png");
    let csv = c
        .root
        .join("run/4.qe-phonon-nosplit/plot/phdos/phonon_dos_elements.csv");
    let text = fs::read_to_string(csv).unwrap();
    assert!(text.starts_with("frequency_cm-1,TDOS,Si,C\n"));
    for line in text.lines().skip(1) {
        let row: Vec<f64> = line.split(',').map(|v| v.parse().unwrap()).collect();
        assert!((row[1] - row[2] - row[3]).abs() < 1e-6);
        assert!((row[3] - 2.0 * row[2]).abs() < 1e-6);
    }
    assert!(!c.root.join("run/5.epw-wannier").exists());
    let calls = c.calls();
    let science = fs::read(c.root.join("run/4.qe-phonon-nosplit/save/scf.dvscf_q2")).unwrap();
    fs::remove_file(
        c.root
            .join("run/4.qe-phonon-nosplit/plot/phonon.png"),
    )
    .unwrap();
    c.accounting(Some(202));
    success(c.numbered_cli(0, &["-r"], Some(202), "qe-plot-ph-repair"));
    png(&c, "run/4.qe-phonon-nosplit/plot/phonon.png");
    assert_eq!(calls, c.calls());
    assert_eq!(
        science,
        fs::read(c.root.join("run/4.qe-phonon-nosplit/save/scf.dvscf_q2")).unwrap()
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
        String::from_utf8_lossy(&o.stderr).contains("atomic projection columns"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(!c.root.join("run/4.qe-phonon-nosplit/qe-post-completion.json").exists());
    assert!(!c.root.join("run/4.qe-phonon-nosplit/plot/completion.json").exists());
    assert!(!c
        .root
        .join("run/4.qe-phonon-nosplit/plot/phonon.png")
        .exists());
}

#[test]
fn phonon_plot_failure_resumes_from_numeric_checkpoint_without_rerunning_matdyn() {
    let c = Case::numbered();
    let blocked = c
        .root
        .join("run/4.qe-phonon-nosplit/plot/phonon.png");
    // 新阶段归档会移动旧文件，因此在 matdyn 替身结束时注入真实图片写入失败。
    let file = c.root.join("qe_native_io.py");
    let source = fs::read_to_string(&file).unwrap();
    fs::write(&file, source.replace("if value('fldos'):", "if value('fldos'):\n            if (ROOT/'block-plot').exists(): Path('plot/phonon.png').mkdir(parents=True,exist_ok=True)")).unwrap();
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
    png(&c, "run/4.qe-phonon-nosplit/plot/phonon.png");
    assert_eq!(calls, c.calls(), "PH、q2r、matdyn Don't run again");
}

#[test]
fn element_pdos_failure_preserves_tdos_and_bands_and_resumes_without_pw_or_dos() {
    let c=Case::numbered();c.stage_setup();c.put("fail-pdos","controlled projection failure");
    success(c.stage_submit("electronic",false));assert!(!c.stage_execute().status.success());
    let root=c.root.join("run");
    let dos=fs::read(root.join("3.qe-electronic/qe_eledos/scf.tdos")).unwrap();
    assert!(root.join("results/3.qe-electronic/band-ranges").is_dir());
    let incomplete:Value=store::read(&root.join("results/3.qe-electronic/plot/electronic-pdos.json")).unwrap();
    assert_eq!(incomplete["complete"],false);
    let calls=c.calls();fs::remove_file(c.root.join("fail-pdos")).unwrap();
    success(c.stage_submit("electronic",true));success(c.stage_execute());
    assert_eq!(dos,fs::read(root.join("3.qe-electronic/qe_eledos/scf.tdos")).unwrap());assert_eq!(calls,c.calls());
    let pdos=c.root.join("pdos-calls");let native=fs::read(&pdos).unwrap();
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    epwflow::foundation::readable_state::update(&cfg.work_dir, epwflow::foundation::stages::Step::Electronic, |d| { d.remove("Electronic Projection"); Ok(()) }).unwrap();
    fs::remove_file(root.join("results/3.qe-electronic/plot/electronic-from-lowest-to-highest.png")).unwrap();
    success(c.stage_submit("electronic",false));success(c.stage_execute());
    assert_eq!(native,fs::read(pdos).unwrap(),"The native projection has been completed and should not be recalculated because of the derivative receipts");
    let complete:Value=store::read(&root.join("results/3.qe-electronic/plot/electronic-pdos.json")).unwrap();assert_eq!(complete["complete"],true);
}

#[test]
fn combined_plots_use_shared_config_and_qe_path_extrema_without_scientific_reruns() {
    let c=Case::numbered();c.stage_setup();
    success(c.stage_submit("wannier",false));success(c.stage_execute());
    let root=c.root.join("run");
    let read=|stage:&str,name:&str|->Value {
        store::read(&root.join(stage).join("plot").join(format!("{name}.plot.json"))).unwrap()
    };
    let qe_full=read("3.qe-electronic","electronic-from-lowest-to-highest");
    let epw_full=read("5.epw-wannier","electronic-from-lowest-to-highest");
    assert_eq!(qe_full["bounds"],epw_full["bounds"]);
    for stage in ["3.qe-electronic","5.epw-wannier"] {
        assert_eq!(read(stage,"electronic")["bounds"],serde_json::json!([-3.,3.]));
        let images=fs::read_dir(root.join(stage).join("plot")).unwrap()
            .filter_map(|e|e.ok()).filter(|e|e.path().extension().is_some_and(|s|s=="png")).count();
        assert_eq!(images,2);
        assert!(!root.join(stage).join("electronic-bands.png").exists());
    }
    let calls=(c.calls(),c.post_calls());
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    let keys=epwflow::foundation::stages::Step::ALL.map(|s|epwflow::foundation::stages::scientific_key(&cfg,s).unwrap());
    c.numbered_change(|cfg|{cfg.plot.electronic_energy_min_ev=-1.5;cfg.plot.electronic_energy_max_ev=4.0;});
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    assert_eq!(keys,epwflow::foundation::stages::Step::ALL.map(|s|epwflow::foundation::stages::scientific_key(&cfg,s).unwrap()));
    success(c.stage_submit("wannier",false));success(c.stage_execute());
    assert_eq!(calls,(c.calls(),c.post_calls()));
    for stage in ["3.qe-electronic","5.epw-wannier"] {
        assert_eq!(read(stage,"electronic")["bounds"],serde_json::json!([-1.5,4.]));
        assert_eq!(read(stage,"electronic-from-lowest-to-highest")["bounds"],qe_full["bounds"]);
    }
}

#[test]
fn stage_three_and_five_plot_commands_migrate_saved_legacy_curves_without_science() {
    let c=Case::numbered();c.stage_setup();
    success(c.stage_submit("wannier",false));success(c.stage_execute());
    let root=c.root.join("run");
    for stage in ["3.qe-electronic","5.epw-wannier"] {
        let path=root.join(stage);
        for (name,leaf) in [("electronic-bands","qe_eband"),("electronic-dos","qe_eledos")] {
            let data=if stage.starts_with('3') {path.join("plot").join(format!("{name}.json"))}
                else {path.join("comparison-native").join(format!("{name}.json"))};
            let legacy=if stage.starts_with('3') {path.join(leaf).join("plots")}
                else {path.join("comparison-native")};
            fs::create_dir_all(&legacy).unwrap();
            fs::copy(&data,legacy.join(format!("{name}.plot.json"))).unwrap();
            fs::remove_file(data).unwrap();
        }
        for name in ["electronic","electronic-from-lowest-to-highest"] {
            for ext in ["png","plot.json"] {fs::remove_file(path.join("plot").join(format!("{name}.{ext}"))).unwrap();}
        }
    }
    c.numbered_change(|cfg|{cfg.plot.electronic_energy_min_ev=-2.0;cfg.plot.electronic_energy_max_ev=4.0;});
    let before=(c.calls(),c.post_calls(),fs::read(c.root.join("submissions")).unwrap());
    success(c.numbered_cli(0,&["--stage","electronic","--plot"],None,"plot-stage-three"));
    assert!(!root.join("5.epw-wannier/plot/electronic.png").exists());
    success(c.numbered_cli(1,&["--plot"],None,"plot-stage-five"));
    assert_eq!(before,(c.calls(),c.post_calls(),fs::read(c.root.join("submissions")).unwrap()));
    for stage in ["3.qe-electronic","5.epw-wannier"] {
        let dir=root.join(stage).join("plot");
        let data:Value=store::read(&dir.join("electronic.plot.json")).unwrap();
        assert_eq!(data["bounds"],serde_json::json!([-2.,4.]));
        assert_eq!(fs::read_dir(&dir).unwrap().filter_map(|f|f.ok()).filter(|f|f.path().extension().is_some_and(|e|e=="png")).count(),2);
    }
}
