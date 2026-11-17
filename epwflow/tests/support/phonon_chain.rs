//! 0048：通过公开进程和实际 shell 验证一次提交、顺序推进和失败续算。
use super::*;

impl Case {
    fn chain_setup(&self) {
        self.direct_setup();
        self.script(
            "scontrol",
            "#!/bin/bash\nprintf 'JobId=%s JobState=RUNNING NumNodes=1\\n' \"${@: -1}\"\n",
        );
    }
    fn chain_attempt(&self) -> Value {
        use epwflow::foundation::{readable_state as logs, stages::Step};
        let root = self.root.join("run");
        let doc = logs::read(&root, Step::Phonon).unwrap();
        json!({"state":match doc.get("Stage allocation","Status"){Some("completed")=>"succeeded",Some("failed")=>"failed",Some("running")=>"running",Some("submitted")=>"submitted",_=>"unknown"},
            "script":{"path":logs::resolve(&root,doc.get("Stage allocation","Script").unwrap())},
            "job":logs::jobs(&root,Step::Phonon).unwrap().get("Stage allocation")})
    }
    fn chain_submit(&self, resume: bool) -> Output {
        let mut args = vec!["--stage", "phonon", "-s"];
        if resume {
            args.push("--resume");
        }
        self.numbered_cli(0, &args, None, "chain-submit")
    }
    fn chain_execute(&self) -> Output {
        let a = self.chain_attempt();
        let job = a["job"].as_u64().unwrap();
        let script = PathBuf::from(a["script"]["path"].as_str().unwrap());
        let body = fs::read_to_string(&script).unwrap();
        assert!(!body.contains("set -") && !body.contains("sbatch"));
        assert!(body.contains("--allocated-stage") && !body.contains("--workspace-attempt"));
        assert!(Command::new("/bin/bash")
            .arg("-n")
            .arg(&script)
            .status()
            .unwrap()
            .success());
        let mut cmd = self.scheduled_command(&script, script.parent().unwrap(), job);
        cmd.env("SLURM_NTASKS", "1");
        let output = cmd.output().unwrap();
        self.put(
            "chain-worker.stdout",
            &String::from_utf8_lossy(&output.stdout),
        );
        self.put(
            "chain-worker.stderr",
            &String::from_utf8_lossy(&output.stderr),
        );
        self.put(
            &format!("job-{job}.state"),
            if output.status.success() {
                "COMPLETED"
            } else {
                "FAILED"
            },
        );
        output
    }
    fn chain_calls(&self, kind: &str) -> usize {
        self.calls().0.lines().filter(|s| *s == kind).count()
    }
    fn submission_count(&self) -> usize {
        fs::read_to_string(self.root.join("submissions"))
            .unwrap_or_default()
            .lines()
            .count()
    }
}

#[test]
fn phonon_chain_single_submission_delivers_save_and_plots_without_downstream() {
    let c = Case::numbered();
    c.chain_setup();
    let out = c.chain_submit(false);
    assert!(String::from_utf8_lossy(&out.stdout).contains("advance automatically within the allocation"));
    success(out);
    assert_eq!(c.submission_count(), 1);
    assert!(!c.chain_submit(false).status.success());
    success(c.chain_execute());
    assert_eq!(c.chain_attempt()["state"], "succeeded");
    let dir = c.root.join("run/4.qe-phonon-nosplit");
    assert!(dir.join("save/scf.dvscf_q1").is_file());
    for file in ["phonon.png","phonon.plot.json"] {
        assert!(c.root.join("run/results/4.qe-phonon-nosplit").join(file).is_file());
    }
    assert_eq!(c.chain_calls("ph"), 1);
    assert_eq!(c.chain_calls("q2r"), 1);
    assert_eq!(c.chain_calls("matdyn"), 2);
    assert!(c.calls().1.is_empty() && c.calls().2.is_empty());
    let out = c.numbered_cli(0, &["--status", "--json"], None, "chain-status");
    let status: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(status["workflow_complete"], false);
    assert_eq!(status["workflow_status"], "incomplete");
    let calls = c.calls();
    success(c.chain_submit(false));
    assert_eq!(c.submission_count(), 1);
    assert_eq!(c.calls(), calls);
    assert!(!c.root.join("fixture-ledger.json").exists());
    success(c.numbered_cli(1, &["--stage", "coarse", "-s"], None, "chain-handoff"));
    assert!(!c.root.join("run/native-plan.json").exists());
    assert_eq!(
        epwflow::foundation::readable_state::read(
            &c.root.join("run"), epwflow::foundation::stages::Step::Coarse,
        ).unwrap().get("Stage allocation", "Status"),
        Some("submitted")
    );
    assert_eq!(c.calls(), calls);
}

