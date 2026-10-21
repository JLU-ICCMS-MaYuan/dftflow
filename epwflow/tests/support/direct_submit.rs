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
        store::read(&self.root.join("run/epw-state/native-active.json")).unwrap()
    }
    pub(super) fn direct_execute(&self) -> i32 {
        let task = self.direct_task();
        let job = task["job"].as_u64().unwrap();
        let script = PathBuf::from(task["script"]["path"].as_str().unwrap());
        let body = fs::read_to_string(&script).unwrap();
        assert!(!body.contains("--workspace-"));
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
        let mut cmd = Command::new("/bin/bash");
        cmd.arg(&script);
        self.environment(&mut cmd, Some(job));
        // 从别处运行仍应 cd 至原生工作目录。
        cmd.current_dir("/tmp");
        let output = cmd.output().unwrap();
        let dir = PathBuf::from(task["directory"].as_str().unwrap());
        let id = task["id"].as_str().unwrap();
        fs::write(dir.join(format!("native-{id}-{job}.out")), &output.stdout).unwrap();
        fs::write(dir.join(format!("native-{id}-{job}.err")), &output.stderr).unwrap();
        let exit = output.status.code().unwrap();
        self.put(
            &format!("job-{job}.state"),
            if exit == 0 { "COMPLETED" } else { "FAILED" },
        );
        exit
    }
    pub(super) fn direct_advance(&self, bin: usize, args: &[&str], label: &str) -> Output {
        self.numbered_cli(bin, args, None, label)
    }
}

#[test]
fn direct_mpirun_comes_from_job_environment_and_keeps_exit_status() {
    for configured in ["mpirun", "/obsolete MPI/bin/mpirun"] {
        let c = Case::numbered();
        c.direct_setup();
        fs::create_dir(c.root.join("MPI runtime")).unwrap();
        c.script("MPI runtime/mpirun", "#!/bin/bash\n[[ $1 == -np && $2 == 1 ]] || exit 91\nshift 2\n\"$@\"\n");
        c.script("science with spaces.x", "#!/bin/bash\nprintf 'science\\n'\nexit 17\n");
        let env = c.root.join("mpi environment.sh");
        c.put("mpi environment.sh", &format!("export PATH={}:$PATH\n", shell_quote(c.root.join("MPI runtime").to_str().unwrap())));
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
        assert!(body.lines().last().unwrap().starts_with("'mpirun' '-np' '1'"));
        assert!(!body.contains("obsolete MPI"));
        assert!(!task["sources"].as_array().unwrap().iter().any(|id| id["path"].as_str().unwrap().ends_with("/mpirun")));
        assert_eq!(c.direct_execute(), 17);
        assert!(!c.direct_advance(0, &["-s"], "mpi-science-failed").status.success());
    }
}

