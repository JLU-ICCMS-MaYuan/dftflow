//! 0072：各阶段自动绘图、失败隔离与当前阶段补图。
use super::*;


// Issue #72 独立验收探针；只使用受控进程，不执行真实科学计算。
#[test]
fn issue_0072_six_stage_image_inventory() {
    let c = Case::numbered();
    c.stage_setup();
    // 真实 EPW 的列表定向输出将矩阵分成三行，不能只测试打包成一行的替身。
    let file = c.root.join("epw_native_io.py");
    let native = fs::read_to_string(&file).unwrap();
    let packed = "' '.join(str(x) for row in bg for x in row)";
    assert!(native.contains(packed));
    fs::write(file, native.replace(packed, "'\\n'.join(' '.join(str(x) for x in row) for row in bg)")).unwrap();
    let mut inventory = serde_json::Map::new();
    for (stage, folder) in [
        ("scf", "1.qe-scf"), ("nscf", "2.qe-nscf"),
        ("electronic", "3.qe-electronic"), ("phonon", "4.qe-phonon-nosplit"),
        ("wannier", "5.epw-wannier"), ("coarse", "6.epw-coarse"),
    ] {
        success(c.stage_submit(stage, false));
        let step = epwflow::foundation::stages::Step::ALL.into_iter().find(|s| s.name() == stage).unwrap();
        let doc = epwflow::foundation::readable_state::read(&c.root.join("run"), step).unwrap();
        assert_eq!(doc.get("Stage allocation", "Status"), Some("submitted"), "{stage}: must submit a whole-stage allocation");
        let script = epwflow::foundation::readable_state::resolve(&c.root.join("run"), doc.get("Stage allocation", "Script").unwrap());
        assert!(fs::read_to_string(script).unwrap().contains(&format!("--allocated-stage {stage}")));
        success(c.stage_execute());
        let mut names = fs::read_dir(c.root.join("run/results").join(folder)).unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|s| s.ends_with(".png")).collect::<Vec<_>>();
        names.sort();
        eprintln!("stage={stage} images={names:?}");
        inventory.insert(stage.into(), json!(names));
    }
    store::write(&c.root.join("issue-0072-inventory.json"), &inventory).unwrap();
    eprintln!("Issue #72 inventory: {}", serde_json::to_string_pretty(&inventory).unwrap());
    for (stage, required) in [
        ("scf", vec!["scf-band-ranges.png"]),
        ("nscf", vec!["nscf-band-ranges.png"]),
        ("electronic", vec!["electronic-band-ranges.png", "electronic.png", "electronic-from-lowest-to-highest.png"]),
        ("phonon", vec!["phonon.png"]),
        ("wannier", vec!["electronic.png", "electronic-from-lowest-to-highest.png"]),
        ("coarse", vec!["phonon.png", "decay-overview.png"]),
    ] {
        for name in required {
            assert!(inventory[stage].as_array().unwrap().iter().any(|n| n == name), "{stage}: missing {name}");
        }
    }
}

#[test]
fn issue_0072_electronic_renderer_failure_keeps_scientific_completion() {
    use epwflow::foundation::{readable_state as logs, stages::Step};
    let c = Case::numbered(); c.stage_setup();
    issue_0072_seed(&c, &["scf", "nscf"]);
    let dos = fs::read_to_string(c.root.join("dos.x")).unwrap();
    let marker = "kind=\"dos\"";
    assert!(dos.contains(marker));
    c.script("dos.x", &dos.replace(marker, &format!(
        "{marker}\nPath(__file__).with_name('run').joinpath('results/3.qe-electronic/electronic.png').mkdir(parents=True, exist_ok=True)")));
    success(c.stage_submit("electronic", false));
    let output = c.stage_execute();
    let doc = logs::read(&c.root.join("run"), Step::Electronic).unwrap();
    eprintln!("worker_success={} scientific_status={:?} stage_status={:?}\n{}",
        output.status.success(), doc.get("Stage", "Scientific status"), doc.get("Stage", "Status"),
        String::from_utf8_lossy(&output.stderr));
    assert!(c.root.join("run/3.qe-electronic/qe_eledos/scf.tdos").is_file());
    assert_eq!(doc.get("Stage", "Scientific status"), Some("completed"),
        "FR-007/SC-005: Electronic 绘图失败影响科学完成标记");
}