#[test]
fn phonon_chain_failure_requires_resume_and_keeps_completed_ph_and_post() {
    for failure in ["q2r", "matdyn"] {
        let c = Case::numbered();
        c.chain_setup();
        c.put("comparison-failure", failure);
        success(c.chain_submit(false));
        assert!(!c.chain_execute().status.success());
        assert_eq!(c.chain_attempt()["state"], "failed");
        assert!(c.root.join("run/4.qe-phonon-nosplit/save/scf.dvscf_q1").exists());
        assert!(!c
            .root
            .join("run/4.qe-phonon-nosplit/completion.json")
            .exists());
        assert!(!c.chain_submit(false).status.success());
        assert_eq!(c.submission_count(), 1);
        fs::remove_file(c.root.join("comparison-failure")).unwrap();
        success(c.chain_submit(true));
        success(c.chain_execute());
        assert_eq!(c.chain_calls("ph"), 1);
        assert_eq!(c.chain_calls("q2r"), if failure == "q2r" { 2 } else { 1 });
        assert_eq!(
            c.chain_calls("matdyn"),
            if failure == "matdyn" { 3 } else { 2 }
        );
        assert_eq!(c.submission_count(), 2);
        assert_eq!(c.chain_attempt()["state"], "succeeded");
    }
}

#[test]
fn phonon_chain_missing_plot_is_rebuilt_without_qe_and_tampered_save_is_rejected() {
    let c = Case::numbered();
    c.chain_setup();
    success(c.chain_submit(false));
    success(c.chain_execute());
    let calls = c.calls();
    fs::remove_file(
        c.root
            .join("run/results/4.qe-phonon-nosplit/phonon.png"),
    )
    .unwrap();
    success(c.chain_submit(false));
    success(c.chain_execute());
    assert_eq!(c.calls(), calls);
    assert_eq!(c.submission_count(), 2);
    c.put("run/4.qe-phonon-nosplit/save/scf.dvscf_q1", "tampered");
    assert!(!c.chain_submit(false).status.success());
    assert_eq!(c.submission_count(), 2);
}

#[test]
fn phonon_chain_unknown_submission_is_not_repeated() {
    let c = Case::numbered();
    c.chain_setup();
    c.script(
        "sbatch",
        "#!/bin/bash\nprintf 'call\\n' >> submissions\nprintf 'unparseable\\n'\n",
    );
    assert!(!c.chain_submit(false).status.success());
    let receipt = c.root.join("run/4.qe-phonon-nosplit/submissions");
    assert_eq!(fs::read_to_string(&receipt).unwrap().lines().count(), 1);
    assert!(!c.chain_submit(true).status.success());
    assert_eq!(fs::read_to_string(receipt).unwrap().lines().count(), 1);
}

#[test]
fn phonon_chain_adopts_verified_native_ph_and_q2r() {
    let c = Case::numbered();
    c.chain_setup();
    c.put("comparison-failure", "matdyn");
    success(c.chain_submit(false));
    assert!(!c.chain_execute().status.success());
    assert_eq!(c.chain_calls("q2r"), 1);
    fs::remove_file(c.root.join("comparison-failure")).unwrap();
    let before = c.submission_count();
    success(c.chain_submit(true));
    success(c.chain_execute());
    assert_eq!(c.chain_calls("ph"), 1);
    assert_eq!(c.chain_calls("q2r"), 1);
    assert_eq!(c.chain_calls("matdyn"), 3);
    assert_eq!(c.submission_count(), before + 1);
    assert!(c.calls().1.is_empty());
}

#[test]
fn phonon_chain_ph_and_save_failures_block_post_and_resume_once() {
    for mode in ["ph_exit", "truncated_dvscf"] {
        let c = Case::numbered();
        c.chain_setup();
        if mode == "ph_exit" {
            c.script("ph.x", "#!/bin/bash\nif [[ -e \"$(dirname \"$0\")/fixture-failure\" ]]; then exit 17; fi\n/usr/bin/python3 \"$(dirname \"$0\")/qe_native_io.py\" ph \"$@\"\n");
        }
        c.put("fixture-failure", mode);
        success(c.chain_submit(false));
        assert!(!c.chain_execute().status.success());
        assert_eq!(c.chain_attempt()["state"], "failed");
        assert_eq!(c.chain_calls("q2r"), 0);
        assert_eq!(c.chain_calls("matdyn"), 0);
        fs::remove_file(c.root.join("fixture-failure")).unwrap();
        success(c.chain_submit(true));
        success(c.chain_execute());
        assert_eq!(c.chain_calls("q2r"), 1);
        assert_eq!(c.chain_calls("matdyn"), 2);
    }
}

