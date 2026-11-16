//! 0057：通过公开 CLI 操作真实临时文件；不调用科学程序、不提交作业。
#![cfg(unix)]
use epwflow::foundation::store;
use serde_json::json;
use std::{
    fs,
    os::unix::fs::{symlink, MetadataExt, PermissionsExt},
    path::PathBuf,
    process::{Command, Output},
};

const HISTORIES: [&str; 3] = [
    "epw-state/history",
    "epw-state/native-attempts/history",
    "epw-state/native-jobs/history",
];
struct Case {
    root: PathBuf,
    work: PathBuf,
}
impl Case {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("epw clear {}", store::token()));
        let work = root.join("run with spaces");
        fs::create_dir_all(work.join("epw-state")).unwrap();
        fs::create_dir_all(root.join("commands")).unwrap();
        fs::write(
            work.join("epw-state/layout.json"),
            br#"{"schema_version":1,"phonon":"nosplit"}"#,
        )
        .unwrap();
        let c = Self { root, work };
        c.script("squeue", "#!/bin/sh\nexit 0\n");
        c.script("sacct", "#!/bin/sh\nprintf '99|COMPLETED|1|1|\\n'\n");
        c.put("1.qe-scf/scf.out", "live science");
        c.put("results/result.json", "{\"result\":42}");
        c.put(
            "epw-state/native-jobs/live.json",
            "{\"state\":\"validated\",\"job\":99}",
        );
        fs::write(c.root.join("ledger.json"), b"keep budget").unwrap();
        c
    }
    fn put(&self, name: &str, text: &str) {
        let path = self.work.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn script(&self, name: &str, body: &str) {
        let path = self.root.join("commands").join(name);
        fs::write(&path, body).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fn histories(&self) {
        for name in HISTORIES {
            self.put(&format!("{name}/old/input.out"), "old data");
            // 历史内部损坏的 JSON 不参与现用记录检查。
            self.put(&format!("{name}/old/broken.json"), "old invalid json");
        }
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_epw_clear"));
        command
            .current_dir(&self.work)
            .env("PATH", self.root.join("commands"));
        command
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }
    fn assert_preserved(&self) {
        assert_eq!(
            fs::read(self.work.join("1.qe-scf/scf.out")).unwrap(),
            b"live science"
        );
        assert_eq!(
            fs::read(self.work.join("results/result.json")).unwrap(),
            b"{\"result\":42}"
        );
        assert_eq!(
            fs::read(self.root.join("ledger.json")).unwrap(),
            b"keep budget"
        );
        assert_eq!(
            fs::read(self.work.join("epw-state/native-jobs/live.json")).unwrap(),
            b"{\"state\":\"validated\",\"job\":99}"
        );
    }
    fn assert_all_histories_present(&self) {
        for name in HISTORIES {
            assert!(self.work.join(name).join("old/input.out").is_file());
        }
        self.assert_preserved();
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn default_and_explicit_preview_leave_every_file_unchanged() {
    let c = Case::new();
    c.histories();
    for args in [vec![], vec!["--dry-run"]] {
        let output = c.run(&args);
        success(&output);
        assert!(String::from_utf8_lossy(&output.stdout).contains("Read-only preview"));
        c.assert_all_histories_present();
    }
}

#[test]
fn deletes_all_three_histories_keeps_roots_and_live_data_then_is_idempotent() {
    let c = Case::new();
    c.histories();
    let inodes: Vec<_> = HISTORIES
        .iter()
        .map(|name| fs::metadata(c.work.join(name)).unwrap().ino())
        .collect();
    for _ in 0..2 {
        success(&c.run(&["--yes"]));
        for (name, inode) in HISTORIES.iter().zip(&inodes) {
            assert_eq!(fs::metadata(c.work.join(name)).unwrap().ino(), *inode);
            assert_eq!(fs::read_dir(c.work.join(name)).unwrap().count(), 0);
        }
        c.assert_preserved();
    }
}

#[test]
fn missing_histories_are_not_created_and_work_argument_resolves_outside_workspace() {
    let c = Case::new();
    success(
        &c.command()
            .current_dir(&c.root)
            .args(["--work", c.work.to_str().unwrap(), "--yes"])
            .output()
            .unwrap(),
    );
    for name in HISTORIES {
        assert!(!c.work.join(name).exists());
    }
    c.assert_preserved();
}

#[test]
fn history_links_are_unlinked_without_following_external_targets() {
    let c = Case::new();
    c.histories();
    fs::create_dir(c.root.join("external")).unwrap();
    fs::write(c.root.join("external/keep"), b"outside").unwrap();
    symlink(
        c.root.join("external"),
        c.work.join("epw-state/history/external"),
    )
    .unwrap();
    symlink("does-not-exist", c.work.join("epw-state/history/dangling")).unwrap();
    success(&c.run(&["--yes"]));
    assert_eq!(fs::read(c.root.join("external/keep")).unwrap(), b"outside");
    c.assert_preserved();
}

#[test]
fn root_and_ancestor_symlinks_are_rejected_before_any_history_is_removed() {
    for relative in [
        "epw-state/history",
        "epw-state/native-attempts",
        "epw-state/native-jobs",
    ] {
        let c = Case::new();
        c.histories();
        let old = c.work.join(relative);
        let external = c.root.join("external");
        fs::rename(&old, &external).unwrap();
        symlink(&external, &old).unwrap();
        let output = c.run(&["--yes"]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("rejecting links"));
        for name in HISTORIES {
            assert!(c.work.join(name).join("old/input.out").is_file());
        }
    }
}

#[test]
fn live_json_script_relative_path_and_symlink_references_block_all_deletion() {
    for kind in ["json", "relative", "script", "link"] {
        let c = Case::new();
        c.histories();
        let target = c.work.join("epw-state/history/old/input.out");
        match kind {
            "json" => c.put("results/ref.json", &json!({"source":target}).to_string()),
            "relative" => c.put(
                "results/ref.json",
                r#"{"source":"../epw-state/history/old/input.out"}"#,
            ),
            "script" => c.put("run.sh", &format!("cat '{}'\n", target.display())),
            "link" => {
                symlink(&target, c.work.join("live-link")).unwrap();
            }
            _ => unreachable!(),
        }
        let output = c.run(&["--yes"]);
        assert!(!output.status.success(), "{kind}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("References to history"));
        c.assert_all_histories_present();
    }
}

#[test]
fn current_tasks_and_unregistered_queue_jobs_block_cleanup() {
    for state in ["running", "submitted", "unknown", "submission_pending"] {
        let c = Case::new();
        c.histories();
        c.put(
            "epw-state/native-active.json",
            &json!({"state":state,"job":99}).to_string(),
        );
        c.script("sacct", "#!/bin/sh\nprintf '99|RUNNING|1|1|\\n'\n");
        assert!(!c.run(&["--yes"]).status.success(), "{state}");
        c.assert_all_histories_present();
    }
    let c = Case::new();
    c.histories();
    c.script(
        "squeue",
        &format!(
            "#!/bin/sh\nprintf '101|{}\\n'\n",
            c.work.join("7.epw-fine").display()
        ),
    );
    let output = c.run(&["--yes"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("101"));
    c.assert_all_histories_present();
}

#[test]
fn terminal_scheduler_evidence_allows_stale_submitted_record_without_rewriting_it() {
    let c = Case::new();
    c.histories();
    let original = r#"{"state":"submitted","job":99}"#;
    c.put("epw-state/native-active.json", original);
    success(&c.run(&["--yes"]));
    assert_eq!(
        fs::read_to_string(c.work.join("epw-state/native-active.json")).unwrap(),
        original
    );
    c.assert_preserved();
}

#[test]
fn independent_submission_receipt_resolves_pending_active_attempt_read_only() {
    let c = Case::new();
    c.histories();
    let snapshot = c.work.join("epw-state/attempts/current/input.toml");
    c.put(
        "epw-state/attempts/current/submission.json",
        r#"{"job":99,"error":null}"#,
    );
    let active = json!({"state":"submission_pending","job":null,"phonon_chain":true,"config":{"path":snapshot}}).to_string();
    c.put("epw-state/active.json", &active);
    success(&c.run(&["--yes"]));
    assert_eq!(
        fs::read_to_string(c.work.join("epw-state/active.json")).unwrap(),
        active
    );
}

#[test]
fn obsolete_attempt_state_does_not_override_current_validated_task() {
    let c = Case::new();
    c.histories();
    c.put(
        "epw-state/attempts/old/attempt.json",
        r#"{"state":"submission_pending","job":null}"#,
    );
    c.put(
        "epw-state/attempts/old/submission.json",
        r#"{"job":99,"error":null}"#,
    );
    c.put(
        "epw-state/attempts/old/ref.json",
        r#"{"source":"epw-state/history/old/input.out"}"#,
    );
    c.put(
        "epw-state/attempts/old/broken.json",
        "obsolete invalid record",
    );
    success(&c.run(&["--yes"]));
    c.assert_preserved();
}

#[test]
fn current_attempt_references_and_linked_active_records_are_not_bypassed() {
    for kind in ["current-snapshot", "active-link"] {
        let c = Case::new();
        c.histories();
        match kind {
            "current-snapshot" => {
                let config = c.work.join("epw-state/attempts/current/input.toml");
                c.put(
                    "epw-state/active.json",
                    &json!({"state":"succeeded","job":99,"config":{"path":config}}).to_string(),
                );
                c.put(
                    "epw-state/attempts/current/input.toml",
                    &format!(
                        "source = '{}'\n",
                        c.work.join("epw-state/history/old/input.out").display()
                    ),
                );
            }
            "active-link" => {
                fs::write(
                    c.root.join("active.json"),
                    r#"{"state":"running","job":null}"#,
                )
                .unwrap();
                symlink(
                    c.root.join("active.json"),
                    c.work.join("epw-state/native-active.json"),
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(!c.run(&["--yes"]).status.success(), "{kind}");
        c.assert_all_histories_present();
    }
}

#[test]
fn invalid_layout_records_and_scheduler_errors_stop_before_deletion() {
    for kind in ["layout", "missing-state", "broken-json", "scheduler"] {
        let c = Case::new();
        c.histories();
        match kind {
            "layout" => c.put(
                "epw-state/layout.json",
                r#"{"schema_version":2,"phonon":"nosplit"}"#,
            ),
            "missing-state" => c.put("epw-state/native-active.json", r#"{"job":99}"#),
            "broken-json" => c.put("epw-state/native-active.json", "invalid"),
            "scheduler" => c.script("squeue", "#!/bin/sh\nexit 1\n"),
            _ => unreachable!(),
        }
        assert!(!c.run(&["--yes"]).status.success(), "{kind}");
        c.assert_all_histories_present();
    }
}

#[test]
fn cwd_inside_history_and_conflicting_flags_are_rejected() {
    let c = Case::new();
    c.histories();
    assert!(!c
        .command()
        .current_dir(c.work.join("epw-state/history/old"))
        .args(["--work", c.work.to_str().unwrap(), "--yes"])
        .output()
        .unwrap()
        .status
        .success());
    assert!(!c.run(&["--yes", "--dry-run"]).status.success());
    c.assert_all_histories_present();
}

#[test]
fn partial_unlink_failure_lists_path_and_keeps_live_data() {
    let c = Case::new();
    c.histories();
    let mut command = c.command();
    if unsafe { libc::geteuid() } == 0 {
        // root 可绕过目录权限；仅对子进程降权，确保容器中也真正覆盖 unlink 失败。
        use std::os::unix::process::CommandExt;
        fn writable(path: &std::path::Path) {
            fs::set_permissions(path, fs::Permissions::from_mode(0o777)).unwrap();
            for entry in fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    writable(&path);
                }
            }
        }
        writable(&c.root);
        command.uid(65534);
    }
    let protected = c.work.join("epw-state/history/old");
    fs::set_permissions(&protected, fs::Permissions::from_mode(0o555)).unwrap();
    let output = command.arg("--yes").output().unwrap();
    fs::set_permissions(&protected, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(protected.to_str().unwrap()));
    c.assert_preserved();
}

#[test]
fn directories_named_history_outside_allowlist_are_preserved() {
    let c = Case::new();
    c.histories();
    c.put("7.epw-fine/history/keep.out", "keep unrelated");
    success(&c.run(&["--yes"]));
    assert_eq!(
        fs::read(c.work.join("7.epw-fine/history/keep.out")).unwrap(),
        b"keep unrelated"
    );
}
