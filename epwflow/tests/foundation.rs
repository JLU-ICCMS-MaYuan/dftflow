#[path = "support/cli.rs"]
mod stage_cli;
#[path = "support/native_fixture.rs"]
mod native_fixture;
use std::{collections::BTreeMap, fs, path::PathBuf, process::{Command, Output}, sync::atomic::{AtomicU64, Ordering}};
use epwflow::foundation::{environment, state};
use std::os::unix::fs::PermissionsExt;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Case { root: PathBuf }
impl Case {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("epw-f1-{}-{}", std::process::id(), NEXT.fetch_add(1,Ordering::Relaxed)));
        fs::create_dir(&root).unwrap();
        let c=Self {root};
        c.put("input.toml", include_str!("fixtures/input.toml"));
        c.put("POSCAR", "MgB2\n1\n3 0 0\n-1.5 2.598 0\n0 0 3.5\nMg B\n1 2\nDirect\n0 0 0\n0.3333 0.6667 0.5\n0.6667 0.3333 0.5\n");
        c.put("Mg.upf", "Mg test pseudo"); c.put("B.upf", "B test pseudo");
        for p in ["pw.x","ph.x","epw.x","launcher","sbatch"] { c.script(p,"#!/bin/sh\necho unexpectedly-executed >> calls\nexit 90\n"); }
        c
    }
    fn put(&self, name:&str,text:&str) { fs::write(self.root.join(name),text).unwrap(); }
    fn script(&self,name:&str,text:&str) { self.put(name,text); fs::set_permissions(self.root.join(name),fs::Permissions::from_mode(0o755)).unwrap(); }
    fn cmd(&self, args: &[&str]) -> Command {
        let mut c=stage_cli::command(args, &self.root);
        c.current_dir(&self.root).env("EPWFLOW_LOCAL_STATE",self.root.join("state")).env("XDG_STATE_HOME",self.root.join("state")).env("PATH",format!("{}:/usr/bin:/bin", self.root.display())); c
    }
    fn run(&self,args:&[&str])->Output {self.cmd(args).output().unwrap()}
    fn json(&self,args:&[&str])->serde_json::Value { let o=self.run(args);assert!(o.status.success(),"{}",String::from_utf8_lossy(&o.stderr));serde_json::from_slice(&o.stdout).unwrap() }
    fn edit(&self,from:&str,to:&str) {let p=self.root.join("input.toml");let t=fs::read_to_string(&p).unwrap();assert!(t.contains(from));fs::write(p,t.replace(from,to)).unwrap();}
}
impl Drop for Case {fn drop(&mut self){ fs::remove_dir_all(&self.root).unwrap(); }}
fn fail(o:Output, expected:&str) {assert!(!o.status.success());assert!(String::from_utf8_lossy(&o.stderr).contains(expected),"expected {expected}: {}",String::from_utf8_lossy(&o.stderr));}




#[test]
fn removed_resource_fields_are_rejected_with_the_config_path() {
    let c = Case::new();
    c.edit("[slurm]", "[resources]\npartition = \"test\"\nnodes = 1\nmpi = 1\n\n[slurm]");
    let out = c.run(&["check", "-c", "input.toml"]);
    assert!(!out.status.success());
    let error = String::from_utf8_lossy(&out.stderr);
    assert!(error.contains("input.toml") && error.contains("resources") && error.contains("slurm"), "{error}");
    assert!(!c.root.join("calls").exists());
}

#[test]
fn all_public_programs_remove_time_and_storage_cli_options() {
    for bin in [env!("CARGO_BIN_EXE_epw_phonon"), env!("CARGO_BIN_EXE_epw_wannier"), env!("CARGO_BIN_EXE_epw_superconductivity")] {
        let c = Case::new();
        let help = Command::new(bin).arg("--help").output().unwrap();
        assert!(help.status.success());
        let text = String::from_utf8_lossy(&help.stdout);
        for flag in ["--wall-seconds", "--storage-bytes", "--budget", "--ledger", "--label", "--token", "--job"] {
            assert!(!text.contains(flag));
            let out = Command::new(bin).args([flag, "1"]).current_dir(&c.root).output().unwrap();
            assert!(!out.status.success());
            assert!(String::from_utf8_lossy(&out.stderr).contains(flag));
        }
    }
}


