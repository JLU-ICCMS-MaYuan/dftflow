//! 外部 PH 导入的公开进程测试。PH 替身只用于生成历史来源，不参与导入。
use super::*;
use epwflow::{
    foundation::stage::Backend,
    foundation::{
        layout,
        stages::{self, Step},
    },
    qe_data::{self, qe_config::QeSettings},
};
fn execute_import(c: &Case, job: u64, label: &str) -> Output {
    let script = c.root.join("run/4.qe-phonon-nosplit/phonon.sh");
    let mut cmd = c.scheduled_command(&script, script.parent().unwrap(), job);
    cmd.env("SLURM_NTASKS", "1");
    let out = cmd.output().unwrap();
    c.put(
        &format!("{label}.stderr"),
        &String::from_utf8_lossy(&out.stderr),
    );
    c.put(
        &format!("{label}.stdout"),
        &String::from_utf8_lossy(&out.stdout),
    );
    out
}
pub(super) fn prepared() -> (Case, PathBuf) {
    let c = Case::numbered();
    c.numbered_change(|cfg| cfg.phonon.q_grid = [2, 1, 1]);
    c.accounting(Some(201));
    success(c.numbered_cli(0, &["--stage", "scf", "-r"], Some(201), "import-scf"));
    c.accounting(Some(202));
    success(c.numbered_cli(0, &["--stage", "nscf", "-r"], Some(202), "import-nscf"));
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let qe = QeSettings::from_config(&cfg).unwrap();
    let (qs, rs) = qe_data::phonon::run_nosplit(&qe, &cfg.work_dir, &[], Backend::Local).unwrap();
    let source = c.root.join("external phonon/save");
    fs::create_dir_all(source.join("scf.phsave")).unwrap();
    fs::copy(&qs[0].source_dyn0, source.join("scf.dyn0")).unwrap();
    fs::copy(
        &rs[0].control_metadata,
        source.join("scf.phsave/control_ph.xml"),
    )
    .unwrap();
    for r in rs {
        let raw = fs::read_to_string(&r.dyn_file).unwrap();
        let mut doc=raw.replace("<GEOMETRY_INFO>","<GEOMETRY_INFO><NUMBER_OF_TYPES>1</NUMBER_OF_TYPES><TYPE_NAME.1>Si</TYPE_NAME.1><MASS.1>28.085</MASS.1>");
        let tail = (1..=6)
            .map(|m| format!("<DISPLACEMENT.{m}>0 0 0 0 0 0 0 0 0 0 0 0</DISPLACEMENT.{m}>"))
            .collect::<String>();
        doc = doc.replace(
            "</FREQUENCIES_THZ_CMM1>",
            &(tail + "</FREQUENCIES_THZ_CMM1>"),
        );
        fs::write(source.join(format!("scf.dyn_q{}.xml", r.q_id)), doc).unwrap();
        fs::copy(
            r.patterns,
            source.join(format!("scf.phsave/patterns.{}.xml", r.q_id)),
        )
        .unwrap();
        fs::copy(
            r.dvscf.unwrap(),
            source.join(format!("scf.dvscf_q{}", r.q_id)),
        )
        .unwrap();
    }
    fs::rename(
        cfg.work_dir.join("4.qe-phonon-nosplit"),
        c.root.join("historical-ph"),
    )
    .unwrap();
    c.accounting(None);
    (c, source)
}
#[test]
fn external_import_one_job_no_pw_ph_idempotent_and_plot_repair() {
    let (c, source) = prepared();
    let args = ["--import-phonon", source.to_str().unwrap(), "-s"];
    let before = c.calls().0;
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let src = qe_data::save::files(&source).unwrap();
    let electronic = qe_data::save::files(&layout::path(&cfg.work_dir, "nscf/out")).unwrap();
    success(c.numbered_cli(
        0,
        &["--import-phonon", source.to_str().unwrap(), "--check"],
        None,
        "import-check",
    ));
    assert!(!cfg.work_dir.join("epw-state/imports").exists());
    let mut elsewhere = Command::new(BINS[0]);
    c.environment(&mut elsewhere, None);
    elsewhere.current_dir(&cfg.work_dir).args([
        "--config",
        c.root.join("input.toml").to_str().unwrap(),
        "--import-phonon",
        "external phonon/save",
        "--check",
    ]);
    success(elsewhere.output().unwrap());
    assert!(!cfg.work_dir.join("epw-state/imports").exists());
    let overlap = cfg.work_dir.join("epw-state/invalid-source");
    fs::create_dir_all(&overlap).unwrap();
    let rejected = c.numbered_cli(
        0,
        &["--import-phonon", overlap.to_str().unwrap()],
        None,
        "import-overlap",
    );
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("overlaps"));
    assert!(!cfg.work_dir.join("epw-state/imports").exists());
    success(c.numbered_cli(0, &args, None, "import-submit"));
    c.accounting(Some(201));
    success(execute_import(&c, 201, "import-allocation"));
    c.accounting(None);
    assert!(stages::reusable(&cfg, Step::Phonon).unwrap());
    assert!(!cfg.work_dir.join("3.qe-electronic").exists());
    let after = c.calls().0;
    assert_eq!(
        before
            .lines()
            .filter(|s| ["pw", "ph"].contains(s))
            .collect::<Vec<_>>(),
        after
            .lines()
            .filter(|s| ["pw", "ph"].contains(s))
            .collect::<Vec<_>>()
    );
    assert_eq!(src, qe_data::save::files(&source).unwrap());
    assert_eq!(
        electronic,
        qe_data::save::files(&layout::path(&cfg.work_dir, "nscf/out")).unwrap()
    );
    let submitted = fs::read_to_string(c.root.join("submissions")).unwrap();
    success(c.numbered_cli(0, &args, None, "import-repeat"));
    assert_eq!(
        submitted,
        fs::read_to_string(c.root.join("submissions")).unwrap()
    );
    let image = cfg
        .work_dir
        .join("4.qe-phonon-nosplit/plot/phonon.png");
    assert!(image.is_file());
    fs::remove_file(&image).unwrap();
    success(c.numbered_cli(0, &args, None, "import-repair-submit"));
    c.accounting(Some(202));
    success(execute_import(&c, 202, "import-repair"));
    assert!(image.is_file());
    assert_eq!(after, c.calls().0);
    c.accounting(None);
    // 即使来源绑定、报告和 QE 清单都被误删，剩余 PH 回执仍须阻止 PH 回退。
    for path in [
        cfg.work_dir.join("epw-state/imports/phonon-source.json"),
        cfg.work_dir.join("epw-state/imports/external-source.json"),
    ] {
        fs::remove_file(path).unwrap();
    }
    let calls = c.calls().0;
    assert!(!c
        .numbered_cli(
            1,
            &["--stage", "coarse", "-s"],
            None,
            "import-missing-markers"
        )
        .status
        .success());
    assert_eq!(calls, c.calls().0);
}
#[test]
fn external_import_bad_source_and_failed_post_never_fall_back_to_ph() {
    let (c, source) = prepared();
    let before = c.calls().0;
    let response = source.join("scf.dvscf_q2");
    let original = fs::read(&response).unwrap();
    fs::write(&response, [0u8; 8]).unwrap();
    let failed = c.numbered_cli(
        0,
        &["--import-phonon", source.to_str().unwrap(), "--check"],
        None,
        "import-truncated",
    );
    assert!(!failed.status.success());
    assert_eq!(before, c.calls().0);
    fs::write(&response, &original).unwrap();
    c.put("post-failure", "fail q2r");
    let args = ["--import-phonon", source.to_str().unwrap(), "-s"];
    success(c.numbered_cli(0, &args, None, "import-submit"));
    c.accounting(Some(201));
    assert!(!execute_import(&c, 201, "import-failed-post")
        .status
        .success());
    c.accounting(None);
    assert!(!c
        .numbered_cli(0, &args, None, "import-must-resume")
        .status
        .success());
    fs::remove_file(c.root.join("post-failure")).unwrap();
    success(c.numbered_cli(
        0,
        &[
            "--import-phonon",
            source.to_str().unwrap(),
            "--resume",
            "-s",
        ],
        None,
        "import-resume",
    ));
    c.accounting(Some(202));
    success(execute_import(&c, 202, "import-resumed"));
    assert_eq!(
        before
            .lines()
            .filter(|s| ["pw", "ph"].contains(s))
            .collect::<Vec<_>>(),
        c.calls()
            .0
            .lines()
            .filter(|s| ["pw", "ph"].contains(s))
            .collect::<Vec<_>>()
    );
}
#[test]
fn external_import_rejects_duplicate_wrong_q_patterns_nan_and_missing_files() {
    let (c, source) = prepared();
    let before = c.calls().0;
    let dynfile = source.join("scf.dyn_q2.xml");
    let orig = fs::read_to_string(&dynfile).unwrap();
    fs::copy(&dynfile, source.join("scf.dyn_q2")).unwrap();
    assert!(!c
        .numbered_cli(
            0,
            &["--import-phonon", source.to_str().unwrap(), "--check"],
            None,
            "import-duplicate"
        )
        .status
        .success());
    fs::remove_file(source.join("scf.dyn_q2")).unwrap();
    for (name, replacement) in [
        ("import-wrong-q", orig.replace("<Q_POINT>", "<Q_POINT>1 ")),
        (
            "import-star-count",
            orig.replace("<NUMBER_OF_Q>1", "<NUMBER_OF_Q>2"),
        ),
        (
            "import-missing-mode",
            orig.replace("<OMEGA.6>1 100</OMEGA.6>", ""),
        ),
    ] {
        fs::write(&dynfile, replacement).unwrap();
        assert!(!c
            .numbered_cli(
                0,
                &["--import-phonon", source.to_str().unwrap(), "--check"],
                None,
                name
            )
            .status
            .success());
    }
    fs::write(&dynfile, &orig).unwrap();
    let pattern = source.join("scf.phsave/patterns.2.xml");
    let original = fs::read(&pattern).unwrap();
    fs::remove_file(&pattern).unwrap();
    assert!(!c
        .numbered_cli(
            0,
            &["--import-phonon", source.to_str().unwrap(), "--check"],
            None,
            "import-missing-pattern"
        )
        .status
        .success());
    fs::write(pattern, original).unwrap();
    let response = source.join("scf.dvscf_q2");
    let mut bytes = fs::read(&response).unwrap();
    bytes[..8].copy_from_slice(&f64::NAN.to_le_bytes());
    fs::write(response, bytes).unwrap();
    assert!(!c
        .numbered_cli(
            0,
            &["--import-phonon", source.to_str().unwrap(), "--check"],
            None,
            "import-nan"
        )
        .status
        .success());
    assert_eq!(before, c.calls().0);
}
#[test]
fn external_import_upgrades_nested_config_without_changing_electronic_outputs() {
    let (c, source) = prepared();
    let path = c.root.join("input.toml");
    let cfg = config::Config::load(&path).unwrap();
    let before = ["1.qe-scf/scf.out", "2.qe-nscf/nscf.out"].map(|p| store::Identity::capture(&cfg.work_dir.join(p)).unwrap());
    let calls=c.calls().0;
    let mut old = serde_json::to_value(&cfg).unwrap();
    old["stages"] = cfg.stage_document().unwrap();
    old.as_object_mut().unwrap().remove("workflow");
    fn remove_null(v: &mut Value) {
        match v {
            Value::Object(m) => {
                m.retain(|_, v| !v.is_null());
                for v in m.values_mut() {
                    remove_null(v);
                }
            }
            Value::Array(a) => {
                for v in a {
                    remove_null(v);
                }
            }
            _ => (),
        }
    }
    remove_null(&mut old);
    let legacy = toml::to_string_pretty(&old).unwrap();
    fs::write(&path, &legacy).unwrap();

    success(c.numbered_cli(
        0,
        &["--import-phonon", source.to_str().unwrap()],
        None,
        "import-upgrade-prepare",
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(),legacy, "Import cannot rewrite original user configuration");
    assert!(fs::read_dir(cfg.work_dir.join("epw-state")).unwrap().flatten().any(|e|e.file_name().to_string_lossy().ends_with("-config.toml") && fs::read_to_string(e.path()).ok().as_deref()==Some(legacy.as_str())));
    assert!(!cfg.work_dir.join("epw-state/imports/config-upgrade.json").exists());
    success(c.numbered_cli(0,&["--import-phonon",source.to_str().unwrap(),"-s"],None,"nested-import-submit"));
    c.accounting(Some(201));
    success(execute_import(&c,201,"nested-import-worker"));
    for file in before {file.verify().unwrap();}
    assert_eq!(calls,c.calls().0.lines().filter(|s| ["pw","ph"].contains(s)).map(|s|format!("{s}\n")).collect::<String>());

}
#[test]
fn external_import_changed_science_or_electronic_evidence_rejects_rebinding() {
    let (c, source) = prepared();
    let path = c.root.join("input.toml");
    let original = fs::read(&path).unwrap();
    let cfg = config::Config::load(&path).unwrap();
    let before = c.calls().0;
    let mut v = serde_json::to_value(&cfg).unwrap();
    v["stages"]["2.qe-nscf"]["ecutwfc"] = json!(41.0);
    let updated: config::Config = serde_json::from_value(v).unwrap();
    fs::write(&path, toml::to_string_pretty(&updated).unwrap()).unwrap();
    assert!(!c
        .numbered_cli(
            0,
            &["--import-phonon", source.to_str().unwrap()],
            None,
            "import-physics-change"
        )
        .status
        .success());
    fs::write(&path, original).unwrap();
    let xml = cfg
        .work_dir
        .join("2.qe-nscf/out/scf.save/data-file-schema.xml");
    fs::write(xml, "corrupted").unwrap();
    assert!(!c
        .numbered_cli(
            0,
            &["--import-phonon", source.to_str().unwrap()],
            None,
            "import-damaged-electrons"
        )
        .status
        .success());
    assert_eq!(before, c.calls().0);
    assert!(!c.root.join("submissions").exists());
}
