//! #75：公开 CLI、真实 shell、多进程状态及隔离 Slurm 协议替身。
use super::*;
use epwflow::{
    foundation::stages::Step,
    qe_data::phonon_state::{PhononState, TaskStatus},
};

impl Case {
    fn parallel_submission_count(&self) -> usize {
        fs::read_to_string(self.root.join("submissions"))
            .unwrap_or_default()
            .lines()
            .count()
    }
    fn parallel_calls(&self, kind: &str) -> usize {
        self.calls().0.lines().filter(|s| *s == kind).count()
    }
    fn parallel_setup(&self, mode: config::PhMode) {
        self.numbered_change(|cfg| cfg.phonon.mode = mode);
        self.direct_setup();
        self.script(
            "scontrol",
            &format!(
                r#"#!/bin/bash
if [[ "$1" == update ]]; then
    printf '%s\n' "$*" >> {updates}
    exit 0
fi
printf 'JobId=%s JobState=RUNNING NumNodes=1\n' "${{@: -1}}"
"#,
                updates = shell_quote(self.root.join("dependency-updates").to_str().unwrap())
            ),
        );
        success(self.numbered_cli(0, &["--stage", "scf", "-r"], Some(190), "parallel-scf"));
    }
    fn parallel_state_path(&self) -> PathBuf {
        let cfg = config::Config::load(&self.root.join("input.toml")).unwrap();
        epwflow::qe_data::phonon_parallel::state_path(&cfg)
    }
    fn parallel_state(&self) -> PhononState {
        PhononState::load(&self.parallel_state_path()).unwrap()
    }
    fn parallel_execute(&self, task: &str) -> Output {
        let s = self.parallel_state();
        let (path, job) = match task {
            "init" => (s.init.script, s.init.job_id.unwrap()),
            "collect" => (s.collect.script, s.collect.job_id.unwrap()),
            _ => {
                let q = &s.qpoints[task.parse::<usize>().unwrap() - 1];
                (q.script.clone(), q.job_id.unwrap())
            }
        };
        let path = self.parallel_state_path().parent().unwrap().join(path);
        let out = self
            .scheduled_command(&path, path.parent().unwrap(), job)
            .output()
            .unwrap();
        self.put(
            &format!("parallel-{task}.stdout"),
            &String::from_utf8_lossy(&out.stdout),
        );
        self.put(
            &format!("parallel-{task}.stderr"),
            &String::from_utf8_lossy(&out.stderr),
        );
        self.put(
            &format!("job-{job}.state"),
            if out.status.success() {
                "COMPLETED"
            } else {
                "FAILED"
            },
        );
        out
    }
    fn parallel_submit(&self, resume: bool) -> Output {
        self.numbered_cli(
            0,
            if resume { &["-s", "--resume"] } else { &["-s"] },
            None,
            "parallel-submit",
        )
    }
}

#[test]
fn phonon_parallel_public_scripts_do_not_submit_or_execute_science() {
    for mode in [config::PhMode::Index, config::PhMode::Coords] {
        let c = Case::numbered();
        c.numbered_change(|cfg| cfg.phonon.mode = mode);
        success(c.numbered_cli(0, &[], None, "parallel-prepare"));
        let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
        let dir = Step::Phonon.directory(&cfg);
        for path in ["init/ph_init.sh", "q/q0001/ph.sh", "collect/collect.sh"] {
            assert!(Command::new("bash")
                .arg("-n")
                .arg(dir.join(path))
                .status()
                .unwrap()
                .success());
        }
        assert!(c.parallel_state().qpoints.is_empty());
        assert_eq!(c.parallel_submission_count(), 0);
        assert!(c.calls().0.is_empty());
        let state = c.parallel_state_path();
        fs::remove_file(&state).unwrap();
        assert!(!c.parallel_submit(true).status.success());
        assert!(!state.exists());
        assert!(!c
            .numbered_cli(0, &[], None, "parallel-missing-state")
            .status
            .success());
    }
}

