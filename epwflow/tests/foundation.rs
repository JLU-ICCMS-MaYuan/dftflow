use std::{collections::BTreeMap, fs, path::PathBuf, process::{Command, Output}, sync::atomic::{AtomicU64, Ordering}};
use epwflow::foundation::{budget, environment, state, storage};
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
    fn cmd(&self) -> Command {
        let mut c=Command::new(env!("CARGO_BIN_EXE_epwflow"));
        c.current_dir(&self.root).env("XDG_STATE_HOME",self.root.join("state")).env("PATH",format!("{}:/usr/bin:/bin", self.root.display())); c
    }
    fn run(&self,args:&[&str])->Output {self.cmd().args(args).output().unwrap()}
    fn json(&self,args:&[&str])->serde_json::Value { let o=self.run(args);assert!(o.status.success(),"{}",String::from_utf8_lossy(&o.stderr));serde_json::from_slice(&o.stdout).unwrap() }
    fn edit(&self,from:&str,to:&str) {let p=self.root.join("input.toml");let t=fs::read_to_string(&p).unwrap();assert!(t.contains(from));fs::write(p,t.replace(from,to)).unwrap();}
}
impl Drop for Case {fn drop(&mut self){ fs::remove_dir_all(&self.root).unwrap(); }}
fn fail(o:Output, expected:&str) {assert!(!o.status.success());assert!(String::from_utf8_lossy(&o.stderr).contains(expected),"expected {expected}: {}",String::from_utf8_lossy(&o.stderr));}

#[test]
fn readonly_check_all_modes_and_cross_directory() {
    for ph in ["nosplit","index","coords"] {for sc in ["iso","aniso","both"] {
        let c=Case::new();c.edit("[phonon]",&format!("[phonon]\nmode = \"{ph}\""));c.edit("mode = \"both\"",&format!("mode = \"{sc}\""));
        let a=c.json(&["check","-c","input.toml","--json"]);
        let b=c.cmd().current_dir("/").args(["check","-c",c.root.join("input.toml").to_str().unwrap(),"--json"]).output().unwrap();
        assert!(b.status.success()); assert_eq!(a,serde_json::from_slice::<serde_json::Value>(&b.stdout).unwrap());
        assert_eq!(a["environment"]["runtime_version"],"unverified");
        assert!(!c.root.join("run").exists());assert!(!c.root.join("state").exists());assert!(!c.root.join("calls").exists());
    }}
}
#[test]
fn rejects_unknown_nonfinite_range_and_incompatible_inputs() {
    for (from,to,field) in [
        ("schema_version = 1","schema_version = 2","schema_version"),
        ("schema_version = 1","bogus = 4\nschema_version = 1","bogus"),
        ("nodes = 1","nodes = 2","resources.nodes"),
        ("mpi = 1","mpi = 65","resources.mpi"),
        ("mu_star = 0.1","mu_star = nan","superconductivity.mu_star"),
        ("min_k = 1.0","min_k = 0.0","min_k"),
        ("min_k = 1.0","min_k = 1.001","min_k"),
        ("mode = \"both\"","mode = \"wrong\"","mode"),
        ("k_grid = [8,8,4]","k_grid = [7,8,4]","interpolation.k_grid"),
        ("frozen_window_ev = [-2.0,2.0]","frozen_window_ev = [-20.0,2.0]","frozen_window_ev"),
        ("[qe]","[qe]\nnoncolin = true","noncolin"),
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
    fail(c.run(&["resume","-w","run","-s"]),"not_implemented");
    assert_eq!(before,fs::read(&snap).unwrap());
    fs::write(c.root.join("run/POSCAR-init"),"tampered").unwrap();
    fail(c.run(&["resume","-w","run","-s"]),"input_conflict");
}
#[test]
fn refuses_conflicts_and_no_science_success_before_integration() {
    let c=Case::new(); fs::create_dir(c.root.join("run")).unwrap();c.put("run/important", "keep");
    fail(c.run(&["prepare","-c","input.toml"]),"input_conflict"); assert_eq!(fs::read_to_string(c.root.join("run/important")).unwrap(),"keep");
    let c=Case::new();fail(c.run(&["submit","-c","input.toml"]),"not_implemented");assert!(!c.root.join("calls").exists());
}
#[test]
fn help_and_execution_modes() {
    let c=Case::new();for cmd in ["check","prepare","probe","run","submit","status","resume","budget"] { assert!(c.run(&[cmd,"--help"]).status.success(),"{cmd}"); }
    fail(c.run(&["probe","-c","input.toml"]),"required");
    fail(c.run(&["probe","-c","input.toml","-s","-r"]),"cannot");
    fail(c.run(&["probe","-c","input.toml","-r"]),"allocation");
}

#[test]
fn budget_reserve_attach_reconcile_and_deny() {
    let c = Case::new();
    std::env::set_var("EPWFLOW_LOCAL_STATE", &c.root);
    let ledger = c.root.join("ledger.json");
    // 预留在授权上限内（8 节点小时 = 28800 节点秒）。
    let t1 = budget::with_ledger(&ledger, "epic-0007", |l| {
        budget::reserve(l, "stage-a", 1, 1, 3600, 1u64 << 30)
    })
    .expect("reserve 应在上限内成功");
    // 仍在预留态时，超出剩余预算的预留必须拒绝，且不删除既有数据。
    let denied = budget::with_ledger(&ledger, "epic-0007", |l| {
        budget::reserve(l, "stage-b", 1, 1, 25201, 1u64 << 30)
    });
    assert!(denied.is_err(), "超出预算的预留必须 budget_denied");
    // 绑定分配占用一个并发车道。
    budget::with_ledger(&ledger, "epic-0007", |l| budget::attach(l, &t1, Some(11)))
        .expect("attach 应成功");
    // 结算：同一 allocation 仅结算一次，释放车道。
    let settled =
        budget::with_ledger(&ledger, "epic-0007", |l| budget::reconcile(l)).expect("reconcile");
    assert!(settled >= 3600, "结算必须包含已提交任务");
    let st = budget::status_json(&ledger, "epic-0007").expect("status");
    assert_eq!(st["lanes"], 0, "结算后车道归零");
    assert_eq!(st["spent_node_seconds"], 3600, "结算后消耗计入");
}

#[test]
fn storage_monitor_stops_on_limit() {
    let mut m = storage::StorageMonitor::new(1000);
    assert!(m.add(500).is_ok());
    assert!(m.add(600).is_err(), "超出存储上限必须 storage_denied");
    assert!(m.should_stop(), "超限必须请求停止");
    assert_eq!(m.peak(), 1100, "必须记录实际峰值");
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
        "依赖未完成必须阻断"
    );
}

#[test]
fn environment_evidence_rejects_missing_program() {
    // 未探测的运行版本标为未验证，缺失程序必须返回环境错误而非零退出。
    let r = environment::verify_program(std::path::Path::new("/no/such/program-xyz"));
    assert!(r.is_err(), "缺失程序必须 environment 错误");
}
