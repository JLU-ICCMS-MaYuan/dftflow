//! 无 accounting 数据库时，公开调度查询只采用真实控制器／完成日志证据。
#![cfg(unix)]
use epwflow::foundation::{slurm, store};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn completion_query_child() {
    let Ok(kind) = std::env::var("EPW_COMPLETION_CASE") else { return };
    let result = slurm::accounting(3);
    match kind.as_str() {
        "completed" => {
            let a = result.unwrap();
            assert!(a.terminal);
            assert_eq!(a.state, "FAILED");
            assert_eq!(a.nodes, 1);
            assert_eq!(a.node_seconds, 3);
            assert_eq!(slurm::exit_code_record(3).unwrap().trim(), "3|1:0");
            assert_eq!(slurm::query(3).as_deref(), Some("FAILED"));
        }
        "running" => {
            let a = result.unwrap();
            assert!(!a.terminal);
            assert_eq!(a.state, "RUNNING");
            assert_eq!(a.nodes, 1);
        }
        _ => assert!(result.is_err(), "{kind} We can't create a fictional end"),
    }
}

#[test]
fn disabled_accounting_queries_a_private_copy_and_preserves_unknown_jobs() {
    for kind in ["completed", "running", "missing", "duplicate", "query_failed", "other_error"] {
        let root = std::env::temp_dir().join(format!("epw-jobcomp-test-{}", store::token()));
        fs::create_dir(&root).unwrap();
        let source = root.join("source.log");
        fs::write(&source, "original read-only completion evidence\n").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o444)).unwrap();
        let programs = [
            ("scontrol", r#"#!/bin/sh
if [ "$2" = job ]; then
  [ "$EPW_COMPLETION_CASE" = running ] || exit 1
  printf 'JobId=3 JobState=RUNNING NumNodes=1\n'
else
  printf 'JobCompType = jobcomp/filetxt\nJobCompLoc = %s\n' "$EPW_COMPLETION_SOURCE"
fi
"#),
            ("sacct", r#"#!/bin/sh
if [ "$1" != --completion ]; then
  if [ "$EPW_COMPLETION_CASE" = other_error ]; then
    echo 'database connection failed' >&2
  else
    echo 'Slurm accounting storage is disabled' >&2
  fi
  exit 1
fi
[ "$2" = --file ] || exit 2
cmp "$3" "$EPW_COMPLETION_SOURCE" || exit 3
[ "$3" != "$EPW_COMPLETION_SOURCE" ] || exit 4
printf '%s\n' "$3" > "$EPW_COMPLETION_QUERY"
case "$*" in
  *--format=JobIDRaw,ExitCode) printf '3|1:0\n'; exit 0 ;;
  *--format=JobIDRaw,State,ElapsedRaw,NNodes) ;;
  *) exit 5 ;;
esac
case "$EPW_COMPLETION_CASE" in
  completed) printf '3|FAILED|3|1|\n' ;;
  missing) printf '4|COMPLETED|5|1|\n' ;;
  duplicate) printf '3|FAILED|3|1|\n3|COMPLETED|4|1|\n' ;;
  query_failed) exit 9 ;;
esac
"#),
        ];
        for (name, body) in programs {
            let path = root.join(name);
            fs::write(&path, body).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let query = root.join("query");
        let out = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "completion_query_child", "--nocapture"])
            .env("PATH", format!("{}:/usr/bin:/bin", root.display()))
            .env("EPW_COMPLETION_CASE", kind)
            .env("EPW_COMPLETION_SOURCE", &source)
            .env("EPW_COMPLETION_QUERY", &query)
            .output().unwrap();
        assert!(out.status.success(), "{kind}: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(fs::read_to_string(&source).unwrap(), "original read-only completion evidence\n");
        assert_eq!(fs::metadata(&source).unwrap().permissions().mode() & 0o777, 0o444);
        if query.exists() {
            let snapshot = fs::read_to_string(&query).unwrap();
            assert!(!std::path::Path::new(snapshot.trim()).exists(), "Query copy uncleaned");
        } else {
            assert!(matches!(kind, "running" | "other_error"));
        }
        fs::remove_dir_all(root).unwrap();
    }
}