#[test]
fn phonon_parallel_resume_cancelled_q_keeps_completed_q_and_collect_job() {
    for mode in [config::PhMode::Index, config::PhMode::Coords] {
        let c = Case::numbered();
        c.parallel_setup(mode);
        success(c.parallel_submit(false));
        assert_eq!(c.parallel_submission_count(), 1);
        success(c.parallel_execute("init"));
        assert_eq!(
            c.parallel_submission_count(),
            4,
            "init expands all q jobs and collect without another public command"
        );
        success(c.parallel_submit(false));
        let s = c.parallel_state();
        assert_eq!(s.qpoints.len(), 2);
        assert_eq!(c.parallel_submission_count(), 4);
        let calls = fs::read_to_string(c.root.join("submissions")).unwrap();
        assert_eq!(
            calls
                .lines()
                .filter(|l| l.contains("--dependency=afterok:201 "))
                .count(),
            2
        );
        assert!(calls.contains("--dependency=afterok:202:203"));
        success(c.parallel_execute("1"));
        c.put("job-203.state", "CANCELLED");
        let before = c.calls().0;
        assert!(!c.parallel_submit(false).status.success());
        // 长作业续算时 init 可能已不在控制器中；不能再引用其旧作业号。
        c.script(
            "sbatch",
            &format!(
                r#"#!/bin/bash
if [[ "$*" == *afterok:201* ]]; then exit 17; fi
printf '%s\n' "$*" >> {calls}
n=$(wc -l < {calls})
printf '%s\n' "$((200+n))"
"#,
                calls = shell_quote(c.root.join("submissions").to_str().unwrap())
            ),
        );
        let lock = c
            .parallel_state_path()
            .with_file_name("phonon-qpoints.json.lock");
        fs::create_dir(&lock).unwrap();
        fs::write(lock.join("owner.json"), serde_json::to_vec(&json!({"host":"fixture","pid":1,"token":"cancelled-q","created_unix":0,"job_id":203})).unwrap()).unwrap();
        success(c.parallel_submit(true));
        assert!(
            !lock.exists(),
            "only the confirmed cancelled allocation's stale state lock is recovered"
        );
        let retried = c.parallel_state();
        assert_eq!(retried.qpoints[0].status, TaskStatus::Completed);
        assert_eq!(retried.qpoints[0].job_id, s.qpoints[0].job_id);
        assert_eq!(retried.qpoints[1].job_id, Some(205));
        assert_eq!(retried.collect.job_id, s.collect.job_id);
        assert_eq!(c.parallel_submission_count(), 5);
        assert_eq!(c.calls().0, before);
        assert!(fs::read_to_string(c.root.join("dependency-updates"))
            .unwrap()
            .contains("Dependency=afterok:205"));
        success(c.parallel_execute("2"));
        success(c.parallel_execute("collect"));
        assert_eq!(c.parallel_state().collect.status, TaskStatus::Completed);
        let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
        assert!(epwflow::qe_data::direct::phonon(&cfg).is_ok());
        assert_eq!(c.parallel_calls("ph"), 3);
        assert_eq!(c.parallel_calls("q2r"), 1);
        assert_eq!(c.parallel_calls("matdyn"), 2);
        assert_eq!(c.parallel_calls("pw"), 1);
        let stage = Step::Phonon.directory(&cfg);
        for relative in ["init/ph_init.sh", "q/q0001/ph.sh", "q/q0002/ph.sh"] {
            assert!(fs::read_to_string(stage.join(relative))
                .unwrap()
                .contains("--phonon-task"));
        }
        for relative in [
            "init/ph_init.err",
            "init/ph_init.exitcode",
            "q/q0001/ph.err",
            "q/q0001/ph.exitcode",
        ] {
            assert!(
                !stage.join(relative).exists(),
                "parallel PH keeps the merged native log contract"
            );
        }
        assert!(cfg
            .work_dir
            .join(format!("results/4.qe-phonon-{}/phonon.png", s.mode))
            .is_file());
    }
}

