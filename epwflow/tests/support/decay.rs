//! 0051：实际公开进程／阶段脚本的衰减交付与恢复；科学执行使用原生 IO 替身。
use super::*;

#[test]
fn decay_is_published_before_comparison_failure_and_repairs_without_rerunning_science() {
    let c = Case::numbered();
    // 先完成 PH 单侧后处理；后面的故障只针对第 6 步的比较任务。
    c.accounting(Some(201));
    success(c.numbered_cli(
        0,
        &["--stage", "phonon", "-r"],
        Some(201),
        "phonon-reference",
    ));
    c.stage_setup();
    c.put("job-201.state", "COMPLETED");
    c.put("submissions", "allocated preparation\n");
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    c.put("comparison-failure", "matdyn");
    success(c.stage_submit("coarse", false));
    assert!(!c.stage_execute().status.success());
    let science = c.root.join("run/6.epw-coarse/epwdata.fmt");
    let receipt = fs::read(&science).unwrap();
    let native = c.root.join("run/6.epw-coarse");
    let result = c.root.join("run/results/6.epw-coarse/decay");
    let index: Value = store::read(&c.root.join("run/results/index.json")).unwrap();
    assert!(index["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["stage"] == "6.epw-coarse" && e["unit"] == "decay"));
    for stem in [
        "decay-H",
        "decay-dynmat",
        "decay-epmate",
        "decay-epmatp",
        "decay-overview",
    ] {
        for format in ["png", "svg", "pdf"] {
            assert!(result.join(format!("{stem}.{format}")).is_file());
        }
    }
    assert!(!c.root.join("run/7.epw-fine").exists());
    assert!(!c
        .root
        .join("run/6.epw-coarse/comparison-native/sources.json")
        .exists());
    let calls = c.calls();
    let post = c.post_calls();
    fs::remove_file(native.join("decay/decay-H.png")).unwrap();
    fs::remove_file(result.join("decay-overview.pdf")).unwrap();
    // 模拟分配被终止后的遗留锁：只恢复原作业，错误所有者必须保留。
    let job = epwflow::foundation::readable_state::jobs(
        &c.root.join("run"), epwflow::foundation::stages::Step::Coarse,
    ).unwrap()["Stage allocation"];
    let locks = [
        native.join("decay/decay-publication.lock"),
        result.join("stage-publication.lock"),
        c.root.join("run/results/index.lock"),
    ];
    for lock in &locks {
        fs::create_dir(lock).unwrap();
        store::write(
            &lock.join("owner.json"),
            &serde_json::json!({
                "host":"fixture", "pid":1, "token":"interrupted-decay",
                "created_unix":0, "job_id":job + 1
            }),
        )
        .unwrap();
    }
    assert!(!c.stage_submit("coarse", true).status.success());
    assert!(locks.iter().all(|lock| lock.join("owner.json").is_file()));
    for lock in &locks {
        store::write(
            &lock.join("owner.json"),
            &serde_json::json!({
                "host":"fixture", "pid":1, "token":"interrupted-decay",
                "created_unix":0, "job_id":job
            }),
        )
        .unwrap();
    }
    // 对比仍失败时也应补齐独立衰减交付，不能重新调用粗网格科学程序。
    success(c.stage_submit("coarse", true));
    assert!(locks.iter().all(|lock| !lock.exists()));
    assert!(!c.stage_execute().status.success());
    assert!(native.join("decay/decay-H.png").is_file());
    assert!(result.join("decay-overview.pdf").is_file());
    assert_eq!(receipt, fs::read(&science).unwrap());
    assert_eq!(calls.1, c.calls().1);
    assert_eq!(post, c.post_calls());
    fs::remove_file(c.root.join("comparison-failure")).unwrap();
    success(c.stage_submit("coarse", true));
    success(c.stage_execute());
    assert_eq!(
        calls.1.lines().filter(|l| l.starts_with("coarse ")).count(),
        c.calls()
            .1
            .lines()
            .filter(|l| l.starts_with("coarse "))
            .count()
    );
    let stable = c.calls();
    let before_post = c.post_calls();
    let before = store::hash_file(&native.join("decay.H")).unwrap();
    let moved = c.root.join("portable-decay");
    copy_results(&c.root.join("run/results"), &moved);
    fs::remove_file(moved.join("6.epw-coarse/decay/decay-H.png")).unwrap();
    let mut command = Command::new(BINS[1]);
    command
        .args(["--plot", "--results", moved.to_str().unwrap()])
        .env("PATH", "");
    success(command.output().unwrap());
    epwflow::foundation::stage_results::check(&moved).unwrap();
    assert_eq!(before, store::hash_file(&native.join("decay.H")).unwrap());
    assert_eq!(stable, c.calls());
    assert_eq!(before_post, c.post_calls());
    // 原生诊断缺失只报告派生缺项，不能让粗网格科学重新提交。
    fs::remove_file(native.join("decay.H")).unwrap();
    fs::remove_file(native.join("decay.epmatp")).unwrap();
    let prepared = c.stage_submit("coarse", false);
    let out = if prepared.status.success() { c.stage_execute() } else { prepared };
    assert!(!out.status.success());
    let error = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        error.contains("decay.H") && error.contains("decay.epmatp"),
        "{error}"
    );
    assert_eq!(stable, c.calls());
    assert_eq!(receipt, fs::read(&science).unwrap());
}