#[test]
fn readonly_check_all_modes_and_cross_directory() {
    for ph in ["nosplit","index","coords"] {for sc in ["iso","aniso","both"] {
        let c=Case::new();let mut cfg:epwflow::foundation::config::Config=toml::from_str(&fs::read_to_string(c.root.join("input.toml")).unwrap()).unwrap();native_fixture::phonon(&mut cfg,serde_json::from_value(serde_json::json!(ph)).unwrap());c.put("input.toml",&toml::to_string_pretty(&cfg).unwrap());c.edit("mode = \"both\"",&format!("mode = \"{sc}\""));
        let a=c.json(&["check","-c","input.toml","--json"]);
        let b=c.cmd(&["check","-c",c.root.join("input.toml").to_str().unwrap(),"--json"]).current_dir("/").output().unwrap();
        assert!(b.status.success()); assert_eq!(a,serde_json::from_slice::<serde_json::Value>(&b.stdout).unwrap());
        assert_eq!(a["environment"]["runtime_version"],"unverified");
        let state=c.root.join("run/epw-state");
        assert_eq!(fs::read_dir(c.root.join("run")).unwrap().count(),1);
        assert_eq!(fs::read_dir(&state).unwrap().count(),2,"Check to save only original and source records");
        assert_eq!(fs::read(state.join("0000-config.toml")).unwrap(),fs::read(c.root.join("input.toml")).unwrap());
        assert!(state.join("1.qe-scf.log").is_file());
        assert!(!epwflow::foundation::layout::enabled(&c.root.join("run")));
        assert!(!c.root.join("state").exists());assert!(!c.root.join("calls").exists());
    }}
}
#[test]
fn rejects_unknown_nonfinite_range_and_incompatible_inputs() {
    for (from,to,field) in [
        ("schema_version = 2","schema_version = 1","schema_version"),
        ("schema_version = 2","bogus = 4\nschema_version = 2","bogus"),
        ("muc = 0.1","muc = nan","muc"),
        ("temps = [ 5.0, 10.0, 15.0,]","temps = [ 0.0, 10.0, 15.0,]","temps"),
        ("temps = [ 5.0, 10.0, 15.0,]","temps = [ 5.001, 10.0, 15.0,]","temps"),
        ("mode = \"both\"","mode = \"wrong\"","mode"),
        ("nkf1 = 8","nkf1 = 7","stages.7.epw-fine.nkf1"),
        ("dis_froz_min = -2.0","dis_froz_min = -20.0","stages.5.wannier90.dis_froz_min"),
        ("[stages.\"1.qe-scf\"]","[stages.\"1.qe-scf\"]\nnoncolin = true","noncolin"),
    ] {
        let c=Case::new();c.edit(from,to);fail(c.run(&["check","-c","input.toml"]),field);assert!(!c.root.join("run").exists());
    }
}
#[test]
fn missing_files_and_literal_paths() {
    let c=Case::new(); let name="structure with spaces $(touch PWNED)";
    fs::rename(c.root.join("POSCAR"),c.root.join(name)).unwrap();c.edit("\"POSCAR\"",&format!("\"{name}\""));
    c.json(&["check","-c","input.toml","--json"]);assert!(!c.root.join("PWNED").exists());
    fs::remove_file(c.root.join("Mg.upf")).unwrap();fail(c.run(&["check","-c","input.toml"]),"pseudos.Mg");
}
#[test]
fn snapshots_idempotent_and_external_input_is_not_reread() {
    let c=Case::new();assert!(c.run(&["prepare","-c","input.toml"]).status.success());
    let snap=c.root.join("run/input-init.toml");let before=fs::read(&snap).unwrap();
    assert!(c.run(&["prepare","-c","input.toml"]).status.success());assert_eq!(before,fs::read(&snap).unwrap());
    c.put("POSCAR","changed original");c.put("input.toml","invalid external config");
    let s=c.json(&["status","-w","run","--json"]);assert_eq!(s["workflow_complete"],false);
    // 旧分配续算仍读取固定快照；新 -s 要求显式迁移编号工作区。
    fail(c.run(&["resume","-w","run","-r"]),"allocation");
    assert_eq!(before,fs::read(&snap).unwrap());
    fs::write(c.root.join("run/POSCAR-init"),"tampered").unwrap();
    fail(c.run(&["resume","-w","run","-r"]),"input_conflict");
}
#[test]
fn refuses_conflicts_and_no_science_success_on_failed_submission() {
    let c=Case::new(); fs::create_dir(c.root.join("run")).unwrap();c.put("run/important", "keep");
    fail(c.run(&["prepare","-c","input.toml"]),"input_conflict"); assert_eq!(fs::read_to_string(c.root.join("run/important")).unwrap(),"keep");
    let c=Case::new();c.edit("schema_version = 2","schema_version = 2\nlayout = 'numbered'");
    fail(c.run(&["submit","-c","input.toml","--stage","scf"]),"scheduler_unknown");
    assert!(!c.root.join("run/calculations").exists(),"Failed to submit cannot run the science phase");
    let cfg = epwflow::foundation::config::Config::load(&c.root.join("input.toml")).unwrap();
    let task = epwflow::foundation::native_jobs::active_task(&cfg).unwrap().unwrap();
    assert_eq!(task.state, "unknown");
    assert!(!cfg.work_dir.join("epw-state/native-active.json").exists());
}
#[test]
fn help_and_execution_modes() {
    let c=Case::new();for cmd in ["check","prepare","probe","run","submit","status","resume"] { assert!(c.run(&[cmd,"--help"]).status.success(),"{cmd}"); }
    fail(c.run(&["probe","-c","input.toml"]),"required");
    fail(c.run(&["probe","-c","input.toml","-s","-r"]),"cannot");
    fail(c.run(&["probe","-c","input.toml","-r"]),"allocation");
}


