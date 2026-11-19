//! #73：实际公开 CLI、分支脚本和跨进程检查点；原生程序为受控替身。
use super::*;
use epwflow::foundation::{readable_state as logs, stages::Step};

impl Case {
    fn electronic_calls(&self) -> String {
        fs::read_to_string(self.root.join("electronic-calls")).unwrap_or_default()
    }
    pub(super) fn electronic_execute_branch(&self, branch: &str) -> Output {
        let root = self.root.join("run");
        let label = format!("Electronic branch · {branch}");
        let doc = logs::read(&root, Step::Electronic).unwrap();
        let script = logs::resolve(&root, doc.get(&label, "Script").unwrap());
        let job = logs::jobs(&root, Step::Electronic).unwrap()[&label];
        let output = self.scheduled_command(&script, script.parent().unwrap(), job).output().unwrap();
        self.put(&format!("{branch}-worker.stdout"), &String::from_utf8_lossy(&output.stdout));
        self.put(&format!("{branch}-worker.stderr"), &String::from_utf8_lossy(&output.stderr));
        self.put(&format!("job-{job}.state"), if output.status.success() { "COMPLETED" } else { "FAILED" });
        output
    }
}

fn case() -> Case {
    let c = Case::numbered();
    c.stage_setup();
    for stage in ["scf", "nscf"] {
        success(c.stage_submit(stage, false));
        success(c.stage_execute());
    }
    for name in ["bands", "dos", "projwfc"] {
        let body = fs::read_to_string(c.root.join(format!("{name}.x"))).unwrap();
        let marker = format!("kind=\"{name}\"");
        assert!(body.contains(&marker));
        c.script(&format!("{name}.x"), &body.replace(&marker, &format!("{marker}\nwith Path(__file__).with_name('electronic-calls').open('a') as trace: trace.write(kind+'\\n')")));
    }
    c
}

#[test]
fn electronic_0073_submits_both_before_execution_and_gates_combined_plot() {
    let c = case();
    let before = c.calls();
    success(c.stage_submit("electronic", false));
    assert_eq!(before, c.calls(), "Submission must not synchronously execute science");
    let root = c.root.join("run");
    let doc = logs::read(&root, Step::Electronic).unwrap();
    let jobs = logs::jobs(&root, Step::Electronic).unwrap();
    let bands = jobs["Electronic branch · qe_eband"];
    let dos = jobs["Electronic branch · qe_eledos"];
    assert_ne!(bands, dos);
    assert_eq!(doc.get("Electronic branch · qe_eband", "Status"), Some("submitted"));
    assert_eq!(doc.get("Electronic branch · qe_eledos", "Status"), Some("submitted"));
    let script = logs::resolve(&root, doc.get("Stage allocation", "Script").unwrap());
    assert!(fs::read_to_string(script).unwrap().contains(&format!("--dependency=afterany:{bands}:{dos}")));
    let duplicate = c.stage_submit("electronic", false);
    assert!(!duplicate.status.success());
    success(c.electronic_execute_branch("qe_eledos"));
    assert!(!root.join("results/3.qe-electronic/electronic.png").exists(), "DOS must not plot incomplete combined data");
    let post = c.electronic_calls();
    assert!(post.find("dos\n").unwrap() < post.find("projwfc\n").unwrap());
    let native = logs::jobs(&root, Step::Electronic).unwrap();
    for (name, job) in native.iter().filter(|(n, _)| n.starts_with("Task · 3.qe-electronic/qe_eledos")) {
        assert_eq!(*job, dos, "{name} must share the DOS branch allocation");
    }
    success(c.electronic_execute_branch("qe_eband"));
    assert!(root.join("results/3.qe-electronic/electronic-band-ranges.png").is_file());
    success(c.stage_execute());
    assert!(root.join("results/3.qe-electronic/electronic.png").is_file());
    let scientific = (c.calls(), c.electronic_calls());
    success(c.stage_submit("electronic", false));
    assert_eq!(scientific, (c.calls(), c.electronic_calls()));
}

#[test]
fn electronic_0073_failed_projection_reuses_bands_and_total_dos() {
    let c = case();
    c.put("fail-pdos", "controlled failure");
    success(c.stage_submit("electronic", false));
    assert!(!c.stage_execute().status.success());
    let root = c.root.join("run");
    let band = fs::read(root.join("3.qe-electronic/qe_eband/eband.out")).unwrap();
    let dos = fs::read(root.join("3.qe-electronic/qe_eledos/scf.tdos")).unwrap();
    assert!(!root.join("results/3.qe-electronic/electronic.png").exists());
    let before = c.electronic_calls();
    fs::remove_file(c.root.join("fail-pdos")).unwrap();
    success(c.stage_submit("electronic", true));
    let doc = logs::read(&root, Step::Electronic).unwrap();
    assert_eq!(doc.get("Electronic branch · qe_eband", "Status"), Some("completed"));
    assert_eq!(doc.get("Electronic branch · qe_eledos", "Status"), Some("submitted"));
    success(c.stage_execute());
    assert_eq!(band, fs::read(root.join("3.qe-electronic/qe_eband/eband.out")).unwrap());
    assert_eq!(dos, fs::read(root.join("3.qe-electronic/qe_eledos/scf.tdos")).unwrap());
    assert_eq!(before.matches("dos\n").count(), c.electronic_calls().matches("dos\n").count());
    assert!(root.join("results/3.qe-electronic/electronic.png").is_file());
}