#[test]
fn phonon_parallel_unknown_receipt_and_corrupt_state_never_resubmit() {
    let c = Case::numbered();
    c.parallel_setup(config::PhMode::Index);
    c.script("sbatch", "#!/bin/bash\nprintf 'not-a-job-id\\n'\n");
    assert!(!c.parallel_submit(false).status.success());
    assert_eq!(c.parallel_state().init.status, TaskStatus::Submitted);
    assert_eq!(c.parallel_state().init.job_id, None);
    assert!(!c.parallel_submit(true).status.success());
    fs::write(c.parallel_state_path(), "broken").unwrap();
    assert!(!c.parallel_submit(true).status.success());
    assert_eq!(
        fs::read_to_string(c.parallel_state_path()).unwrap(),
        "broken"
    );
}

#[test]
fn phonon_parallel_failed_init_only_resubmits_init() {
    let c = Case::numbered();
    c.parallel_setup(config::PhMode::Coords);
    success(c.parallel_submit(false));
    c.put("job-201.state", "CANCELLED");
    success(c.parallel_submit(true));
    assert_eq!(c.parallel_submission_count(), 2);
    let state = c.parallel_state();
    assert_eq!(state.init.job_id, Some(202));
    assert_eq!(state.init.status, TaskStatus::Submitted);
    assert!(state.qpoints.is_empty());
    assert_eq!(state.collect.job_id, None);
}

#[test]
fn phonon_parallel_collect_retry_never_repeats_ph() {
    let c = Case::numbered();
    c.parallel_setup(config::PhMode::Index);
    success(c.parallel_submit(false));
    success(c.parallel_execute("init"));
    success(c.parallel_execute("1"));
    success(c.parallel_execute("2"));
    c.script("q2r.x", "#!/bin/bash\nexit 17\n");
    assert!(!c.parallel_execute("collect").status.success());
    assert_eq!(c.parallel_state().collect.status, TaskStatus::Failed);
    let calls = c.parallel_calls("ph");
    // 原生输入绑定程序路径；脚本复原同一替身，验证后处理局部恢复。
    c.script(
        "q2r.x",
        &format!(
            "#!/bin/sh\nexec /usr/bin/python3 {} q2r \"$@\"\n",
            shell_quote(c.root.join("qe_native_io.py").to_str().unwrap())
        ),
    );
    success(c.parallel_submit(true));
    assert_eq!(c.parallel_submission_count(), 5);
    assert_eq!(c.parallel_state().collect.job_id, Some(205));
    success(c.parallel_execute("collect"));
    assert_eq!(c.parallel_state().collect.status, TaskStatus::Completed);
    assert_eq!(c.parallel_calls("ph"), calls);
}

#[test]
fn phonon_parallel_q_workers_overlap_without_losing_state() {
    let c = Case::numbered();
    c.parallel_setup(config::PhMode::Coords);
    c.script(
        "ph.x",
        &format!(
            r#"#!/usr/bin/python3
import pathlib,subprocess,sys,time
root=pathlib.Path({root})
if pathlib.Path.cwd().name.startswith('q'):
    (root/pathlib.Path.cwd().name).write_text('running')
    end=time.monotonic()+8
    while not all((root/name).exists() for name in ['q0001','q0002']):
        if time.monotonic()>end: sys.exit(17)
        time.sleep(0.02)
sys.exit(subprocess.call(['/usr/bin/python3',str(root/'qe_native_io.py'),'ph',*sys.argv[1:]]))
"#,
            root = serde_json::to_string(c.root.to_str().unwrap()).unwrap()
        ),
    );
    success(c.parallel_submit(false));
    success(c.parallel_execute("init"));
    std::thread::scope(|scope| {
        let first = scope.spawn(|| c.parallel_execute("1"));
        let second = scope.spawn(|| c.parallel_execute("2"));
        success(first.join().unwrap());
        success(second.join().unwrap());
    });
    let state = c.parallel_state();
    assert!(state
        .qpoints
        .iter()
        .all(|q| q.status == TaskStatus::Completed));
    assert_eq!(c.parallel_submission_count(), 4);
    success(c.parallel_execute("collect"));
    assert_eq!(c.parallel_state().collect.status, TaskStatus::Completed);
}

