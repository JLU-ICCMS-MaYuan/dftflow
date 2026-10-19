//! 三个真实进程的阶段边界、脚本生成和执行身份；不调用真实 QE/EPW/Slurm。
use epwflow::foundation::store;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output},
};
const BINS: [&str; 3] = [
    env!("CARGO_BIN_EXE_epw_phonon"),
    env!("CARGO_BIN_EXE_epw_wannier"),
    env!("CARGO_BIN_EXE_epw_superconductivity"),
];
struct Case {
    root: PathBuf,
}
impl Case {
    fn new() -> Self {
        let c = Self {
            root: std::env::temp_dir().join(format!("epw-three-{}", store::token())),
        };
        fs::create_dir(&c.root).unwrap();
        fs::write(
            c.root.join("inputepw.toml"),
            include_str!("fixtures/input.toml"),
        )
        .unwrap();
        fs::write(c.root.join("POSCAR"), "MgB2\n1\n3 0 0\n-1.5 2.598 0\n0 0 3.5\nMg B\n1 2\nDirect\n0 0 0\n0.3333 0.6667 0.5\n0.6667 0.3333 0.5\n").unwrap();
        for name in ["Mg.upf", "B.upf"] {
            fs::write(c.root.join(name), "fixture").unwrap();
        }
        for name in ["pw.x", "ph.x", "epw.x", "launcher", "sbatch"] {
            c.script(name, "#!/bin/sh\nprintf 'called\\n' >> calls\nexit 91\n");
        }
        c.script("scontrol", include_str!("fixtures/slurm_controller.py"));
        c.script("sacct", "#!/bin/sh\nprintf '123|RUNNING|0|1|\\n'\n");
        c
    }
    fn script(&self, name: &str, text: &str) {
        let p = self.root.join(name);
        fs::write(&p, text).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fn cmd(&self, i: usize) -> Command {
        let mut cmd = Command::new(BINS[i]);
        cmd.current_dir(&self.root)
            .env("PATH", format!("{}:/usr/bin:/bin", self.root.display()))
            .env("EPWFLOW_LOCAL_STATE", self.root.join("state"))
            .env_remove("SLURM_JOB_ID");
        cmd
    }
    fn run(&self, i: usize, args: &[&str]) -> Output {
        self.cmd(i).args(args).output().unwrap()
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn success(o: Output) {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn exactly_three_programs_and_no_legacy_public_subcommands() {
    let cargo: toml::Value = toml::from_str(include_str!("../Cargo.toml")).unwrap();
    assert_eq!(cargo["package"]["autobins"].as_bool(), Some(false));
    let names: Vec<_> = cargo["bin"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["epw_phonon", "epw_wannier", "epw_superconductivity"]
    );
    let c = Case::new();
    for (i, name) in names.iter().enumerate() {
        let out = c.run(i, &["--help"]);
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains(name));
        success(c.run(i, &["--check", "--json"]));
        assert!(!c
            .run(i, &["submit", "-c", "inputepw.toml"])
            .status
            .success());
    }
    assert!(!c.root.join("run").exists());
    assert!(!c.root.join("calls").exists());
}

#[test]
fn default_generation_is_idempotent_and_does_not_reserve_or_submit() {
    let c = Case::new();
    for i in 0..3 {
        success(c.run(i, &[]));
    }
    let dir = c.root.join("run/.epwflow/generated");
    for name in ["qe-data", "interpolation", "all"] {
        let file = dir.join(format!("{name}.sh"));
        let id = store::Identity::capture(&file).unwrap();
        assert!(Command::new("/bin/bash")
            .args(["-n"])
            .arg(&file)
            .status()
            .unwrap()
            .success());
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.contains("--run-script"));
        assert!(text.contains("#SBATCH --ntasks=1"));
        success(c.run(
            match name {
                "qe-data" => 0,
                "interpolation" => 1,
                _ => 2,
            },
            &[],
        ));
        id.verify().unwrap();
    }
    assert!(!c.root.join("calls").exists());
    assert!(!c.root.join("campaign/ledger.json").exists());
    assert!(!c.root.join("run/calculations").exists());
    for i in [1, 2] {
        let out = c.run(i, &["-s"]);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("前一个程序"));
    }
    assert!(!c.root.join("calls").exists());
}

#[test]
fn generated_script_is_authenticated_and_registers_one_allocation() {
    let c = Case::new();
    success(c.run(0, &[]));
    let script = c.root.join("run/.epwflow/generated/qe-data.sh");
    let out = c
        .cmd(0)
        .args(["--run-script", "-w", "run"])
        .env("SLURM_JOB_ID", "123")
        .output()
        .unwrap();
    assert!(!out.status.success(), "替身退出必须失败");
    let r: serde_json::Value = store::read(&c.root.join("run/.epwflow/workflow.json")).unwrap();
    assert_eq!(r["state"], "failed");
    let l: serde_json::Value = store::read(&c.root.join("campaign/ledger.json")).unwrap();
    assert_eq!(l["allocations"].as_array().unwrap().len(), 1);
    assert_eq!(l["reservations"].as_array().unwrap().len(), 0);
    let attempt = r["attempt_id"].as_str().unwrap();
    let out = c
        .cmd(1)
        .args(["--execute-attempt", attempt, "-w", "run"])
        .env("SLURM_JOB_ID", "123")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("程序与记录阶段不同"));
    fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
    let out = c
        .cmd(0)
        .args(["--run-script", "-w", "run"])
        .env("SLURM_JOB_ID", "124")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let after: serde_json::Value = store::read(&c.root.join("campaign/ledger.json")).unwrap();
    assert_eq!(l, after);
}