#[test]
fn issue_0072_pdos_failure_keeps_band_and_total_dos_plots() {
    use epwflow::foundation::{readable_state as logs, stages::Step};
    let c = Case::numbered(); c.stage_setup();
    issue_0072_seed(&c, &["scf", "nscf"]);
    c.put("fail-pdos", "controlled projection failure");
    success(c.stage_submit("electronic", false));
    let output = c.stage_execute();
    let doc = logs::read(&c.root.join("run"), Step::Electronic).unwrap();
    let root = c.root.join("run/results/3.qe-electronic");
    eprintln!("worker_success={} scientific_status={:?} band_ranges={} electronic={}\n{}",
        output.status.success(), doc.get("Stage", "Scientific status"),
        root.join("electronic-band-ranges.png").is_file(), root.join("electronic.png").is_file(),
        String::from_utf8_lossy(&output.stderr));
    assert!(c.root.join("run/3.qe-electronic/qe_eledos/scf.tdos").is_file());
    assert!(root.join("electronic-band-ranges.png").is_file());
    assert!(root.join("electronic.png").is_file(),
        "US2/SC-002: PDOS 失败后电子能带和总 DOS 图没有保留");
}

#[test]
fn issue_0072_phonon_renderer_failure_keeps_scientific_completion() {
    use epwflow::foundation::{readable_state as logs, stages::Step};
    let c = Case::numbered(); c.stage_setup();
    issue_0072_seed(&c, &["scf"]);
    let file = c.root.join("qe_native_io.py");
    let text = fs::read_to_string(&file).unwrap();
    let marker = "if value('fldos'):";
    assert!(text.contains(marker));
    fs::write(&file, text.replace(marker,
        "if value('fldos'):\n            (ROOT/'run/results/4.qe-phonon-nosplit/phonon.png').mkdir(parents=True,exist_ok=True)")).unwrap();
    success(c.stage_submit("phonon", false));
    let output = c.stage_execute();
    let doc = logs::read(&c.root.join("run"), Step::Phonon).unwrap();
    eprintln!("worker_success={} scientific_status={:?} stage_status={:?} allocation={:?}\n{}",
        output.status.success(), doc.get("Stage", "Scientific status"), doc.get("Stage", "Status"),
        doc.get("Stage allocation", "Status"), String::from_utf8_lossy(&output.stderr));
    assert_eq!(doc.get("Stage", "Scientific status"), Some("completed"));
    // 作业失败与科学完成分别记录；只要求科学数据和完成标记保留。
}

fn issue_0072_seed(c: &Case, stages: &[&str]) {
    for &stage in stages {
        success(c.stage_submit(stage, false));
        let step = epwflow::foundation::stages::Step::ALL.into_iter().find(|s| s.name() == stage).unwrap();
        let doc = epwflow::foundation::readable_state::read(&c.root.join("run"), step).unwrap();
        assert_eq!(doc.get("Stage allocation", "Status"), Some("submitted"), "{stage}: must submit a whole-stage allocation");
        let script = epwflow::foundation::readable_state::resolve(&c.root.join("run"), doc.get("Stage allocation", "Script").unwrap());
        assert!(fs::read_to_string(script).unwrap().contains(&format!("--allocated-stage {stage}")));
        success(c.stage_execute());
    }
}

#[test]
fn issue_0072_scf_nscf_current_stage_repair() {
    let c = Case::numbered(); c.stage_setup();
    issue_0072_seed(&c, &["scf", "nscf"]);
    let calls = (c.calls(), c.post_calls());
    for stage in ["scf", "nscf"] {
        let folder = if stage == "scf" {"1.qe-scf"} else {"2.qe-nscf"};
        let image = c.root.join("run/results").join(folder).join(format!("{stage}-band-ranges.png"));
        let mut reader = png::Decoder::new(fs::File::open(&image).unwrap()).read_info().unwrap();
        assert!(reader.info().width >= 3000 && reader.info().height >= 2000);
        assert_eq!(reader.info().pixel_dims.unwrap().xppu, 11812);
        let mut pixels = vec![0; reader.output_buffer_size()]; reader.next_frame(&mut pixels).unwrap();
        fs::remove_file(&image).unwrap();
        success(c.stage_submit(stage, false)); success(c.stage_execute());
        assert!(image.is_file()); assert_eq!(calls, (c.calls(), c.post_calls()));
    }
}

#[test]
fn issue_0072_next_stage_does_not_repair_upstream_scf_plot() {
    let c = Case::numbered(); c.stage_setup(); issue_0072_seed(&c, &["scf"]);
    let image=c.root.join("run/results/1.qe-scf/scf-band-ranges.png");
    fs::remove_file(&image).unwrap();
    issue_0072_seed(&c, &["nscf"]);
    assert!(!image.exists(), "SC-006: 当前阶段补图不能隐式处理上游");
}

#[test]
fn issue_0072_scf_explicit_result_redraw() {
    let c = Case::numbered(); c.stage_setup(); issue_0072_seed(&c, &["scf"]);
    let image=c.root.join("run/results/1.qe-scf/scf-band-ranges.png");
    fs::remove_file(&image).unwrap();
    let results=image.parent().unwrap().to_str().unwrap();
    success(c.numbered_cli(0, &["--plot", "--results", results], None, "explicit-scf"));
    assert!(image.is_file());
}
