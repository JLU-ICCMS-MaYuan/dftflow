//! 0026：公开 CLI、真实 shell 脚本和受控原生替身的手动推进契约。
use super::*;

impl Case {
    pub(super) fn direct_setup(&self) {
        self.script("sacct", &format!(r#"#!/usr/bin/python3
from pathlib import Path
import sys
root=Path({root})
job=sys.argv[sys.argv.index('-j')+1]
state=(root/('job-'+job+'.state')).read_text().strip() if (root/('job-'+job+'.state')).exists() else 'RUNNING'
if '--format=JobIDRaw,ExitCode' in sys.argv:
    print(job+'|'+('0:0' if state=='COMPLETED' else '17:0')+'|')
else:
    print(job+'|'+state+'|1|1|')
"#,root=serde_json::to_string(self.root.to_str().unwrap()).unwrap()));
    }
    pub(super) fn direct_task(&self) -> Value {
        {
            let cfg = config::Config::load(&self.root.join("input.toml")).unwrap();
            serde_json::to_value(
                epwflow::foundation::native_jobs::active_task(&cfg)
                    .unwrap()
                    .unwrap(),
            )
            .unwrap()
        }
    }
    pub(super) fn scheduled_command(
        &self,
        script: &std::path::Path,
        directory: &std::path::Path,
        job: u64,
    ) -> Command {
        let body = fs::read_to_string(script).unwrap();
        assert!(
            !body
                .lines()
                .filter_map(|l| l.trim().strip_prefix("#SBATCH"))
                .flat_map(str::split_whitespace)
                .any(|a| a.starts_with("--time") || a.starts_with("-t")),
            "{}",
            script.display()
        );
        let expected = shell_quote(directory.to_str().unwrap());
        assert_eq!(
            body.lines()
                .find_map(|l| l.strip_prefix("#SBATCH --chdir=")),
            Some(expected.as_str())
        );
        assert!(!body
            .lines()
            .any(|l| l.starts_with("cd ") || l.starts_with("if ! cd ")));
        let mut cmd = Command::new("/bin/bash");
        cmd.arg(script);
        self.environment(&mut cmd, Some(job));
        // 模拟 Slurm 在正文执行前设置 WorkDir；bash 自身不会解释 #SBATCH。
        cmd.current_dir(directory);
        cmd
    }
    pub(super) fn direct_execute(&self) -> i32 {
        self.execute_record(self.direct_task())
    }
    pub(super) fn method_task(&self, mode: &str) -> Value {
        {
            let cfg = config::Config::load(&self.root.join("input.toml")).unwrap();
            serde_json::to_value(
                epwflow::foundation::native_jobs::group_task(&cfg, mode)
                    .unwrap()
                    .unwrap(),
            )
            .unwrap()
        }
    }
    pub(super) fn execute_record(&self, task: Value) -> i32 {
        let job = task["job"].as_u64().unwrap();
        let script = PathBuf::from(task["script"]["path"].as_str().unwrap());
        let body = fs::read_to_string(&script).unwrap();
        assert!(!body.contains("--workspace-"));
        assert!(!body.contains("--batch-finish") && !body.contains("--batch-exit"));
        assert_eq!(
            fs::metadata(&script).unwrap().permissions().mode() & 0o111,
            0o111
        );
        for token in body.split_whitespace() {
            let name = token.trim_matches(['\'', '"']).rsplit('/').next().unwrap();
            assert!(
                !["epw_phonon", "epw_wannier", "epw_superconductivity"].contains(&name),
                "{}",
                script.display()
            );
        }
        for arg in task["command"].as_array().unwrap() {
            let name = std::path::Path::new(arg.as_str().unwrap())
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            assert!(!["epw_phonon", "epw_wannier", "epw_superconductivity"].contains(&name));
        }
        assert!(!body.contains("set -") && !body.contains("|| exit $?"));
        assert!(!body.lines().any(|l| l.starts_with("exec ")));
        assert!(Command::new("/bin/bash")
            .arg("-n")
            .arg(&script)
            .status()
            .unwrap()
            .success());
        let dir = PathBuf::from(task["directory"].as_str().unwrap());
        let output = self.scheduled_command(&script, &dir, job).output().unwrap();
        if task["schema_version"] == 1 {
            let id = task["id"].as_str().unwrap();
            fs::write(dir.join(format!("native-{id}-{job}.out")), &output.stdout).unwrap();
            fs::write(dir.join(format!("native-{id}-{job}.err")), &output.stderr).unwrap();
        } else {
            let log = body
                .lines()
                .find_map(|l| l.strip_prefix("#SBATCH --output="))
                .unwrap();
            let mut combined = output.stdout.clone();
            combined.extend(&output.stderr);
            fs::write(dir.join(log), combined).unwrap();
            assert!(body.contains("#SBATCH --open-mode=truncate"));
            assert!(!dir.join("submit.sh").exists());
            assert_eq!(
                body.lines()
                    .find_map(|l| l.strip_prefix("#SBATCH --error=")),
                Some(log)
            );
        }
        let exit = output.status.code().unwrap();
        self.put(
            &format!("job-{job}.state"),
            if exit == 0 { "COMPLETED" } else { "FAILED" },
        );
        exit
    }
    pub(super) fn direct_advance(&self, bin: usize, args: &[&str], label: &str) -> Output {
        // 默认声子自动链另测；这里保留单任务 SCF 和第 5～8 步原生推进契约。
        let mut args = args.to_vec();
        if bin == 0 && !args.contains(&"--stage") && !args.contains(&"--all") {
            args.extend(["--stage", "scf"]);
        }
        self.numbered_cli(bin, &args, None, label)
    }
}

#[test]
fn direct_mpirun_comes_from_job_environment_and_keeps_exit_status() {
    for configured in ["mpirun", "/obsolete MPI/bin/mpirun"] {
        let c = Case::numbered();
        c.direct_setup();
        fs::create_dir(c.root.join("MPI runtime")).unwrap();
        c.script(
            "MPI runtime/mpirun",
            "#!/bin/bash\n[[ $1 == -np && $2 == 1 ]] || exit 91\nshift 2\n\"$@\"\n",
        );
        c.script(
            "science with spaces.x",
            "#!/bin/bash\nprintf 'science\\n'\nexit 17\n",
        );
        let env = c.root.join("mpi environment.sh");
        c.put(
            "mpi environment.sh",
            &format!(
                "export PATH={}:$PATH\n",
                shell_quote(c.root.join("MPI runtime").to_str().unwrap())
            ),
        );
        c.numbered_change(|cfg| {
            cfg.programs.launcher = vec![configured.into(), "-np".into(), "1".into()];
            cfg.programs.pw.path = c.root.join("science with spaces.x");
            cfg.programs.environment_script = Some(env);
        });
        // 配置读取发生在没有 MPI 的登录环境中；无旧绝对路径存在性要求。
        let loaded = config::Config::load(&c.root.join("input.toml")).unwrap();
        assert_eq!(loaded.programs.launcher[0], configured);
        success(c.direct_advance(0, &["-s"], "mpi-environment-submit"));
        let task = c.direct_task();
        let body = fs::read_to_string(task["script"]["path"].as_str().unwrap()).unwrap();
        assert!(body.lines().last().unwrap().starts_with("mpirun -np 1 "));
        assert!(!body.contains("obsolete MPI"));
        assert!(!task["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id["path"].as_str().unwrap().ends_with("/mpirun")));
        assert_eq!(c.direct_execute(), 17);
        assert!(!c
            .direct_advance(0, &["-s"], "mpi-science-failed")
            .status
            .success());
    }
}

#[test]
fn stage_allocations_complete_upstream_once_then_both_native_solvers() {
    let c = Case::numbered();
    c.numbered_change(|cfg| {cfg.superconductivity.task=config::SuperconductivityTask::Gap;cfg.superconductivity.gap_temperatures_k=vec![20.,30.];});
    c.stage_setup();
    // 所有原生阶段都实际写两路日志，证明归集读取的是执行中的同一 .out。
    let programs = [
        "pw", "ph", "q2r", "matdyn", "bands", "dos", "projwfc", "postw90", "epw",
    ];
    for name in programs {
        let implementation = c.root.join(format!("{name}-implementation"));
        fs::rename(c.root.join(format!("{name}.x")), &implementation).unwrap();
        c.script(&format!("{name}.x"), &format!(
            "#!/bin/bash\nprintf 'merged stdout {name}\\n'\nprintf 'merged stderr {name}\\n' >&2\n{} \"$@\"\n",
            shell_quote(implementation.to_str().unwrap())));
    }
    for (bin, stage) in [(0, "electronic"), (1, "coarse"), (2, "fine")] {
        success(c.numbered_cli(bin, &["--stage", stage, "-s"], None, "stage-chain-submit"));
        success(c.stage_execute());
    }
    success(c.direct_advance(2, &["--stage", "superconductivity", "-s"], "solver-stage"));
    for mode in ["iso","aniso"] {assert_eq!(c.execute_record(c.method_task(mode)),0);}
    success(c.direct_advance(2, &["--stage", "superconductivity", "-s"], "solver-collect"));
    let result: Value =
        store::read(&c.root.join("run/8.epw-superconductivity/manifest.json")).unwrap();
    assert_eq!(result["execution_complete"],true);
    assert_eq!(result["plan"].as_array().unwrap().len(),4);
    assert_eq!(c.calls().0.lines().filter(|s| *s == "pw").count(), 3);
    assert_eq!(c.calls().0.lines().filter(|s| *s == "ph").count(), 1);
    assert_eq!(c.post_calls(), "postw90\n");
    assert!(c
        .root
        .join("run/results/8.epw-superconductivity/native.plot.json")
        .is_file());
    let mut seen = std::collections::BTreeSet::new();
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    for record in epwflow::foundation::native_jobs::tasks(&cfg).unwrap() {
        let task = serde_json::to_value(record).unwrap();
        assert_eq!(task["schema_version"], 3);
        assert_eq!(task["stdout"], task["stderr"]);
        let log = fs::read_to_string(task["stdout"]["path"].as_str().unwrap()).unwrap();
        let name = task["command"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .find_map(|arg| {
                programs
                    .iter()
                    .find(|name| arg.ends_with(&format!("/{name}.x")))
                    .copied()
            })
            .unwrap();
        assert!(log.contains(&format!("merged stdout {name}\n")));
        assert!(log.contains(&format!("merged stderr {name}\n")));
        seen.insert(name);
        let dir = PathBuf::from(task["directory"].as_str().unwrap());
        assert!(!dir
            .join(format!("{}.err", task["stem"].as_str().unwrap()))
            .exists());
        let script = PathBuf::from(task["script"]["path"].as_str().unwrap());
        assert!(fs::read_to_string(&script).unwrap().contains(" 2>&1"));
        assert!(Command::new("/bin/bash")
            .arg("-n")
            .arg(script)
            .status()
            .unwrap()
            .success());
    }
    assert_eq!(seen, programs.into_iter().collect());
}

#[test]
fn direct_failed_scf_needs_resume_and_manual_script_edits_are_not_adopted() {
    let c = Case::numbered();
    c.direct_setup();
    success(c.direct_advance(0, &["-s"], "direct-first"));
    let task = c.direct_task();
    let script = PathBuf::from(task["script"]["path"].as_str().unwrap());
    let original = fs::read(&script).unwrap();
    fs::write(&script, "#!/bin/bash\nexit 0\n").unwrap();
    assert!(!c
        .direct_advance(0, &["-s"], "direct-edited")
        .status
        .success());
    fs::write(&script, original).unwrap();
    let job = task["job"].as_u64().unwrap();
    let dir = PathBuf::from(task["directory"].as_str().unwrap());
    c.put(&format!("job-{job}.state"), "FAILED");
    fs::write(dir.join("scf.out"), "native failed").unwrap();
    fs::write(dir.join("scf.err"), "exit 17").unwrap();
    assert!(!c
        .direct_advance(0, &["-s"], "direct-failed")
        .status
        .success());
    success(c.direct_advance(0, &["--resume", "-s"], "direct-retry"));
    assert_ne!(c.direct_task()["id"], task["id"]);
    assert_eq!(
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}

#[test]
fn explicit_scf_resume_recomputes_success_each_time_and_blocks_running_jobs() {
    let c = Case::numbered();
    c.direct_setup();
    let args = ["--stage", "scf", "-s"];
    let resume = ["--stage", "scf", "--resume", "-s"];
    success(c.direct_advance(0, &args, "force-scf-initial"));
    assert_eq!(c.direct_execute(), 0);
    success(c.direct_advance(0, &args, "force-scf-initial-complete"));
    let dir = c.root.join("run/1.qe-scf");
    for iteration in 0..2 {
        let previous = c.direct_task();
        let previous_generation = previous["generation"].clone();
        let old_script = format!("#!/bin/bash\n# old script {iteration}\n");
        fs::write(dir.join("scf.sh"), &old_script).unwrap();
        success(c.direct_advance(0, &resume, &format!("force-scf-restart-{iteration}")));
        let next = c.direct_task();
        assert_ne!(next["generation"], previous_generation);
        assert_ne!(next["job"], previous["job"]);
        assert!(!dir.join("snapshot").exists());
        assert!(!c.root.join("run/epw-state/history").exists());
        let script = fs::read_to_string(dir.join("scf.sh")).unwrap();
        assert!(script.contains(" > scf.out 2>&1"));
        assert!(!c
            .direct_advance(0, &resume, "force-scf-running-rejected")
            .status
            .success());
        assert_eq!(c.direct_task()["id"], next["id"]);
        assert_eq!(fs::read_to_string(dir.join("scf.sh")).unwrap(), script);
        assert_eq!(c.direct_execute(), 0);
        // 普通 -s 负责归集；成功后的再次普通 -s 不重复提交。
        success(c.direct_advance(0, &args, "force-scf-collect"));
        success(c.direct_advance(0, &args, "force-scf-reuse"));
        let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
        assert!(epwflow::foundation::stages::reusable(&cfg, epwflow::foundation::stages::Step::Scf).unwrap());
        assert!(dir.join("out").is_dir());
        assert!(!dir.join("scf.err").exists());
    }
    assert_eq!(
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count(),
        3
    );
}

#[test]
fn direct_status_is_read_only_and_unknown_receipt_never_resubmits() {
    let c = Case::numbered();
    c.direct_setup();
    c.script(
        "sbatch",
        "#!/bin/bash\nprintf 'warning: ambiguous receipt\\n'\n",
    );
    assert!(!c
        .direct_advance(0, &["-s"], "unknown-submit")
        .status
        .success());
    let active = c.root.join("run/epw-state/1.qe-scf.log");
    let before = fs::read(&active).unwrap();
    let out = c.direct_advance(0, &["--status", "--json"], "unknown-status");
    assert!(out.status.success());
    let status: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(status["native_task"]["state"], "unknown");
    assert_eq!(
        before,
        fs::read(&active).unwrap()
    );
    assert!(!c
        .direct_advance(0, &["--resume", "-s"], "unknown-retry")
        .status
        .success());
}

#[test]
fn direct_source_is_unconditional_and_paths_with_spaces_work() {
    let c = Case::numbered();
    c.direct_setup();
    let executable = c.root.join("pw with spaces.x");
    fs::copy(c.root.join("pw.x"), &executable).unwrap();
    let environment = c.root.join("environment with spaces.sh");
    c.put(
        "environment with spaces.sh",
        "printf 'environment loaded\\n'\nreturn 17\n",
    );
    c.numbered_change(|cfg| {
        cfg.programs.pw.path = executable;
        cfg.programs.environment_script = Some(environment);
    });
    success(c.direct_advance(0, &["-s"], "environment-submit"));
    let body = fs::read_to_string(c.direct_task()["script"]["path"].as_str().unwrap()).unwrap();
    assert!(body.contains("pw with spaces.x"));
    assert!(body.contains("\nsource ") && !body.contains("if source"));
    assert_eq!(c.direct_execute(), 0);
    assert!(c.root.join("scientific-calls").exists());
    success(c.direct_advance(0, &["-s"], "environment-next"));
}

pub(super) fn dos_failure_preserves_verified_pw_and_bands() {
    let c = Case::numbered();
    c.direct_setup();
    let dos = c.root.join("dos.x");
    let original = fs::read_to_string(&dos).unwrap();
    c.script(
        "dos.x",
        &original.replace(
            "kind=\"dos\"",
            "kind=\"dos\"\nif (Path(__file__).parent/'fail-dos').exists(): sys.exit(17)",
        ),
    );
    c.put("fail-dos", "controlled failure");
    c.stage_setup();
    success(c.stage_submit("wannier", false));
    assert!(!c.stage_execute().status.success());
    let receipts = c.root.join("run/3.qe-electronic/qe_eband");
    let pw = fs::read(receipts.join("eband.out")).unwrap();
    let bands = fs::read(receipts.join("scf_band.gnu")).unwrap();
    let calls = c.calls();
    assert!(c
        .root
        .join("run/results/3.qe-electronic/electronic-band-ranges.png")
        .is_file());
    assert!(!c.stage_submit("wannier", false).status.success());
    fs::remove_file(c.root.join("fail-dos")).unwrap();
    success(c.stage_submit("wannier", true));
    success(c.stage_execute());
    assert_eq!(pw, fs::read(receipts.join("eband.out")).unwrap());
    assert_eq!(bands, fs::read(receipts.join("scf_band.gnu")).unwrap());
    assert_eq!(calls.0, c.calls().0, "Success PW and PH Do not run again");
    assert!(c.root.join("run/3.qe-electronic/qe_eledos/scf.tdos").is_file());
    assert!(c.root.join("run/results/3.qe-electronic/electronic.png").is_file());
}

#[test]
fn sequential_public_commands_submit_only_once_without_a_ledger() {
    let c = Case::numbered();
    c.direct_setup();
    success(c.direct_advance(0, &["-s"], "sequential-a"));
    let again = c.direct_advance(0, &["-s"], "sequential-b");
    assert!(!again.status.success());
    assert!(String::from_utf8_lossy(&again.stderr).contains("remains RUNNING"));
    assert_eq!(
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert!(!c.root.join("fixture-ledger.json").exists());
}

#[test]
fn zero_exit_with_invalid_native_output_stops_before_next_submission() {
    let c = Case::numbered();
    c.direct_setup();
    c.script(
        "pw.x",
        "#!/bin/sh\nprintf 'invalid scientific output\\n'\nexit 0\n",
    );
    success(c.direct_advance(0, &["-s"], "invalid-submit"));
    assert_eq!(c.direct_execute(), 0);
    let status = c.direct_advance(0, &["--status", "--json"], "invalid-before-validation");
    assert!(status.status.success());
    let status: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status["native_task"]["state"], "awaiting_validation");
    assert!(!c
        .direct_advance(0, &["-s"], "invalid-collect")
        .status
        .success());
    assert_eq!(c.direct_task()["state"], "failed");
    assert_eq!(
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert!(!c.root.join("run/1.qe-scf/completion.json").exists());
}

#[test]
fn edited_script_can_be_submitted_for_independent_debug_without_formal_adoption() {
    let c = Case::numbered();
    c.direct_setup();
    success(c.direct_advance(0, &[], "debug-prepare"));
    let task = c.direct_task();
    let script = PathBuf::from(task["script"]["path"].as_str().unwrap());
    let mut body = fs::read_to_string(&script).unwrap();
    body.push_str("\n# independent debug\n");
    fs::write(&script, body).unwrap();
    c.script("sbatch", "#!/bin/bash\n/bin/bash \"$1\"\n");
    let before = fs::read(c.root.join("run/epw-state/1.qe-scf.log")).unwrap();
    let mut cmd = Command::new("sbatch");
    cmd.arg(&script);
    success(c.output(cmd, Some(900), "debug-manual-sbatch"));
    assert_eq!(
        before,
        fs::read(c.root.join("run/epw-state/1.qe-scf.log")).unwrap()
    );
    assert!(!c.root.join("fixture-ledger.json").exists());
    assert!(!c.root.join("run/1.qe-scf/completion.json").exists());
    assert!(!c
        .direct_advance(0, &["-s"], "debug-no-adoption")
        .status
        .success());
}

#[test]
fn direct_gap_points_run_exactly_once_without_search_and_reject_damage() {
    let c = Case::numbered();
    for relative in [
        "epw-state/control.lock",
        "epw-state/execution.lock",
        "8.epw-superconductivity/execution.lock",
    ] {
        let lock = c.root.join("run").join(relative);
        fs::create_dir_all(&lock).unwrap();
        fs::write(lock.join("owner.json"), "stale owner").unwrap();
    }
    c.numbered_change(|cfg| {
        let s = &mut cfg.superconductivity;
        s.task = config::SuperconductivityTask::Gap;
        s.mode = config::Mode::Both;
        s.gap_temperatures_k = vec![20., 30., 40.];
        s.mixing_beta = -0.3;
        s.iso = None;
        s.aniso = None;
    });
    c.accounting(Some(201));
    success(c.numbered_cli(2, &["--stage", "fine", "-r"], Some(201), "gap-upstream"));
    c.direct_setup();
    c.put("job-201.state", "COMPLETED");
    c.put("submissions", "allocated preparation\n");
    let args = ["--stage", "superconductivity", "-s"];
    success(c.direct_advance(2, &args, "gap-both-submit"));
    assert_eq!(
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count(),
        3
    );
    success(c.direct_advance(2, &args, "gap-active-no-duplicate"));
    assert_eq!(
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count(),
        3
    );
    let iso = c.method_task("iso");
    let aniso = c.method_task("aniso");
    assert_ne!(iso["job"], aniso["job"]);
    assert_eq!(c.execute_record(aniso), 0);
    assert!(!c
        .root
        .join("run/results/8.epw-superconductivity/native.plot.json")
        .exists());
    assert_eq!(c.execute_record(iso), 0);
    // 原生作业只计算；公开命令归集核验后才发布。
    assert!(!c
        .root
        .join("run/results/8.epw-superconductivity/native.plot.json")
        .exists());
    success(c.direct_advance(2, &args, "gap-both-collect"));
    assert!(c
        .root
        .join("run/results/8.epw-superconductivity/native.plot.json")
        .is_file());
    epwflow::foundation::stage_results::check(&c.root.join("run/results")).unwrap();
    let launches: Vec<Value> = fs::read_to_string(c.root.join("epw-invocations.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(launches.len(), 2);
    for call in launches {
        assert_eq!(call["temperatures"], json!([20., 30., 40.]));
    }
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let root = c.root.join("run/8.epw-superconductivity");
    let run = epwflow::superconductivity::points::published(
        &cfg.work_dir,
        &cfg,
        &store::read::<Value>(&root.join("manifest.json")).unwrap()["run_id"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert!(run.execution_complete && run.data_complete);
    assert_eq!(run.attempts.len(), 6);
    let calls: Vec<Value> = c
        .calls()
        .2
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(calls.len(), 6);
    for mode in ["iso", "aniso"] {
        assert_eq!(
            calls
                .iter()
                .filter(|v| v["mode"] == mode)
                .map(|v| v["temperature_k"].as_f64().unwrap())
                .collect::<Vec<_>>(),
            [20., 30., 40.]
        );
    }
    assert!(calls.iter().all(|v| v["linear"] == false));
    let before = c.calls();
    success(c.direct_advance(2, &["--stage", "superconductivity", "-s"], "gap-reuse"));
    assert_eq!(before, c.calls());
    let results = c.root.join("run/results/8.epw-superconductivity");
    assert!(results.join("native.plot.json").is_file());
    assert!(!results.join("tc.csv").exists());
    assert!(!results.join("figures/tc_interval.svg").exists());
    assert_eq!(
        epwflow::delivery::check_results(&results).unwrap()["workflow_complete"],
        true
    );
    // 有效单初值 aniso 数据须参与分布，结果包可脱离原工作区重绘。
    let (_, data) = epwflow::delivery::native::load(&results).unwrap();
    assert_eq!(
        epwflow::delivery::native_plot::gap_distributions(&data)
            .unwrap()
            .len(),
        3
    );
    let moved = c.root.join("portable");
    fs::create_dir(&moved).unwrap();
    for f in epwflow::delivery::native::load(&results).unwrap().0.files {
        fs::copy(results.join(&f.relative_path), moved.join(&f.relative_path)).unwrap();
    }
    assert!(!results.join("manifest.json").exists());
    let matrix = &run.attempts[0].record.as_ref().unwrap().path;
    let record: epwflow::superconductivity::attempt::AttemptResult = store::read(matrix).unwrap();
    let id = &record.source.matrix_files[0];
    let raw = fs::read(&id.path).unwrap();
    fs::write(&id.path, "damaged matrix").unwrap();
    assert!(epwflow::superconductivity::points::Run::load(&root.join("manifest.json")).is_err());
    epwflow::delivery::native_plot::plot(&moved, None, &["svg".into()]).unwrap();
    fs::write(&id.path, raw).unwrap();
}

#[test]
fn direct_gap_failure_needs_resume_and_freezes_failed_native_evidence() {
    let c = Case::numbered();
    c.numbered_change(|cfg| {
        let s = &mut cfg.superconductivity;
        s.task = config::SuperconductivityTask::Gap;
        s.mode = config::Mode::Iso;
        s.gap_temperatures_k = vec![20., 30.];
        s.iso = None;
        s.aniso = None;
    });
    c.accounting(Some(201));
    success(c.numbered_cli(
        2,
        &["--stage", "fine", "-r"],
        Some(201),
        "gap-before-failure",
    ));
    c.direct_setup();
    c.put("job-201.state", "COMPLETED");
    c.put("submissions", "allocated preparation\n");
    c.put("gap-fail30", "transient failure");
    let args = ["--stage", "superconductivity", "-s"];
    success(c.direct_advance(2, &args, "gap-batch-submit"));
    assert_eq!(c.execute_record(c.method_task("iso")), 17);
    let count = fs::read_to_string(c.root.join("submissions"))
        .unwrap()
        .lines()
        .count();
    assert!(!c
        .direct_advance(2, &args, "gap-failed-without-resume")
        .status
        .success());
    assert_eq!(
        count,
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count()
    );
    fs::remove_file(c.root.join("gap-fail30")).unwrap();
    success(c.direct_advance(
        2,
        &["--stage", "superconductivity", "--resume", "-s"],
        "gap-30-resubmit",
    ));
    assert_eq!(c.execute_record(c.method_task("iso")), 0);
    success(c.direct_advance(2, &args, "gap-30-collect"));
    let run = epwflow::superconductivity::points::Run::load(
        &c.root.join("run/8.epw-superconductivity/manifest.json"),
    )
    .unwrap();
    assert!(run.execution_complete && run.data_complete);
    assert_eq!(run.attempts.len(), 2, "Save only current temperature results, failed to try to overwrite directly");
    assert!(run.attempts.iter().all(|a|a.status=="valid"));
    let calls: Vec<Value> = c
        .calls()
        .2
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        calls
            .iter()
            .map(|v| v["temperature_k"].as_f64().unwrap())
            .collect::<Vec<_>>(),
        [20., 30., 30.]
    );
}

#[test]
fn workspace_residual_locks_do_not_block_public_submission() {
    for owner in [false, true] {
        let c = Case::numbered();
        c.direct_setup();
        for name in [
            "epw-state/control.lock",
            "epw-state/execution.lock",
            "8.epw-superconductivity/execution.lock",
        ] {
            let lock = c.root.join("run").join(name);
            fs::create_dir_all(&lock).unwrap();
            if owner {
                fs::write(lock.join("owner.json"), "stale owner").unwrap();
            }
        }
        success(c.direct_advance(0, &["--stage", "scf", "-s"], "old-lock-submit"));
        let again = c.direct_advance(0, &["--stage", "scf", "-s"], "old-lock-repeat");
        assert!(!again.status.success());
        let error = String::from_utf8_lossy(&again.stderr);
        assert!(error.contains("RUNNING"), "{error}");
        assert!(!error.contains("Lock"), "{error}");
        assert_eq!(c.direct_execute(), 0);
        success(c.direct_advance(0, &["--stage", "scf", "-s"], "old-lock-complete"));
        assert_eq!(
            fs::read_to_string(c.root.join("submissions"))
                .unwrap()
                .lines()
                .count(),
            1
        );
        assert!(!c.root.join("run/epw-state/control.lock").exists());
        assert!(!c.root.join("run/epw-state/execution.lock").exists());

    }
}

fn wait_for_file(path: &std::path::Path) {
    let start = std::time::Instant::now();
    while !path.exists() {
        assert!(
            start.elapsed() < std::time::Duration::from_secs(15),
            "Waiting for timeout:{}",
            path.display()
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn interrupted_public_submit_reenters_receipt_checks_without_workspace_lock() {
    let c = Case::numbered();
    c.direct_setup();
    c.script(
        "sbatch",
        &format!(
            "#!/bin/bash\ntouch {}\nwhile [[ ! -e {} ]]; do sleep 0.02; done\nprintf '201\\n'\n",
            shell_quote(c.root.join("inside-sbatch").to_str().unwrap()),
            shell_quote(c.root.join("release-sbatch").to_str().unwrap())
        ),
    );
    let mut cmd = Command::new(BINS[0]);
    cmd.args(["-c", "input.toml", "--stage", "scf", "-s"]);
    c.environment(&mut cmd, None);
    let mut child = cmd
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    wait_for_file(&c.root.join("inside-sbatch"));
    child.kill().unwrap();
    child.wait().unwrap();
    c.put("release-sbatch", "release orphan fixture");
    let again = c.direct_advance(0, &["--stage", "scf", "-s"], "interrupted-repeat");
    assert!(!again.status.success());
    let error = String::from_utf8_lossy(&again.stderr);
    assert!(error.contains("submission receipt unknown"), "{error}");
    assert!(!error.contains("Lock"), "{error}");
    assert_eq!(c.direct_task()["state"], "submission_pending");
    assert!(!c.root.join("run/epw-state/control.lock").exists());
    assert!(!c.root.join("run/epw-state/execution.lock").exists());
}

#[test]
fn scientific_stdout_is_live_and_arguments_are_quoted_without_shell_expansion() {
    let c = Case::numbered();
    c.direct_setup();
    let program = "science ' \" $HOME `touch injected`.x";
    c.script(program, &format!("#!/bin/bash\nprintf 'live science\\n'\nprintf 'science stderr\\n' >&2\ntouch {}\nwhile [[ ! -e {} ]]; do sleep 0.02; done\nexit 17\n",
        shell_quote(c.root.join("science-started").to_str().unwrap()),
        shell_quote(c.root.join("release-science").to_str().unwrap())));
    c.numbered_change(|cfg| {
        cfg.programs.pw.path = c.root.join(program);
    });
    success(c.direct_advance(0, &["--stage", "scf", "-s"], "live-submit"));
    let task = c.direct_task();
    let script = PathBuf::from(task["script"]["path"].as_str().unwrap());
    assert_eq!(script.file_name().unwrap(), "scf.sh");
    let body = fs::read_to_string(&script).unwrap();
    assert!(body.contains(" < ") && body.contains(" > scf.out 2>&1"));
    assert!(!body.contains("-inp"));
    let dir = PathBuf::from(task["directory"].as_str().unwrap());
    let mut cmd = c.scheduled_command(&script, &dir, 201);
    let mut child = cmd.spawn().unwrap();
    wait_for_file(&c.root.join("science-started"));
    assert!(child.try_wait().unwrap().is_none());
    assert_eq!(
        fs::read_to_string(dir.join("scf.out")).unwrap(),
        "live science\nscience stderr\n"
    );
    assert!(!dir.join("scf.err").exists());
    c.put("release-science", "done");
    assert_eq!(child.wait().unwrap().code(), Some(17));
    assert!(!dir.join("injected").exists());
}

#[test]
fn completed_task_reuses_science_after_fixed_script_and_slurm_log_are_overwritten() {
    let c = Case::numbered();
    c.direct_setup();
    let args = ["--stage", "scf", "-s"];
    success(c.direct_advance(0, &args, "fixed-submit"));
    let task = c.direct_task();
    assert_eq!(c.direct_execute(), 0);
    success(c.direct_advance(0, &args, "fixed-complete"));
    let script = PathBuf::from(task["script"]["path"].as_str().unwrap());
    fs::write(&script, "#!/bin/bash\n# next use of fixed filename\n").unwrap();
    fs::write(script.parent().unwrap().join("slurm-scf.log"), "next log").unwrap();
    success(c.direct_advance(0, &args, "fixed-reuse"));
    assert_eq!(
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    fs::write(
        script.parent().unwrap().join("scf.out"),
        "damaged scientific output",
    )
    .unwrap();
    assert!(!c
        .direct_advance(0, &args, "fixed-reject-damage")
        .status
        .success());
}


#[test]
fn obsolete_budget_files_and_locks_do_not_affect_submission_or_completion() {
    let c = Case::numbered();
    c.direct_setup();
    fs::create_dir_all(c.root.join("campaign")).unwrap();
    fs::create_dir_all(c.root.join("state")).unwrap();
    c.put("campaign/ledger.json", "invalid historical ledger");
    c.put("fixture-ledger.json", r#"{"lanes":2,"allocations":[{"job_id":1},{"job_id":2}]}"#);
    c.put("state/campaigns.json", "invalid historical registry");
    let legacy = c.root.join("fixture-ledger.json");
    let before = fs::read(&legacy).unwrap();
    let _guard = store::Lock::acquire(&legacy).unwrap();
    success(c.direct_advance(0, &["-s"], "obsolete-budget-submit"));
    assert_eq!(c.direct_execute(), 0);
    success(c.direct_advance(0, &["-s"], "obsolete-budget-complete"));
    assert_eq!(fs::read(&legacy).unwrap(), before);
    assert_eq!(fs::read_to_string(c.root.join("campaign/ledger.json")).unwrap(), "invalid historical ledger");
    assert_eq!(fs::read_to_string(c.root.join("state/campaigns.json")).unwrap(), "invalid historical registry");
    assert_eq!(fs::read_to_string(c.root.join("submissions")).unwrap().lines().count(), 1);
}

#[test]
fn three_independent_workspaces_submit_with_one_shared_state_directory() {
    let cases = [Case::numbered(), Case::numbered(), Case::numbered()];
    let shared = cases[0].root.join("shared-state");
    fs::create_dir_all(&shared).unwrap();
    fs::write(shared.join("campaigns.json"), "invalid obsolete registry").unwrap();
    for c in &cases {
        c.direct_setup();
        let mut cmd = Command::new(BINS[0]);
        c.environment(&mut cmd, None);
        cmd.env("EPWFLOW_LOCAL_STATE", &shared).env("XDG_STATE_HOME", &shared)
            .args(["-c", "input.toml", "--stage", "scf", "-s"]);
        success(cmd.output().unwrap());
        assert_eq!(fs::read_to_string(c.root.join("submissions")).unwrap().lines().count(), 1);
        assert!(!c.root.join("campaign").exists());
        assert!(!c.root.join("fixture-ledger.json").exists());
    }
    assert_eq!(fs::read_to_string(shared.join("campaigns.json")).unwrap(), "invalid obsolete registry");
}