#[test]
fn all_generation_submits_afterok_once_and_authenticates_every_stage() {
    let c = Case::new();
    success(c.run(0, &["--all"]));
    success(c.run(0, &["--all"]));
    assert!(!c.root.join("calls").exists());
    assert!(!c.root.join("campaign/ledger.json").exists());
    let dir = c.root.join("run/.epwflow/generated");
    assert!(Command::new("bash")
        .arg("-n")
        .arg(dir.join("submit-all.sh"))
        .status()
        .unwrap()
        .success());
    c.script("sbatch", &format!("#!/bin/bash\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nn=$(wc -l < '{}')\nprintf '%s\\n' \"$((100+n))\"\n", c.root.join("calls").display(), c.root.join("calls").display()));
    success(c.run(0, &["--submit-chain", "-w", "run"]));
    success(c.run(0, &["--submit-chain", "-w", "run"]));
    let calls = fs::read_to_string(c.root.join("calls")).unwrap();
    let rows: Vec<_> = calls.lines().collect();
    assert_eq!(rows.len(), 3);
    assert!(!rows[0].contains("--dependency"));
    assert!(rows[1].contains("--dependency=afterok:101"));
    assert!(rows[2].contains("--dependency=afterok:102"));
    assert!(rows.iter().all(|s| s.contains("--kill-on-invalid-dep=yes")));
    let receipt: serde_json::Value = store::read(&dir.join("chain-submission.json")).unwrap();
    assert_eq!(receipt["state"], "submitted");
    assert_eq!(receipt["jobs"].as_array().unwrap().len(), 3);
    assert!(!c.root.join("campaign/ledger.json").exists());
    fs::write(dir.join("all.sh"), "#!/bin/bash\nexit 0\n").unwrap();
    assert!(!c.run(0, &["--submit-chain", "-w", "run"]).status.success());
    assert_eq!(fs::read_to_string(c.root.join("calls")).unwrap(), calls);
    assert!(!c.run(1, &["--all"]).status.success());
}

#[test]
fn partial_chain_receipt_is_persistent_and_never_retried() {
    let c = Case::new();
    success(c.run(0, &["--all"]));
    c.script("sbatch", &format!("#!/bin/bash\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nn=$(wc -l < '{}')\nif [ \"$n\" = 1 ]; then printf '101\\n'; else printf 'unrecognized receipt\\n'; fi\n", c.root.join("calls").display(), c.root.join("calls").display()));
    for _ in 0..2 {
        assert!(!c.run(0, &["--submit-chain", "-w", "run"]).status.success());
    }
    assert_eq!(
        fs::read_to_string(c.root.join("calls"))
            .unwrap()
            .lines()
            .count(),
        2
    );
    let receipt: serde_json::Value =
        store::read(&c.root.join("run/.epwflow/generated/chain-submission.json")).unwrap();
    assert_eq!(receipt["jobs"].as_array().unwrap().len(), 1);
    assert_eq!(receipt["submitting_phase"], 2);
    assert_eq!(receipt["state"], "submitting");
    assert_eq!(receipt["responses"][1]["stdout"], "unrecognized receipt\n");
    assert_eq!(receipt["responses"][1]["exit_code"], 0);
}

#[test]
fn environment_bootstrap_accepts_optional_variables_and_keeps_strict_execution() {
    let c = Case::new();
    let config = c.root.join("inputepw.toml");
    let environment = c.root.join("environment ' with spaces.sh");
    fs::write(
        &environment,
        "test \"$1\" = --force\nexport EPW_TEST_ENV=\"${EPW_TEST_OPTIONAL}ready\"\n",
    )
    .unwrap();
    let mut cfg: toml::Value = toml::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    cfg["programs"].as_table_mut().unwrap().insert(
        "environment_script".into(),
        environment.to_string_lossy().to_string().into(),
    );
    fs::write(&config, toml::to_string_pretty(&cfg).unwrap()).unwrap();
    success(c.run(0, &["--all"]));
    c.script("sbatch", "#!/bin/sh\nprintf '123\\n'\n");
    success(c.run(0, &["-s"]));
    let workflow: serde_json::Value =
        store::read(&c.root.join("run/.epwflow/workflow.json")).unwrap();
    let mut scripts: Vec<_> = ["qe-data", "interpolation", "all"]
        .iter()
        .map(|name| c.root.join(format!("run/.epwflow/generated/{name}.sh")))
        .collect();
    scripts.push(PathBuf::from(workflow["script"]["path"].as_str().unwrap()));
    for path in scripts {
        let script = fs::read_to_string(path).unwrap();
        // 实际生成脚本的完整初始化段，替换的只有末尾调度／科学入口。
        let (prefix, _) = script.rsplit_once("\nexec ").unwrap();
        let run = |suffix: &str| {
            Command::new("/bin/bash")
                .arg("-c")
                .arg(format!("{prefix}\n{suffix}\n"))
                .env_remove("EPW_TEST_OPTIONAL")
                .env_remove("EPW_TEST_UNSET")
                .output()
                .unwrap()
        };
        let out = run("[[ $- == *u* ]]\nprintf '%s' \"$EPW_TEST_ENV\"");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(out.stdout, b"ready");
        let out = run("printf '%s' \"$EPW_TEST_UNSET\"");
        assert!(!out.status.success(), "执行阶段必须恢复 nounset");
        fs::write(&environment, "return 17\n").unwrap();
        let out = run("printf should-not-run");
        assert_eq!(out.status.code(), Some(17));
        assert!(out.stdout.is_empty(), "环境加载失败不得继续执行");
        fs::write(
            &environment,
            "test \"$1\" = --force\nexport EPW_TEST_ENV=\"${EPW_TEST_OPTIONAL}ready\"\n",
        )
        .unwrap();
    }
}