#[test]
fn phonon_chain_plot_failure_keeps_all_science_for_one_resume() {
    let c = Case::numbered();
    c.chain_setup();
    let p = c.root.join("qe_native_io.py");
    let body = fs::read_to_string(&p).unwrap().replace("if value('fldos'):", "if value('fldos'):\n            if (ROOT/'plot-failure').exists():\n                (ROOT/'run/results/4.qe-phonon-nosplit/phonon.png').mkdir(parents=True,exist_ok=True)");
    fs::write(p, body).unwrap();
    c.put("plot-failure", "block renderer");
    success(c.chain_submit(false));
    assert!(!c.chain_execute().status.success());
    assert_eq!(c.chain_attempt()["state"], "failed");
    let calls = c.calls();
    assert_eq!(c.chain_calls("q2r"), 1);
    assert_eq!(c.chain_calls("matdyn"), 2);
    fs::remove_file(c.root.join("plot-failure")).unwrap();
    fs::remove_dir(c.root.join("run/results/4.qe-phonon-nosplit/phonon.png")).unwrap();
    success(c.numbered_cli(0,&["--plot","--results",c.root.join("run/results/4.qe-phonon-nosplit").to_str().unwrap()],None,"chain-explicit-plot"));
    success(c.chain_submit(true));
    success(c.chain_execute());
    assert_eq!(c.calls(), calls);
    assert_eq!(c.chain_attempt()["state"], "succeeded");
}

#[test]
fn phonon_chain_fast_worker_completion_is_not_overwritten_by_submit_receipt() {
    let c = Case::numbered();
    c.chain_setup();
    c.script("sbatch", &format!("#!/bin/bash\nprintf 'call\\n' >> {}\nSLURM_JOB_ID=201 SLURM_NTASKS=1 /bin/bash \"${{@: -1}}\" > {} 2>&1\nprintf '201\\n'\n",
        shell_quote(c.root.join("submissions").to_str().unwrap()), shell_quote(c.root.join("race-worker.log").to_str().unwrap())));
    success(c.chain_submit(false));
    assert_eq!(c.chain_attempt()["state"], "succeeded");
    c.put("job-201.state", "COMPLETED");
    success(c.chain_submit(false));
    assert_eq!(c.submission_count(), 1);
}

#[test]
fn phonon_chain_terminal_interruption_is_reported_and_resumed() {
    let c = Case::numbered();
    c.chain_setup();
    c.put("comparison-failure", "matdyn");
    success(c.chain_submit(false));
    assert!(!c.chain_execute().status.success());
    // 模拟调度器中断 worker，终态已经可查而磁盘仍停在 running。
    use epwflow::foundation::{readable_state as logs, stages::Step};
    logs::update(&c.root.join("run"),Step::Phonon,|d|{d.set("Stage allocation","Status","running");Ok(())}).unwrap();
    c.put("job-201.state", "CANCELLED");
    let out = c.numbered_cli(0, &["--status", "--json"], None, "interrupted-status");
    let state: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(state["workflow_status"], "incomplete");
    assert_eq!(state["workflow_complete"], false);
    assert_eq!(c.chain_attempt()["state"], "running", "Query does not change the record");
    assert!(!c.chain_submit(false).status.success());
    fs::remove_file(c.root.join("comparison-failure")).unwrap();
    success(c.chain_submit(true));
    success(c.chain_execute());
    assert_eq!(c.chain_calls("ph"), 1);
    assert_eq!(c.chain_calls("q2r"), 1);
    assert_eq!(c.chain_calls("matdyn"), 3);
    assert_eq!(c.chain_attempt()["state"], "succeeded");
}

#[test]
fn phonon_chain_final_registration_interruption_resumes_without_science() {
    let c = Case::numbered();
    c.chain_setup();
    success(c.chain_submit(false));
    success(c.chain_execute());
    let calls = c.calls();
    // 科学与图表回执已发布，模拟父完成记录尚未落盘时 worker 中断。
    use epwflow::foundation::{readable_state as logs, stages::Step};
    logs::update(&c.root.join("run"),Step::Phonon,|d|{d.set("Stage allocation","Status","failed");Ok(())}).unwrap();
    success(c.chain_submit(true));
    assert_eq!(c.submission_count(), 1, "Science and charts are complete and no more empty jobs are submitted");
    assert_eq!(c.calls(), calls);
    let out = c.numbered_cli(
        0,
        &["--status", "--json"],
        None,
        "final-registration-status",
    );
    let status: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(status["workflow_complete"], false);
}
