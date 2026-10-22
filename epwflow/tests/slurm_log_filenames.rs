//! 0022：实际生成与提交入口的日志目录契约；调度器为隔离替身，不提交科学作业。
#![cfg(unix)]
use epwflow::foundation::{config::Config, slurm, stages::Step, store, workflow, workspace};
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const PHONON: &str = env!("CARGO_BIN_EXE_epw_phonon");
const CHILD_ROOT: &str = "DFTFLOW_SLURM_LOG_TEST_ROOT";

struct Case {
    root: PathBuf,
    work: PathBuf,
}
impl Case {
    fn new(numbered: bool) -> Self {
        let root = std::env::temp_dir().join(format!("epw-log-names-{}", store::token()));
        fs::create_dir(&root).unwrap();
        let c = Self {
            work: root.join("run with 'quotes' and spaces"),
            root,
        };
        fs::write(c.root.join("POSCAR"), "MgB2\n1\n3 0 0\n-1.5 2.598 0\n0 0 3.5\nMg B\n1 2\nDirect\n0 0 0\n0.3333 0.6667 0.5\n0.6667 0.3333 0.5\n").unwrap();
        fs::write(c.root.join("KPATH.in"), "0 0 0 G\n0.5 0 0 M\n").unwrap();
        for name in ["Mg.upf", "B.upf"] {
            fs::write(c.root.join(name), "fixture").unwrap();
        }
        for name in ["pw.x", "ph.x", "epw.x", "bands.x", "dos.x", "launcher"] {
            c.executable(name, "#!/bin/sh\nexit 91\n");
        }
        c.executable("sbatch", include_str!("fixtures/sbatch_log_paths.py"));
        c.executable("squeue", "#!/bin/sh\nprintf '201|RUNNING|fixture\\n'\n");
        c.executable("sacct", "#!/bin/sh\nprintf '201|RUNNING|0|1|\\n'\n");
        let mut cfg: Config = toml::from_str(include_str!("fixtures/input.toml")).unwrap();
        cfg.work_dir = c.work.clone();
        cfg.kpath = Some(c.root.join("KPATH.in"));
        cfg.programs.launcher.clear();
        if numbered {
            cfg.layout = Some("numbered".into());
        }
        fs::write(c.config(), toml::to_string_pretty(&cfg).unwrap()).unwrap();
        c
    }
    fn config(&self) -> PathBuf {
        self.root.join("inputepw.toml")
    }
    fn executable(&self, name: &str, text: &str) {
        let path = self.root.join(name);
        fs::write(&path, text).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fn command(&self, program: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(&self.root)
            .env("PATH", format!("{}:/usr/bin:/bin", self.root.display()))
            .env("EPWFLOW_LOCAL_STATE", self.root.join("state"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env_remove("SLURM_JOB_ID")
            .env_remove("SLURM_JOBID");
        command
    }
    fn rows(&self) -> Vec<Value> {
        fs::read_to_string(self.root.join("scheduler.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
    fn submit_from_elsewhere(&self, script: &Path) {
        success(
            self.command(self.root.join("sbatch"))
                .arg("--parsable")
                .arg(script)
                .current_dir("/")
                .output()
                .unwrap(),
        );
    }
    fn api_child(&self, test_name: &str) {
        success(
            self.command(std::env::current_exe().unwrap())
                .args(["--exact", test_name, "--nocapture"])
                .env(CHILD_ROOT, &self.root)
                .output()
                .unwrap(),
        );
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
fn success(output: Output) {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn check_row(row: &Value, job_placeholder: bool, merged: bool) {
    let script = Path::new(row["script"].as_str().unwrap());
    for key in ["output", "error"] {
        let filename = row["options"][key].as_str().unwrap();
        assert_eq!(Path::new(filename).file_name().unwrap(), filename);
        assert_eq!(filename.contains("%j"), job_placeholder, "{filename}");
        assert_eq!(
            row["directives"][key], row["options"][key],
            "提交参数不能覆盖日志目录"
        );
        let path = Path::new(row[key].as_str().unwrap());
        assert_eq!(path.parent(), script.parent());
        assert!(path.is_file());
    }
    assert_eq!(row["output"] == row["error"], merged);
    assert!(Command::new("/bin/bash")
        .arg("-n")
        .arg(script)
        .status()
        .unwrap()
        .success());
}

#[test]
fn numbered_scripts_keep_logs_local_when_submitted_from_other_directories() {
    let c = Case::new(true);
    workspace::generate(&c.config(), Step::Electronic).unwrap();
    let scripts = [
        "1.qe-scf/submit.sh",
        "2.qe-nscf/submit.sh",
        "3.qe-electronic/submit.sh",
        "3.qe-electronic/qe_eband/submit.sh",
        "3.qe-electronic/qe_eledos/submit.sh",
    ];
    let identities: Vec<_> = scripts
        .iter()
        .map(|name| store::Identity::capture(&c.work.join(name)).unwrap())
        .collect();
    for identity in &identities {
        c.submit_from_elsewhere(&identity.path);
        identity.verify().unwrap();
    }
    let rows = c.rows();
    assert_eq!(rows.len(), scripts.len());
    for row in rows {
        assert_eq!(row["cwd"], "/");
        check_row(&row, true, false);
    }
}

#[test]
fn generated_chain_keeps_logs_local_and_preserves_frozen_scripts() {
    let c = Case::new(false);
    success(
        c.command(PHONON)
            .args(["-c", c.config().to_str().unwrap(), "--all"])
            .output()
            .unwrap(),
    );
    let dir = c.work.join("epw-state/generated");
    let identities: Vec<_> = ["qe-data", "interpolation", "all"]
        .iter()
        .map(|name| store::Identity::capture(&dir.join(format!("{name}.sh"))).unwrap())
        .collect();
    success(
        c.command(PHONON)
            .args(["-c", c.config().to_str().unwrap(), "--all"])
            .output()
            .unwrap(),
    );
    success(
        c.command("/bin/bash")
            .arg(dir.join("submit-all.sh"))
            .current_dir("/")
            .output()
            .unwrap(),
    );
    for identity in &identities {
        identity.verify().unwrap();
        c.submit_from_elsewhere(&identity.path);
    }
    let rows = c.rows();
    assert_eq!(rows.len(), 6);
    for (index, row) in rows.into_iter().enumerate() {
        assert_eq!(
            row["cwd"],
            if index < 3 {
                c.work.to_str().unwrap()
            } else {
                "/"
            }
        );
        check_row(&row, true, false);
    }
}

#[test]
fn public_submission_uses_short_names_and_refuses_duplicate_queued_jobs() {
    let c = Case::new(true);
    success(
        c.command(PHONON)
            .args(["-c", c.config().to_str().unwrap(), "--stage", "electronic", "-s"])
            .current_dir("/")
            .output()
            .unwrap(),
    );
    let rows = c.rows();
    assert_eq!(rows.len(), 1);
    check_row(&rows[0], false, true);
    assert_eq!(rows[0]["options"]["output"], "slurm-scf.log");
    assert!(rows[0]["options"].get("chdir").is_none());
    assert!(!rows[0]["argv"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().starts_with("--chdir")));
    let script = Path::new(rows[0]["script"].as_str().unwrap());
    assert_eq!(script.file_name().unwrap(), "scf.sh");
    assert_eq!(Path::new(rows[0]["cwd"].as_str().unwrap()), script.parent().unwrap());
    let identity =
        store::Identity::capture(Path::new(rows[0]["script"].as_str().unwrap())).unwrap();
    let _ = c
        .command(PHONON)
        .args(["-c", c.config().to_str().unwrap(), "--stage", "electronic", "-s"])
        .output()
        .unwrap();
    assert_eq!(c.rows().len(), 1, "排队中的任务不能重复提交");
    identity.verify().unwrap();
}

#[test]
fn legacy_workflow_submission_does_not_override_log_names_with_paths() {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        workflow::start(
            &PathBuf::from(root).join("inputepw.toml"),
            workflow::Boundary::QeData,
            true,
        )
        .unwrap();
        return;
    }
    let c = Case::new(false);
    c.api_child("legacy_workflow_submission_does_not_override_log_names_with_paths");
    let rows = c.rows();
    assert_eq!(rows.len(), 1);
    check_row(&rows[0], true, false);
    for argument in rows[0]["argv"].as_array().unwrap() {
        let argument = argument.as_str().unwrap();
        assert!(!argument.starts_with("--output=") && !argument.starts_with("--error="));
    }
}

#[test]
fn stage_backend_keeps_log_directory_separate_from_scientific_working_directory() {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let cfg = Config::load(&PathBuf::from(root).join("inputepw.toml")).unwrap();
        fs::create_dir_all(&cfg.work_dir).unwrap();
        slurm::submit_via_sbatch(&[], &["/bin/pwd".into()], &cfg.work_dir, "check-cwd").unwrap();
        return;
    }
    let c = Case::new(false);
    c.api_child("stage_backend_keeps_log_directory_separate_from_scientific_working_directory");
    let rows = c.rows();
    assert_eq!(rows.len(), 1);
    check_row(&rows[0], true, false);
    let script = Path::new(rows[0]["script"].as_str().unwrap());
    let result = c
        .command("/bin/bash")
        .arg(script)
        .current_dir("/")
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim(),
        c.work.to_str().unwrap()
    );
}

#[test]
fn wannier_default_logs_stay_local_and_custom_headers_are_preserved() {
    let c = Case::new(false);
    fs::create_dir_all(&c.work).unwrap();
    let mut cfg = wannier90flow::config::wannier_init_template();
    cfg["wannier90"]["executable_path"] = "/bin/true".into();
    cfg["pw2wannier90"]["executable_path"] = "/bin/true".into();
    cfg["postw90"] = serde_json::json!({"executable_path": "/bin/pwd"});
    let generate = |cfg: &Value| {
        wannier90flow::init::create_run_script(c.work.to_str().unwrap(), "seed", cfg).unwrap();
        wannier90flow::post::create_run_script(c.work.to_str().unwrap(), "seed", cfg).unwrap();
    };
    generate(&cfg);
    for name in ["run_wannier90.sh", "run_postw90.sh"] {
        c.submit_from_elsewhere(&c.work.join(name));
    }
    for row in c.rows() {
        assert_eq!(row["cwd"], "/");
        check_row(&row, false, false);
    }
    // Slurm 执行的是 spool 副本，正文不能用 $0 的目录覆盖正确的工作目录。
    let spool = c.root.join("slurm_script");
    fs::copy(c.work.join("run_postw90.sh"), &spool).unwrap();
    success(
        c.command("/bin/bash")
            .arg(spool)
            .current_dir("/")
            .output()
            .unwrap(),
    );
    assert_eq!(
        fs::read_to_string(c.work.join("POSTW90.log"))
            .unwrap()
            .lines()
            .last()
            .unwrap(),
        c.work.to_str().unwrap()
    );
    let header = "#!/bin/bash\n#SBATCH --output=/user/chosen/stdout\n#SBATCH --error=/user/chosen/stderr\n#SBATCH --chdir=/user/chosen\n";
    cfg["slurm"]["header"] = header.into();
    generate(&cfg);
    for name in ["run_wannier90.sh", "run_postw90.sh"] {
        let text = fs::read_to_string(c.work.join(name)).unwrap();
        assert!(text.starts_with(header));
        assert_eq!(text.matches("#SBATCH --chdir=").count(), 1);
    }
}