#[test]
fn electronic_0073_failed_bands_retains_dos_and_early_diagnostic() {
    let c = case();
    let original = fs::read_to_string(c.root.join("bands.x")).unwrap();
    c.script("bands.x", "#!/bin/sh\nexit 17\n");
    success(c.stage_submit("electronic", false));
    assert!(!c.stage_execute().status.success());
    let root = c.root.join("run");
    assert!(root.join("results/3.qe-electronic/electronic-band-ranges.png").is_file());
    assert!(root.join("3.qe-electronic/qe_eledos/scf.tdos").is_file());
    let before = (c.calls(), c.electronic_calls());
    c.script("bands.x", &original);
    success(c.stage_submit("electronic", true));
    let doc = logs::read(&root, Step::Electronic).unwrap();
    assert_eq!(doc.get("Electronic branch · qe_eledos", "Status"), Some("completed"));
    success(c.stage_execute());
    assert_eq!(before.0, c.calls(), "Successful path PW must not rerun");
    assert_eq!(before.1.matches("dos\n").count(), c.electronic_calls().matches("dos\n").count());
    assert_eq!(before.1.matches("projwfc\n").count(), c.electronic_calls().matches("projwfc\n").count());
}

#[test]
fn electronic_0073_unknown_submission_is_not_automatically_retried() {
    let c = case();
    c.script("sbatch", "#!/bin/sh\nprintf 'unparseable receipt\\n'\n");
    assert!(!c.stage_submit("electronic", false).status.success());
    let root = c.root.join("run");
    assert_eq!(logs::read(&root, Step::Electronic).unwrap().get("Electronic branch · qe_eband", "Status"), Some("submission unknown"));
    let retry = c.stage_submit("electronic", true);
    assert!(!retry.status.success());
    assert!(String::from_utf8_lossy(&retry.stderr).contains("scheduler_unknown"));
}