#[test]
fn stage_state_machine_and_dependency_check() {
    let c = Case::new();
    let work = &c.root;
    let mut a = state::StageAttempt::new("scf", vec![], "h1");
    a.advance(state::StageState::SubmissionPending).unwrap();
    a.advance(state::StageState::Submitted).unwrap();
    a.advance(state::StageState::Running).unwrap();
    a.advance(state::StageState::Validating).unwrap();
    a.advance(state::StageState::Succeeded).unwrap();
    a.save(work).unwrap();
    let mut ok = BTreeMap::new();
    ok.insert("scf".to_string(), state::StageState::Succeeded);
    assert!(state::dependencies_satisfied("nscf", &["scf".to_string()], &ok).is_ok());
    let mut blocked = BTreeMap::new();
    blocked.insert("scf".to_string(), state::StageState::Running);
    assert!(
        state::dependencies_satisfied("nscf", &["scf".to_string()], &blocked).is_err(),
        "Reliance incomplete must be blocked"
    );
}

#[test]
fn environment_evidence_rejects_missing_program() {
    // 未探测的运行版本标为未验证，缺失程序必须返回环境错误而非零退出。
    let r = environment::verify_program(std::path::Path::new("/no/such/program-xyz"));
    assert!(r.is_err(), "Missing program must environment Error");
}


#[test]
fn removed_budget_section_is_rejected_with_the_config_path() {
    let c = Case::new();
    c.edit("[slurm]", "[budget]\ncampaign_id = \"epic-0007\"\nledger = \"campaign/ledger.json\"\n\n[slurm]");
    let out = c.run(&["check", "-c", "input.toml"]);
    assert!(!out.status.success());
    let error = String::from_utf8_lossy(&out.stderr);
    assert!(error.contains("input.toml") && error.contains("budget"), "{error}");
    assert!(!c.root.join("calls").exists());
    assert!(!c.root.join("campaign").exists());
}