#[test]
#[ignore = "requires the local Slurm partition; scientific programs remain isolated I/O fixtures"]
fn phonon_parallel_real_slurm_cancel_and_update_dependency() {
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            if let Ok(state) = PhononState::load(&self.0) {
                let jobs = state
                    .qpoints
                    .iter()
                    .filter_map(|q| q.job_id)
                    .chain(state.init.job_id)
                    .chain(state.collect.job_id)
                    .map(|j| j.to_string())
                    .collect::<Vec<_>>();
                if !jobs.is_empty() {
                    let _ = Command::new("/usr/bin/scancel")
                        .args(jobs)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status();
                }
            }
        }
    }
    let c = Case::numbered();
    let mode = match std::env::var("DFTFLOW_PHONON_SLURM_MODE").as_deref() {
        Ok("coords") => config::PhMode::Coords,
        Ok("index") | Err(_) => config::PhMode::Index,
        Ok(other) => panic!("unknown Slurm test mode: {other}"),
    };
    c.parallel_setup(mode);
    c.numbered_change(|cfg| {
        cfg.slurm.header =
            Some("#SBATCH --partition=local\n#SBATCH --nodes=1\n#SBATCH --ntasks=1\n".into())
    });
    c.script(
        "ph.x",
        &format!(
            r#"#!/usr/bin/python3
import pathlib,subprocess,sys,time
root=pathlib.Path({root})
if pathlib.Path.cwd().name.startswith('q'): time.sleep(8)
sys.exit(subprocess.call(['/usr/bin/python3',str(root/'qe_native_io.py'),'ph',*sys.argv[1:]]))
"#,
            root = serde_json::to_string(c.root.to_str().unwrap()).unwrap()
        ),
    );
    for command in ["sacct", "scontrol", "sbatch", "squeue"] {
        fs::remove_file(c.root.join(command)).unwrap();
    }
    let _cleanup = Cleanup(c.parallel_state_path());
    success(c.parallel_submit(false));
    let wait = |predicate: &dyn Fn(&PhononState) -> bool| {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(120);
        loop {
            let state = c.parallel_state();
            if predicate(&state) {
                return state;
            }
            assert!(
                std::time::Instant::now() < end,
                "Slurm test timed out: {state:?}"
            );
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    };
    let submitted = wait(&|s| {
        s.qpoints.len() == 2
            && s.qpoints.iter().all(|q| q.job_id.is_some())
            && s.collect.job_id.is_some()
    });
    c.put(
        "real-slurm-before.json",
        &serde_json::to_string_pretty(&submitted).unwrap(),
    );
    let cancelled = submitted.qpoints[1].job_id.unwrap();
    assert!(Command::new("/usr/bin/scancel")
        .arg(cancelled.to_string())
        .status()
        .unwrap()
        .success());
    wait(&|s| s.qpoints[0].status == TaskStatus::Completed);
    let end = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !epwflow::foundation::slurm::accounting(cancelled).is_ok_and(|a| a.terminal) {
        assert!(std::time::Instant::now() < end);
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    success(c.parallel_submit(true));
    let retried = c.parallel_state();
    c.put(
        "real-slurm-resume.json",
        &serde_json::to_string_pretty(&retried).unwrap(),
    );
    assert_ne!(retried.qpoints[1].job_id, Some(cancelled));
    assert_eq!(retried.qpoints[0].job_id, submitted.qpoints[0].job_id);
    assert_eq!(retried.collect.job_id, submitted.collect.job_id);
    let done = wait(&|s| s.collect.status == TaskStatus::Completed);
    assert!(done
        .qpoints
        .iter()
        .all(|q| q.status == TaskStatus::Completed));
    assert_eq!(c.parallel_calls("ph"), 3);
    // 科学完成回执略早于分配退出；确认真实终态后才运行清理守卫。
    let end = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        if let Ok(a) = epwflow::foundation::slurm::accounting(done.collect.job_id.unwrap()) {
            if a.terminal {
                assert_eq!(a.state, "COMPLETED");
                break;
            }
        }
        assert!(
            std::time::Instant::now() < end,
            "collect allocation did not reach a confirmed terminal state"
        );
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}