#[test]
fn electronic_0073_workers_overlap_and_merge_their_checkpoints() {
    let c = case();
    // 两个原生程序必须先同时进入，随后才能各自退出；整工作区锁会导致超时失败。
    for (program, marker, peer) in [("pw", "bands-entered", "dos-entered"), ("dos", "dos-entered", "bands-entered")] {
        let path = c.root.join(format!("{program}.x"));
        fs::rename(&path, c.root.join(format!("{program}-original.x"))).unwrap();
        c.script(&format!("{program}.x"), &format!(r#"#!/usr/bin/python3
from pathlib import Path
import os,sys,time
root=Path(__file__).parent
(root/{marker:?}).touch()
deadline=time.monotonic()+15
while not (root/{peer:?}).exists():
    if time.monotonic()>deadline: sys.exit(19)
    time.sleep(0.02)
os.execv(str(root/{original:?}),[str(root/{original:?})]+sys.argv[1:])
"#, original=format!("{program}-original.x")));
    }
    success(c.stage_submit("electronic", false));
    let root = c.root.join("run");
    let doc = logs::read(&root, Step::Electronic).unwrap();
    let jobs = logs::jobs(&root, Step::Electronic).unwrap();
    let mut children = Vec::new();
    for branch in ["qe_eband", "qe_eledos"] {
        let label = format!("Electronic branch · {branch}");
        let script = logs::resolve(&root, doc.get(&label, "Script").unwrap());
        let job = jobs[&label];
        let mut cmd = c.scheduled_command(&script, script.parent().unwrap(), job);
        cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
        children.push((job, cmd.spawn().unwrap()));
    }
    for (job, child) in children {
        success(child.wait_with_output().unwrap());
        c.put(&format!("job-{job}.state"), "COMPLETED");
    }
    let doc = logs::read(&root, Step::Electronic).unwrap();
    for task in ["pw-bands", "bands", "dos", "projwfc"] {
        assert_eq!(doc.get(&format!("Electronic substep · {task}"), "Status"), Some("completed"));
    }
    success(c.stage_execute());
}

#[test]
fn electronic_0073_band_diagnostic_is_available_while_dos_is_pending() {
    let c = case();
    success(c.stage_submit("electronic", false));
    success(c.electronic_execute_branch("qe_eband"));
    let root = c.root.join("run");
    assert!(root.join("results/3.qe-electronic/electronic-band-ranges.png").is_file());
    assert!(!root.join("results/3.qe-electronic/electronic.png").exists());
    assert_eq!(logs::read(&root, Step::Electronic).unwrap().get("Electronic branch · qe_eledos", "Status"), Some("submitted"));
    success(c.stage_execute());
}

#[test]
fn electronic_0073_path_sampling_change_only_rebuilds_band_branch() {
    let c = case();
    success(c.stage_submit("electronic", false));
    success(c.stage_execute());
    let before = c.electronic_calls();
    c.numbered_change(|cfg| cfg.comparison.as_mut().unwrap().points_per_edge = 4);
    success(c.stage_submit("electronic", false));
    let root = c.root.join("run");
    let doc = logs::read(&root, Step::Electronic).unwrap();
    assert_eq!(doc.get("Electronic branch · qe_eband", "Status"), Some("submitted"));
    assert_eq!(doc.get("Electronic branch · qe_eledos", "Status"), Some("completed"));
    success(c.stage_execute());
    assert_eq!(before.matches("dos\n").count(), c.electronic_calls().matches("dos\n").count());
    assert_eq!(before.matches("projwfc\n").count(), c.electronic_calls().matches("projwfc\n").count());
    assert_eq!(before.matches("bands\n").count() + 1, c.electronic_calls().matches("bands\n").count());
}

#[test]
fn electronic_0073_diagnostic_failure_does_not_block_bands() {
    let c = case();
    let root = c.root.join("run");
    fs::create_dir_all(root.join("results/3.qe-electronic/electronic-band-ranges.png")).unwrap();
    success(c.stage_submit("electronic", false));
    success(c.electronic_execute_branch("qe_eband"));
    let doc = logs::read(&root, Step::Electronic).unwrap();
    assert_eq!(doc.get("Electronic substep · bands", "Status"), Some("completed"));
    assert_eq!(c.electronic_calls(), "bands\n");
    assert_eq!(doc.get("Electronic branch · qe_eledos", "Status"), Some("submitted"));
}

#[test]
fn electronic_0073_both_failed_branches_are_resubmitted() {
    let c = case();
    let bands = fs::read_to_string(c.root.join("bands.x")).unwrap();
    let dos = fs::read_to_string(c.root.join("dos.x")).unwrap();
    c.script("bands.x", "#!/bin/sh\nexit 17\n");
    c.script("dos.x", "#!/bin/sh\nexit 18\n");
    success(c.stage_submit("electronic", false));
    assert!(!c.stage_execute().status.success());
    c.script("bands.x", &bands); c.script("dos.x", &dos);
    success(c.stage_submit("electronic", true));
    let doc = logs::read(&c.root.join("run"), Step::Electronic).unwrap();
    for branch in ["qe_eband", "qe_eledos"] {
        assert_eq!(doc.get(&format!("Electronic branch · {branch}"), "Status"), Some("submitted"));
    }
    success(c.stage_execute());
}

#[test]
fn electronic_0073_missing_band_table_repairs_only_bands() {
    let c = case();
    success(c.stage_submit("electronic", false)); success(c.stage_execute());
    let calls = c.calls(); let before = c.electronic_calls();
    fs::remove_file(c.root.join("run/3.qe-electronic/qe_eband/scf_band.gnu")).unwrap();
    success(c.stage_submit("electronic", true)); success(c.stage_execute());
    assert_eq!(calls, c.calls());
    assert_eq!(before.matches("dos\n").count(), c.electronic_calls().matches("dos\n").count());
    assert_eq!(before.matches("bands\n").count() + 1, c.electronic_calls().matches("bands\n").count());
}

#[test]
fn electronic_0073_missing_atomic_projection_repairs_only_projwfc() {
    let c = case();
    success(c.stage_submit("electronic", false)); success(c.stage_execute());
    let calls = c.calls(); let before = c.electronic_calls();
    let dir = c.root.join("run/3.qe-electronic/qe_eledos");
    let atom = fs::read_dir(&dir).unwrap().map(|p| p.unwrap().path())
        .find(|p| p.file_name().unwrap().to_string_lossy().contains("pdos_atm#")).unwrap();
    fs::remove_file(atom).unwrap();
    success(c.stage_submit("electronic", true)); success(c.stage_execute());
    assert_eq!(calls, c.calls());
    assert_eq!(before.matches("dos\n").count(), c.electronic_calls().matches("dos\n").count());
    assert_eq!(before.matches("projwfc\n").count() + 1, c.electronic_calls().matches("projwfc\n").count());
}