#[test]
fn direct_native_chain_advances_one_job_per_call_and_completes_both_solvers() {
    let c = Case::numbered();
    c.direct_setup();
    success(c.direct_advance(0, &["--all"], "direct-prepare"));
    assert!(!c.root.join("submissions").exists());
    assert!(c.root.join("run/native-plan.json").exists());
    assert!(!c.root.join("run/submit_all.sh").exists());
    let mut counts = std::collections::BTreeMap::new();
    let mut bin = 0;
    for n in 0..180 {
        let before = fs::read_to_string(c.root.join("submissions"))
            .unwrap_or_default()
            .lines()
            .count();
        let output = c.direct_advance(bin, &["-s"], &format!("direct-{n}"));
        assert!(
            output.status.success(),
            "第 {n} 次：{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let after = fs::read_to_string(c.root.join("submissions"))
            .unwrap_or_default()
            .lines()
            .count();
        assert!(after == before || after == before + 1);
        if String::from_utf8_lossy(&output.stdout).contains("workflow_complete: true") {
            assert_eq!(before, after);
            if bin < 2 {
                bin += 1;
                continue;
            }
            let result: Value =
                store::read(&c.root.join("run/8.epw-superconductivity/manifest.json")).unwrap();
            assert_eq!(result["converged"], true);
            for mode in ["iso", "aniso"] {
                assert_eq!(result["native_modes"][mode]["complete"], true);
                assert!(
                    result["native_modes"][mode]["attempts"]
                        .as_array()
                        .unwrap()
                        .len()
                        > 2
                );
            }
            for name in ["scf", "nscf", "eband", "bands", "dos", "ph"] {
                assert_eq!(counts.get(name), Some(&1usize), "上游不应重复 {name}");
            }
            for name in [
                "scf",
                "nscf",
                "eband",
                "bands",
                "dos",
                "ph",
                "q2r",
                "matdyn_phband",
                "matdyn_phdos",
                "epw",
                "wannier-plot",
                "postw90",
            ] {
                assert!(counts.contains_key(name), "缺少 {name} 执行证据");
            }
            return;
        }
        let task = c.direct_task();
        let stem = task["stem"].as_str().unwrap().to_owned();
        *counts.entry(stem).or_insert(0usize) += 1;
        let blocked = c.direct_advance(bin, &["-s"], "direct-queued");
        assert!(!blocked.status.success());
        assert_eq!(
            after,
            fs::read_to_string(c.root.join("submissions"))
                .unwrap()
                .lines()
                .count()
        );
        assert_eq!(c.direct_execute(), 0, "任务失败：{}", task);
    }
    panic!(
        "没有在有界原生调用次数内完成，证据目录 {}",
        c.root.display()
    );
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
    let id = task["id"].as_str().unwrap();
    let dir = PathBuf::from(task["directory"].as_str().unwrap());
    c.put(&format!("job-{job}.state"), "FAILED");
    fs::write(dir.join(format!("native-{id}-{job}.out")), "native failed").unwrap();
    fs::write(dir.join(format!("native-{id}-{job}.err")), "exit 17").unwrap();
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
    let active = c.root.join("run/epw-state/native-active.json");
    let ledger = c.root.join("fixture-ledger.json");
    let before = (fs::read(&active).unwrap(), fs::read(&ledger).unwrap());
    let out = c.direct_advance(0, &["--status", "--json"], "unknown-status");
    assert!(out.status.success());
    let status: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(status["native_task"]["state"], "unknown");
    assert_eq!(
        before,
        (fs::read(&active).unwrap(), fs::read(&ledger).unwrap())
    );
    assert!(!c
        .direct_advance(0, &["--resume", "-s"], "unknown-retry")
        .status
        .success());
}

#[test]
fn direct_environment_failure_stops_native_command_and_paths_with_spaces_work() {
    let c = Case::numbered();
    c.direct_setup();
    let executable = c.root.join("pw with spaces.x");
    fs::copy(c.root.join("pw.x"), &executable).unwrap();
    let environment = c.root.join("environment with spaces.sh");
    c.put("environment with spaces.sh", "return 17\n");
    c.numbered_change(|cfg| {
        cfg.programs.pw.path = executable;
        cfg.programs.environment_script = Some(environment);
    });
    success(c.direct_advance(0, &["-s"], "environment-submit"));
    let body = fs::read_to_string(c.direct_task()["script"]["path"].as_str().unwrap()).unwrap();
    assert!(body.contains("pw with spaces.x"));
    assert_eq!(c.direct_execute(), 17);
    assert!(!c.root.join("scientific-calls").exists());
    assert!(!c
        .direct_advance(0, &["-s"], "environment-failed")
        .status
        .success());
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
    for n in 0..20 {
        success(c.direct_advance(1, &["--stage", "wannier", "-s"], &format!("dos-next-{n}")));
        let task = c.direct_task();
        let is_dos = task["stem"] == "dos";
        assert_eq!(c.direct_execute(), if is_dos { 17 } else { 0 });
        if !is_dos {
            continue;
        }
        let receipts = c.root.join("run/3.qe-electronic/receipts");
        let pw = fs::read(receipts.join("pw-bands.json")).unwrap();
        let bands = fs::read(receipts.join("bands.json")).unwrap();
        let calls = c.calls();
        assert!(!c
            .direct_advance(1, &["--stage", "wannier", "-s"], "dos-no-resume")
            .status
            .success());
        fs::remove_file(c.root.join("fail-dos")).unwrap();
        success(c.direct_advance(1, &["--stage", "wannier", "--resume", "-s"], "dos-retry"));
        assert_eq!(c.direct_task()["stem"], "dos");
        assert_eq!(calls, c.calls());
        assert_eq!(pw, fs::read(receipts.join("pw-bands.json")).unwrap());
        assert_eq!(bands, fs::read(receipts.join("bands.json")).unwrap());
        assert_eq!(c.direct_execute(), 0);
        success(c.direct_advance(1, &["--stage", "wannier", "-s"], "dos-validated"));
        assert!(receipts.join("dos.json").is_file());
        assert_eq!(calls.0, c.calls().0);
        return;
    }
    panic!("DOS task was not reached");
}

#[test]
fn concurrent_public_commands_submit_only_once_and_reserve_budget_once() {
    let c = Case::numbered();
    c.direct_setup();
    let outputs = std::thread::scope(|scope| {
        let a = scope.spawn(|| c.direct_advance(0, &["-s"], "concurrent-a"));
        let b = scope.spawn(|| c.direct_advance(0, &["-s"], "concurrent-b"));
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert_eq!(outputs.iter().filter(|o| o.status.success()).count(), 1);
    assert_eq!(
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    let ledger: Value = store::read(&c.root.join("fixture-ledger.json")).unwrap();
    assert_eq!(ledger["reservations"].as_array().unwrap().len(), 0);
    assert_eq!(ledger["allocations"].as_array().unwrap().len(), 1);
    assert_eq!(ledger["allocations"][0]["job_id"], 201);
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
    let before = fs::read(c.root.join("run/epw-state/native-active.json")).unwrap();
    let mut cmd = Command::new("sbatch");
    cmd.arg(&script);
    success(c.output(cmd, Some(900), "debug-manual-sbatch"));
    assert_eq!(
        before,
        fs::read(c.root.join("run/epw-state/native-active.json")).unwrap()
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
    let c=Case::numbered();c.direct_setup();
    c.numbered_change(|cfg|{let s=&mut cfg.superconductivity;s.task=config::SuperconductivityTask::Gap;s.mode=config::Mode::Both;s.gap_temperatures_k=vec![20.,30.,40.];s.mixing_beta=-0.3;s.iso=None;s.aniso=None;});
    let stages=[(0,"scf"),(0,"nscf"),(0,"electronic"),(0,"phonon"),(1,"wannier"),(1,"coarse"),(2,"fine"),(2,"superconductivity")];
    let mut step=0;
    let mut finished=false;
    for n in 0..60 {
        let (bin,stage)=stages[step];
        let args=vec!["--stage",stage,"-s"];
        let before=fs::read_to_string(c.root.join("submissions")).unwrap_or_default().lines().count();
        let out=c.direct_advance(bin,&args,&format!("gap-{n}"));
        assert!(out.status.success(),"{}\n{}",String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));
        let after=fs::read_to_string(c.root.join("submissions")).unwrap_or_default().lines().count();
        assert!(after==before || after==before+1);
        if String::from_utf8_lossy(&out.stdout).contains("workflow_complete: true") {
            if step+1<stages.len(){step+=1;continue;}finished=true;break;
        }
        assert_eq!(after,before+1);
        assert!(!c.direct_advance(bin,&args,"gap-active-blocked").status.success());
        assert_eq!(after,fs::read_to_string(c.root.join("submissions")).unwrap().lines().count());
        assert_eq!(c.direct_execute(),0);
    }
    assert!(finished,"未完成公开入口指定温度流程");
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    let root=c.root.join("run/8.epw-superconductivity");
    let run=epwflow::superconductivity::points::published(&cfg.work_dir,&cfg,&store::read::<Value>(&root.join("manifest.json")).unwrap()["run_id"].as_str().unwrap()).unwrap();
    assert!(run.execution_complete&&run.data_complete);assert_eq!(run.attempts.len(),6);
    let calls:Vec<Value>=c.calls().2.lines().map(|l|serde_json::from_str(l).unwrap()).collect();
    assert_eq!(calls.len(),6);
    for mode in ["iso","aniso"]{assert_eq!(calls.iter().filter(|v|v["mode"]==mode).map(|v|v["temperature_k"].as_f64().unwrap()).collect::<Vec<_>>(),[20.,30.,40.]);}
    assert!(calls.iter().all(|v|v["linear"]==false));
    let before=c.calls();success(c.direct_advance(2,&["--stage","superconductivity","-s"],"gap-reuse"));assert_eq!(before,c.calls());
    let results=root.join("results");
    assert!(results.join("points.csv").is_file());assert!(!results.join("tc.csv").exists());assert!(!results.join("figures/tc_interval.svg").exists());
    assert_eq!(epwflow::delivery::check_results(&results).unwrap()["workflow_complete"],true);
    // 有效单初值 aniso 数据须参与分布，结果包可脱离原工作区重绘。
    let (_,data)=epwflow::delivery::native::load(&results).unwrap();
    assert_eq!(epwflow::delivery::native_plot::gap_distributions(&data).unwrap().len(),3);
    let moved=c.root.join("portable");fs::create_dir(&moved).unwrap();
    for f in epwflow::delivery::native::load(&results).unwrap().0.files{fs::copy(results.join(&f.relative_path),moved.join(&f.relative_path)).unwrap();}
    fs::copy(results.join("manifest.json"),moved.join("manifest.json")).unwrap();
    let matrix=&run.attempts[0].record.as_ref().unwrap().path;
    let record:epwflow::superconductivity::attempt::AttemptResult=store::read(matrix).unwrap();
    let id=&record.source.matrix_files[0];let raw=fs::read(&id.path).unwrap();fs::write(&id.path,"damaged matrix").unwrap();
    assert!(epwflow::superconductivity::points::Run::load(&root.join("manifest.json")).is_err());
    epwflow::delivery::native_plot::plot(&moved,None,&["svg".into()]).unwrap();
    fs::write(&id.path,raw).unwrap();
}

#[test]
fn direct_gap_failure_needs_resume_and_freezes_failed_native_evidence() {
    let c=Case::numbered();
    c.numbered_change(|cfg|{let s=&mut cfg.superconductivity;s.task=config::SuperconductivityTask::Gap;s.mode=config::Mode::Iso;s.gap_temperatures_k=vec![20.,30.];s.iso=None;s.aniso=None;});
    c.accounting(Some(201));success(c.numbered_cli(2,&["--stage","fine","-r"],Some(201),"gap-before-failure"));
    c.direct_setup();c.put("job-201.state","COMPLETED");c.put("submissions","allocated preparation\n");
    c.put("gap-fail30","transient failure");
    let args=["--stage","superconductivity","-s"];
    success(c.direct_advance(2,&args,"gap-20-submit"));assert_eq!(c.direct_execute(),0);
    success(c.direct_advance(2,&args,"gap-30-submit"));assert_eq!(c.direct_execute(),17);
    let count=fs::read_to_string(c.root.join("submissions")).unwrap().lines().count();
    assert!(!c.direct_advance(2,&args,"gap-failed-without-resume").status.success());
    assert_eq!(count,fs::read_to_string(c.root.join("submissions")).unwrap().lines().count());
    fs::remove_file(c.root.join("gap-fail30")).unwrap();
    success(c.direct_advance(2,&["--stage","superconductivity","--resume","-s"],"gap-30-resubmit"));assert_eq!(c.direct_execute(),0);
    success(c.direct_advance(2,&args,"gap-30-collect"));
    let run=epwflow::superconductivity::points::Run::load(&c.root.join("run/8.epw-superconductivity/manifest.json")).unwrap();
    assert!(run.execution_complete&&run.data_complete);assert_eq!(run.attempts.len(),3);
    let failed=epwflow::superconductivity::verify::record(&run.attempts[1].record.as_ref().unwrap().path).unwrap();
    assert_eq!(failed.status,"failed");assert_eq!(failed.exit_status,Some(17));assert!(failed.gap.is_none());
    let calls:Vec<Value>=c.calls().2.lines().map(|l|serde_json::from_str(l).unwrap()).collect();assert_eq!(calls.iter().map(|v|v["temperature_k"].as_f64().unwrap()).collect::<Vec<_>>(),[20.,30.,30.]);
}
