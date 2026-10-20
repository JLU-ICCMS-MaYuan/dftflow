//! 三个公开程序与实际脚本的连续进程契约；全部数值为隔离替身，非科学验收。
#![cfg(unix)]
use epwflow::foundation::{config, slurm::shell_quote, store};
use serde_json::{json, Value};
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
    preserve: bool,
}
impl Drop for Case {
    fn drop(&mut self) {
        if !self.preserve && !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
impl Case {
    fn new() -> Self {
        let retained = std::env::var_os("DFTFLOW_THREE_STAGE_TEST_ROOT");
        let preserve = retained.is_some();
        let parent = retained
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::create_dir_all(&parent).unwrap();
        let root = parent.join(format!("public-chain-{}", store::token()));
        fs::create_dir(&root).unwrap();
        eprintln!("完整入口进程证据：{}", root.display());
        let c = Self { root, preserve };
        c.put(
            "POSCAR",
            "Si\n1\n1 0 0\n0 1 0\n0 0 1\nSi\n2\nDirect\n0 0 0\n0.25 0.25 0.25\n",
        );
        c.put("KPATH.in", "0 0 0 G\n0.5 0 0 X\n");
        c.put("qe_native_io.py", include_str!("fixtures/qe_native_io.py"));
        c.put(
            "epw_native_io.py",
            include_str!("fixtures/epw_native_io.py"),
        );
        c.put("epw_tc_io.py", include_str!("fixtures/epw_tc_io.py"));
        for kind in ["pw", "ph", "q2r", "matdyn", "dynmat"] {
            c.script(
                &format!("{kind}.x"),
                &format!(
                    "#!/bin/sh\nexec /usr/bin/python3 {} {kind} \"$@\"\n",
                    shell_quote(c.root.join("qe_native_io.py").to_str().unwrap())
                ),
            );
        }
        c.script("epw.x", &format!(r#"#!/usr/bin/python3
import pathlib,re,runpy,sys
root=pathlib.Path({root})
text=pathlib.Path(sys.argv[sys.argv.index('-inp')+1]).read_text()
if (root/'fail-coarse').exists() and pathlib.Path(sys.argv[sys.argv.index('-inp')+1]).name == 'epw_coarse.in':
    sys.exit(17)
assert not any(re.search(r'\b'+name+r'\s*=',text) for name in ['dis_proj_min','dis_proj_max','dis_froz_proj'])
name='epw_tc_io.py' if re.search(r'(?m)^\s*(?:liso|laniso)\s*=\s*\.true\.',text) else 'epw_native_io.py'
runpy.run_path(str(root/name),run_name='__main__')
"#, root=serde_json::to_string(c.root.to_str().unwrap()).unwrap()));
        c.script(
            "sacct",
            &format!(
                "#!/bin/sh\ncat {}\n",
                shell_quote(c.root.join("accounting").to_str().unwrap())
            ),
        );
        c.script("scontrol", include_str!("fixtures/slurm_controller.py"));
        c.script("squeue", "#!/bin/sh\nexit 0\n");
        c.script("sbatch", &format!("#!/bin/bash\nprintf '%s\\n' \"$*\" >> {calls}\nn=$(wc -l < {calls})\nprintf '%s\\n' \"$((200+n))\"\n", calls=shell_quote(c.root.join("submissions").to_str().unwrap())));
        let mut cfg: config::Config = toml::from_str(include_str!("fixtures/input.toml")).unwrap();
        cfg.work_dir = c.root.join("run");
        cfg.outdir = c.root.join("scratch");
        cfg.structure = c.root.join("POSCAR");
        cfg.kpath = Some(c.root.join("KPATH.in"));
        cfg.pseudos = [("Si".into(), cfg.structure.clone())].into();
        cfg.atomic_masses.clear();
        cfg.budget.ledger = c.root.join("fixture-ledger.json");
        cfg.programs.launcher.clear();
        cfg.programs.environment_script = None;
        for (program, name) in [
            (&mut cfg.programs.pw, "pw.x"),
            (&mut cfg.programs.ph, "ph.x"),
            (&mut cfg.programs.epw, "epw.x"),
        ] {
            program.path = c.root.join(name);
            program.args.clear();
        }
        cfg.programs.epw.args = vec!["--io-contract".into()];
        cfg.programs.wannier90 = None;
        cfg.programs.pw2wannier90 = None;
        cfg.resources.wall_seconds = 600;
        cfg.resources.storage_bytes = 64 * 1024 * 1024;
        cfg.qe.prefix = "scf".into();
        cfg.qe.scf.k_grid = [2; 3];
        cfg.qe.nscf.k_grid = [2; 3];
        cfg.qe.nscf.nbnd = 16;
        cfg.phonon.mode = config::PhMode::Nosplit;
        cfg.phonon.q_grid = [2; 3];
        cfg.wannier.num_wann = 5;
        cfg.wannier.selection_mode = qf_wannier::SelectionMode::EnergyOnly;
        cfg.wannier.projections = vec!["Si:sp3".into()];
        cfg.wannier.energy_reference = config::EnergyReference::QeZero;
        cfg.wannier.exclude_bands = (1..=7).collect();
        cfg.wannier.outer_window_ev = Some([1.5, 5.]);
        cfg.wannier.frozen_window_ev = Some([2.2, 3.3]);
        cfg.wannier.frozen_max_ev = None;
        cfg.interpolation.k_grid = [4; 3];
        cfg.interpolation.q_grid = [2; 3];
        cfg.interpolation.spectral_points = 4;
        cfg.interpolation.phonon_smearings_mev = vec![0.5];
        cfg.interpolation.quality.k_points_crystal = vec![
            [0.; 3],
            [0.25, 0., 0.],
            [0.5, 0., 0.],
            [0.75, 0., 0.],
            [0., 0.25, 0.],
            [0., 0.5, 0.],
        ];
        cfg.interpolation.quality.q_points_crystal =
            vec![[0.; 3], [0.25, 0., 0.], [0.5, 0., 0.], [0.75, 0., 0.]];
        cfg.interpolation.quality.reference_manifest = None;
        cfg.interpolation.quality.phonon_max_cm1 = 50.;
        cfg.interpolation.quality.phonon_rms_cm1 = 20.;
        cfg.comparison = Some(config::Comparison {
            points_per_edge: 2,
            ..Default::default()
        });
        cfg.superconductivity.mode = config::Mode::Both;
        cfg.superconductivity.frequency_cutoff_ev = 0.03;
        cfg.superconductivity.gap_temperatures_k = vec![5., 20.];
        let tc = cfg.superconductivity.iso.as_mut().unwrap();
        tc.min_k = 5.;
        tc.max_k = 50.;
        cfg.superconductivity.aniso = cfg.superconductivity.iso.clone();
        cfg.validate().unwrap();
        c.put("inputepw.toml", &toml::to_string_pretty(&cfg).unwrap());
        c.accounting(None);
        c
    }
    fn put(&self, name: &str, text: &str) {
        fs::write(self.root.join(name), text).unwrap();
    }
    fn script(&self, name: &str, text: &str) {
        self.put(name, text);
        fs::set_permissions(self.root.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    fn accounting(&self, active: Option<u64>) {
        self.put(
            "accounting",
            &(201..=220)
                .map(|id| {
                    format!(
                        "{id}|{}|1|1|\n",
                        if Some(id) == active {
                            "RUNNING"
                        } else {
                            "COMPLETED"
                        }
                    )
                })
                .collect::<String>(),
        );
    }
    fn environment(&self, cmd: &mut Command, job: Option<u64>) {
        cmd.current_dir(&self.root)
            .env("PATH", format!("{}:/usr/bin:/bin", self.root.display()))
            .env("EPWFLOW_LOCAL_STATE", self.root.join("state"))
            .env("XDG_STATE_HOME", self.root.join("xdg"))
            .env_remove("SLURM_JOB_ID")
            .env_remove("SLURM_JOBID");
        if let Some(job) = job {
            cmd.env("SLURM_JOB_ID", job.to_string());
        }
    }
    fn output(&self, mut cmd: Command, job: Option<u64>, label: &str) -> Output {
        self.environment(&mut cmd, job);
        let o = cmd.output().unwrap();
        self.put(
            &format!("{label}.stdout"),
            &String::from_utf8_lossy(&o.stdout),
        );
        self.put(
            &format!("{label}.stderr"),
            &String::from_utf8_lossy(&o.stderr),
        );
        store::write(
            &self.root.join(format!("{label}.json")),
            &json!({"exit_code":o.status.code(),"job":job}),
        )
        .unwrap();
        eprintln!("{label}: {:?}", o.status.code());
        o
    }
    fn cli(&self, i: usize, args: &[&str], label: &str) -> Output {
        let mut cmd = Command::new(BINS[i]);
        cmd.args(args);
        self.output(cmd, None, label)
    }
    fn bash(&self, path: impl AsRef<std::path::Path>, job: Option<u64>, label: &str) -> Output {
        let mut cmd = Command::new("/bin/bash");
        cmd.arg(path.as_ref());
        self.output(cmd, job, label)
    }
    fn workflow(&self) -> Value {
        store::read(&self.root.join("run/.epwflow/workflow.json")).unwrap()
    }
    fn calls(&self) -> (String, String, String) {
        let read = |name| fs::read_to_string(self.root.join(name)).unwrap_or_default();
        (
            read("scientific-calls"),
            read("epw-calls"),
            read("tc-calls.jsonl"),
        )
    }
}
fn success(o: Output) {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
fn copy_results(source: &std::path::Path, target: &std::path::Path) {
    fs::create_dir(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let to = target.join(entry.file_name());
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            copy_results(&entry.path(), &to);
        } else {
            assert!(kind.is_file(), "结果包只能包含普通文件与目录");
            fs::copy(entry.path(), to).unwrap();
        }
    }
}

#[test]
fn failed_coarse_can_change_mpi_without_repeating_qe_or_independent_references() {
    let c = Case::new();
    c.script(
        "launcher",
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\nshift 2\nexec \"$@\"\n",
            shell_quote(c.root.join("mpi-launches").to_str().unwrap())
        ),
    );
    let input = c.root.join("inputepw.toml");
    let mut cfg = config::Config::load(&input).unwrap();
    cfg.resources.mpi = 16;
    cfg.programs.launcher = vec![
        c.root.join("launcher").to_string_lossy().into(),
        "-np".into(),
        "16".into(),
    ];
    cfg.programs.epw.args.extend(["-nk".into(), "16".into()]);
    fs::write(&input, toml::to_string_pretty(&cfg).unwrap()).unwrap();
    c.put("fail-coarse", "test failure");
    c.accounting(Some(201));
    success(c.cli(0, &["-s"], "mpi-phonon-submit"));
    success(c.bash(
        c.workflow()["script"]["path"].as_str().unwrap(),
        Some(201),
        "mpi-phonon",
    ));
    let calc = PathBuf::from(c.workflow()["calculation_dir"].as_str().unwrap());
    let fixed = store::Identity::capture(&c.root.join("run/input-init.toml")).unwrap();
    let binding = store::Identity::capture(&c.root.join("run/run-binding.json")).unwrap();
    let original = config::Config::load(&fixed.path).unwrap();
    let mut adjusted = original.clone();
    epwflow::foundation::parallelism::apply(&mut adjusted, 8).unwrap();
    let (identity, _) =
        epwflow::foundation::parallelism::request_identity(&adjusted, &adjusted.programs.launcher)
            .unwrap();
    assert_eq!(
        serde_json::to_value(&identity).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    adjusted.qe.nscf.nbnd += 1;
    assert!(epwflow::foundation::parallelism::request_identity(
        &adjusted,
        &adjusted.programs.launcher
    )
    .is_err());
    c.accounting(Some(202));
    success(c.cli(1, &["-s"], "mpi-wannier-submit"));
    assert!(!c
        .bash(
            c.workflow()["script"]["path"].as_str().unwrap(),
            Some(202),
            "mpi-coarse-fail"
        )
        .status
        .success());
    assert_eq!(c.workflow()["current_stage"], "wannier");
    assert_eq!(c.workflow()["state"], "failed");
    let qe_completion = c.workflow()["completed"]["qe-data"].clone();
    let references =
        store::Identity::capture(&calc.join("quality-sources/reference.json")).unwrap();
    let before = c.calls().0;
    let submissions = fs::read_to_string(c.root.join("submissions")).unwrap();
    for mpi in ["0", "9", "65"] {
        assert!(!c
            .cli(
                1,
                &["--resume", "-w", "run", "--mpi", mpi, "-s"],
                "mpi-invalid"
            )
            .status
            .success());
    }
    assert!(!c
        .cli(
            1,
            &["--resume", "-w", "run", "--mpi", "8", "-s"],
            "mpi-active-old-job"
        )
        .status
        .success());
    assert_eq!(
        submissions,
        fs::read_to_string(c.root.join("submissions")).unwrap()
    );
    c.accounting(Some(203));
    fs::remove_file(c.root.join("fail-coarse")).unwrap();
    success(c.cli(
        1,
        &["--resume", "-w", "run", "--mpi", "8", "-s"],
        "mpi-resume-submit",
    ));
    assert_eq!(c.workflow()["mpi_override"], 8);
    assert_eq!(c.workflow()["calculation_dir"], calc.to_str().unwrap());
    assert!(fs::read_to_string(c.root.join("submissions"))
        .unwrap()
        .lines()
        .last()
        .unwrap()
        .contains("--ntasks 8"));
    let active_record = fs::read(c.root.join("run/.epwflow/workflow.json")).unwrap();
    assert!(!c
        .cli(
            1,
            &["--resume", "-w", "run", "--mpi", "4", "-s"],
            "mpi-active-reject"
        )
        .status
        .success());
    assert_eq!(
        active_record,
        fs::read(c.root.join("run/.epwflow/workflow.json")).unwrap()
    );
    success(c.bash(
        c.workflow()["script"]["path"].as_str().unwrap(),
        Some(203),
        "mpi-coarse-resume",
    ));
    assert_eq!(c.workflow()["state"], "succeeded");
    assert_eq!(c.workflow()["completed"]["qe-data"], qe_completion);
    fixed.verify().unwrap();
    binding.verify().unwrap();
    references.verify().unwrap();
    for id in qe_completion["artifacts"].as_array().unwrap() {
        serde_json::from_value::<store::Identity>(id.clone())
            .unwrap()
            .verify()
            .unwrap();
    }
    let after = c.calls().0;
    for role in ["ph", "dynmat"] {
        assert_eq!(
            before.lines().filter(|l| l.starts_with(role)).count(),
            after.lines().filter(|l| l.starts_with(role)).count()
        );
    }
    assert_eq!(
        after.lines().filter(|l| *l == "pw").count(),
        before.lines().filter(|l| *l == "pw").count() + 1,
        "仅新增比较路径 NSCF；不得重做主链 SCF/NSCF 或独立电子参考"
    );
    assert!(fs::read_to_string(c.root.join("mpi-launches"))
        .unwrap()
        .lines()
        .any(|l| l.starts_with("-np 8 ") && l.contains("-nk 8") && l.contains("epw_coarse.in")));
    assert!(fs::read_dir(calc.join("native-attempts/history"))
        .unwrap()
        .any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("coarse-")));
    c.accounting(Some(204));
    success(c.cli(2, &["--resume", "-w", "run", "-s"], "mpi-third-inherits"));
    assert_eq!(c.workflow()["mpi_override"], 8);
    assert!(fs::read_to_string(c.root.join("submissions"))
        .unwrap()
        .lines()
        .last()
        .unwrap()
        .contains("--ntasks 8"));
    assert!(!c
        .cli(
            1,
            &["--resume", "-w", "run", "--mpi", "4", "-s"],
            "mpi-complete-coarse-reject"
        )
        .status
        .success());
}

#[test]
fn public_chain_resumes_partial_solver_then_only_unfinished_delivery() {
    let c = Case::new();
    success(c.cli(0, &["--all"], "generate"));
    let generated = c.root.join("run/.epwflow/generated");
    success(c.bash(generated.join("submit-all.sh"), None, "submit-chain"));
    assert_eq!(
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count(),
        3
    );
    assert!(!c.root.join("fixture-ledger.json").exists());
    c.accounting(Some(201));
    success(c.bash(generated.join("qe-data.sh"), Some(201), "phonon"));
    let calc = PathBuf::from(c.workflow()["calculation_dir"].as_str().unwrap());
    assert!(calc.join("qe-data/manifest.json").is_file());
    assert!(calc.join("scf/snapshot-files.json").is_file());
    let scf = store::Identity::capture(&calc.join("scf/snapshot-files.json")).unwrap();
    assert_eq!(c.workflow()["completed"].as_object().unwrap().len(), 1);
    c.accounting(Some(202));
    success(c.bash(generated.join("interpolation.sh"), Some(202), "wannier"));
    assert_eq!(calc.to_str().unwrap(), c.workflow()["calculation_dir"]);
    let coarse = store::Identity::capture(&calc.join("wannier/manifest.json")).unwrap();
    assert!(epwflow::delivery::comparison_plot::check_figure_formats(
        &calc.join("results"),
        &["png", "svg", "pdf"]
    )
    .unwrap());
    assert!(!calc.join("interpolation/manifest.json").exists());
    assert!(!calc.join("superconductivity").exists());
    assert!(!c.calls().1.lines().any(|l| l.starts_with("fine ")));
    // 最终输出发生可恢复的用户文件冲突；生产代码必须保留该文件并拒绝覆盖。
    let conflict = c.root.join("run/results/figures/a2f_coupling.svg");
    fs::create_dir_all(conflict.parent().unwrap()).unwrap();
    fs::write(&conflict, "test-owned conflict").unwrap();
    c.accounting(Some(203));
    c.put("tc-failure", "test-owned transient failure");
    let o = c.bash(generated.join("all.sh"), Some(203), "partial-solver");
    assert!(!o.status.success());
    assert_eq!(c.workflow()["current_stage"], "superconductivity");
    assert!(c.workflow()["completed"].get("superconductivity").is_none());
    let fine = store::Identity::capture(&calc.join("interpolation/manifest.json")).unwrap();
    let partial_path = calc.join("superconductivity/manifest.json");
    let partial: Value = store::read(&partial_path).unwrap();
    assert_eq!(partial["converged"], false);
    assert_eq!(partial["native_modes"]["iso"]["complete"], false);
    assert_eq!(partial["native_modes"]["aniso"]["complete"], true);
    let failed_iso = partial["native_modes"]["iso"]["attempts"]
        .as_array()
        .unwrap();
    assert_eq!(failed_iso.len(), 1);
    assert_eq!(failed_iso[0]["status"], "failed");
    let mut retained = vec![];
    for mode in ["iso", "aniso"] {
        for a in partial["native_modes"][mode]["attempts"]
            .as_array()
            .unwrap()
        {
            let id: store::Identity = serde_json::from_value(a["record"].clone()).unwrap();
            retained.push(id);
        }
    }
    // 通过实际生产者发布的 partial 重验，检查本身不得启动进程或修补状态。
    let cfg = config::Config::load(&c.root.join("inputepw.toml")).unwrap();
    let run_id = partial["run_id"].as_str().unwrap();
    let inspect = || epwflow::superconductivity::verify::published_complete(&calc, &cfg, run_id);
    assert!(!inspect().unwrap());
    assert!(epwflow::superconductivity::verify::complete(&calc, &cfg, run_id).is_err());
    assert!(
        epwflow::superconductivity::verify::published_complete(&calc, &cfg, "wrong-run").is_err()
    );
    let mut changed_cfg = cfg.clone();
    changed_cfg.superconductivity.max_iterations += 1;
    assert!(
        epwflow::superconductivity::verify::published_complete(&calc, &changed_cfg, run_id)
            .is_err()
    );
    let original_manifest = fs::read(&partial_path).unwrap();
    let mut missing_mode = partial.clone();
    missing_mode["native_modes"]
        .as_object_mut()
        .unwrap()
        .remove("aniso");
    store::write(&partial_path, &missing_mode).unwrap();
    assert!(inspect().is_err());
    fs::write(&partial_path, &original_manifest).unwrap();
    for path in [
        calc.join("superconductivity/temperatures.csv"),
        fine.path.clone(),
    ] {
        let original = fs::read(&path).unwrap();
        fs::write(&path, "tampered").unwrap();
        assert!(inspect().is_err());
        fs::write(&path, original).unwrap();
    }
    let search_path = calc.join("superconductivity/iso/search.json");
    let original_search = fs::read(&search_path).unwrap();
    fs::remove_file(&search_path).unwrap();
    assert!(inspect().is_err());
    assert!(!search_path.exists(), "只读检查不得重建缺失的搜索文件");
    fs::write(&search_path, &original_search).unwrap();
    // 预算拒绝等前置错误可产生零尝试；仍由当前 F3 原生角色核验其绑定。
    let f3 = epwflow::interpolation::manifest::InterpolationManifest::load(&fine.path).unwrap();
    let source = epwflow::superconductivity::source::bind(&calc, &cfg, &f3).unwrap();
    let empty_root = c.root.join("empty-search-binding");
    let empty = epwflow::superconductivity::engine::Engine::open(
        &cfg,
        &source,
        &cfg.programs.launcher,
        &empty_root,
        config::Mode::Iso,
    )
    .unwrap()
    .result;
    let saved_outputs: Vec<_> = ["manifest.json", "tc.json", "temperatures.csv"]
        .into_iter()
        .map(|name| {
            let path = calc.join("superconductivity").join(name);
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    let mut modes: std::collections::BTreeMap<
        String,
        epwflow::superconductivity::engine::ModeResult,
    > = serde_json::from_value(partial["native_modes"].clone()).unwrap();
    store::write(&search_path, &empty).unwrap();
    modes.insert("iso".into(), empty);
    epwflow::superconductivity::scheduler::publish(
        &calc.join("superconductivity"),
        run_id.into(),
        fine.sha256.clone(),
        config::Mode::Both,
        modes,
    )
    .unwrap();
    assert!(!inspect().unwrap());
    fs::write(&search_path, &original_search).unwrap();
    for (path, bytes) in saved_outputs {
        fs::write(path, bytes).unwrap();
    }
    assert!(!inspect().unwrap());
    let partial_calls = c.calls();
    fs::remove_file(c.root.join("tc-failure")).unwrap();
    c.accounting(None);
    success(c.cli(2, &["--resume", "-w", "run", "-s"], "partial-resume-submit"));
    let retry = PathBuf::from(c.workflow()["script"]["path"].as_str().unwrap());
    c.accounting(Some(204));
    let o = c.bash(retry, Some(204), "superconductivity-conflict");
    assert!(!o.status.success());
    let failed = c.workflow();
    assert_eq!(
        failed["current_stage"],
        "results",
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert_eq!(failed["state"], "failed");
    assert!(failed["completed"].get("superconductivity").is_some());
    assert!(failed["completed"].get("results").is_none());
    assert_eq!(
        fs::read_to_string(&conflict).unwrap(),
        "test-owned conflict"
    );
    assert!(c.root.join("run/results/manifest.json").exists());
    let before = c.calls();
    assert!(before.1.lines().any(|l| l.starts_with("fine ")));
    fine.verify().unwrap();
    let superconductivity =
        store::Identity::capture(&calc.join("superconductivity/manifest.json")).unwrap();
    let f4: Value = store::read(&superconductivity.path).unwrap();
    assert_eq!(f4["converged"], true);
    assert_eq!(partial_calls.0, before.0, "SCF／PH 不得重复");
    assert_eq!(partial_calls.1, before.1, "粗／细网格不得重复");
    let aniso_calls = |s: &str| {
        s.lines()
            .filter(|l| serde_json::from_str::<Value>(l).unwrap()["mode"] == "aniso")
            .count()
    };
    assert_eq!(aniso_calls(&partial_calls.2), aniso_calls(&before.2));
    assert_eq!(
        f4["native_modes"]["aniso"],
        partial["native_modes"]["aniso"]
    );
    let iso_attempts = f4["native_modes"]["iso"]["attempts"].as_array().unwrap();
    assert!(iso_attempts.len() > failed_iso.len());
    assert_eq!(&iso_attempts[..failed_iso.len()], failed_iso);
    for id in &retained {
        id.verify().unwrap();
    }
    assert_eq!(f4["native_modes"].as_object().unwrap().len(), 2);
    for (mode, boundary) in [("iso", 3200), ("aniso", 3600)] {
        let search = &f4["native_modes"][mode]["search"];
        let interval = search["interval_centikelvin"].as_array().unwrap();
        assert_eq!(search["status"], "located");
        assert!(
            interval[0].as_i64().unwrap() < boundary && interval[1].as_i64().unwrap() >= boundary
        );
        assert!(interval[1].as_i64().unwrap() - interval[0].as_i64().unwrap() <= 50);
    }
    // 仅移除本测试创建的冲突。所有科学数据由生产恢复链重新验证。
    fs::remove_file(&conflict).unwrap();
    // 模拟原分配被终止后遗留的 F5 锁；只能凭该分配的终态证据回收。
    let final_locks = [
        "native-results.lock",
        "comparison.lock",
        "figures/native-figures.lock",
        "figures/comparison-figures.lock",
    ];
    for name in final_locks {
        let lock = c.root.join("run/results").join(name);
        fs::create_dir(&lock).unwrap();
        store::write(
            &lock.join("owner.json"),
            &json!({
                "host":"fixture", "pid":u32::MAX, "token":"test-owned-final-lock",
                "created_unix":0, "job_id":204,
            }),
        )
        .unwrap();
    }
    c.accounting(None);
    success(c.cli(2, &["--resume", "-w", "run", "-s"], "resume-submit"));
    let script = PathBuf::from(c.workflow()["script"]["path"].as_str().unwrap());
    c.accounting(Some(205));
    success(c.bash(script, Some(205), "resume-delivery"));
    for name in final_locks {
        assert!(!c.root.join("run/results").join(name).exists());
    }
    assert_eq!(before, c.calls(), "恢复交付不能重做科学执行");
    for id in [&scf, &coarse, &fine, &superconductivity] {
        id.verify().unwrap();
    }
    assert_eq!(calc.to_str().unwrap(), c.workflow()["calculation_dir"]);
    assert_eq!(c.workflow()["completed"].as_object().unwrap().len(), 5);
    c.accounting(None);
    let o = c.cli(
        2,
        &["--status", "-w", "run", "--refresh", "--json"],
        "status",
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let status: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(status["workflow_complete"], true);
    let results = c.root.join("run/results");
    assert_eq!(
        epwflow::delivery::check_results(&results).unwrap()["workflow_complete"],
        true
    );
    for name in epwflow::delivery::native_plot::KINDS
        .iter()
        .chain(epwflow::delivery::comparison::KINDS.iter())
    {
        for format in ["png", "svg", "pdf"] {
            assert!(results
                .join("figures")
                .join(format!("{name}.{format}"))
                .is_file());
        }
    }
    let ledger: Value = store::read(&c.root.join("fixture-ledger.json")).unwrap();
    assert_eq!(ledger["spent_node_seconds"], 5);
    assert_eq!(ledger["allocations"].as_array().unwrap().len(), 0);
    assert_eq!(ledger["reservations"].as_array().unwrap().len(), 0);
    assert_eq!(
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count(),
        5
    );
    // 复制到新位置后独立重绘；保留原包的工作流完成证据，篡改副本必须拒绝。
    let moved = c.root.join("moved-results");
    copy_results(&results, &moved);
    success(c.cli(
        2,
        &[
            "--plot",
            "--results",
            moved.to_str().unwrap(),
            "--formats",
            "svg",
        ],
        "moved-plot",
    ));
    success(c.cli(
        2,
        &[
            "--results-check",
            "--results",
            moved.to_str().unwrap(),
            "--json",
        ],
        "moved-check",
    ));
    let image = moved.join("figures/a2f_coupling.svg");
    let original = fs::read(&image).unwrap();
    fs::write(&image, "tampered").unwrap();
    assert!(!c
        .cli(
            2,
            &[
                "--results-check",
                "--results",
                moved.to_str().unwrap(),
                "--json"
            ],
            "tampered-check"
        )
        .status
        .success());
    fs::write(&image, original).unwrap();
    assert_eq!(before, c.calls());
    store::write(
        &c.root.join("verification.json"),
        &json!({"kind":"synthetic process contract","scientific_validation":false}),
    )
    .unwrap();
}

// 0018：编号工作区协议；替身仍仅验证 IO、命令、依赖与复用，不作科学验收。
impl Case {
    fn numbered() -> Self {
        let c=Self::new();
        let mut cfg=config::Config::load(&c.root.join("inputepw.toml")).unwrap();
        cfg.layout=Some("numbered".into());cfg.resources.mpi=1;
        cfg.programs.epw.args.extend(["-nk".into(),"1".into()]);
        c.put("input.toml",&toml::to_string_pretty(&cfg).unwrap());
        let qe=include_str!("fixtures/qe_native_io.py").replace("nscf=value('calculation')=='nscf'","nscf=value('calculation') in ('nscf','bands')")
            .replace("nat = len(ET.parse(Path('out')/(prefix+'.save')/'data-file-schema.xml').getroot().findall('output/atomic_structure/atomic_positions/atom'))", "nat = int(ET.parse(next(Path.cwd().glob('*.dyn*.xml'))).getroot().find('GEOMETRY_INFO/NUMBER_OF_ATOMS').text)");
        c.put("qe_native_io.py",&qe);
        let epw=include_str!("fixtures/epw_native_io.py")
            .replace("coarse=value('epwread')=='.false.'","coarse=value('epwread')=='.false.'\nwannier_only='elph = .false.' in text\nreuse=coarse and 'wannierize = .false.' in text")
            .replace("('coarse' if coarse else", "('wannier' if wannier_only else 'coarse' if coarse else")
            .replace("assert (Path(value('dvscf_dir'))/(prefix+'.dvscf_q2')).is_file(), 'PH 未交付'", "assert wannier_only or (Path(value('dvscf_dir'))/(prefix+'.dvscf_q2')).is_file(), 'PH 未交付'")
            .replace("kpoints=[cart(list(map(float,l.split()[:3]))) for l in Path('kpoints.dat').read_text().splitlines()[1:]]", "kpoints=kpoints if wannier_only else [cart(list(map(float,l.split()[:3]))) for l in Path('kpoints.dat').read_text().splitlines()[1:]]")
            .replace("qcr=[list(map(float,l.split()[:3])) for l in Path('qpoints.dat').read_text().splitlines()[1:]]", "qcr=[[0,0,0]] if wannier_only else [list(map(float,l.split()[:3])) for l in Path('qpoints.dat').read_text().splitlines()[1:]]")
            .replace("if coarse:\n        lower=window_min()", "if coarse and not reuse:\n        lower=window_min()")
            .replace("Path(prefix+'.wout').write_text(wout)","if not reuse:Path(prefix+'.wout').write_text(wout)\n    if wannier_only:\n        Path(prefix+'.win').write_text('num_wann = '+str(n)+'\\n')\n        Path(prefix+'.chk').write_text('fixture checkpoint')")
            .replace("parent/'nscf/out'", "parent/'2.qe-nscf/out'")
            .replace("ROOT/'run/nscf/out'", "ROOT/'run/2.qe-nscf/out'");
        c.put("epw_native_io.py",&epw);
        for kind in ["bands","dos","wannier90","postw90"] {
            c.script(&format!("{kind}.x"),&format!(r#"#!/usr/bin/python3
from pathlib import Path
import re,sys,xml.etree.ElementTree as ET
kind={kind:?}
if kind in ['bands','dos']:
    text=Path(sys.argv[sys.argv.index('-inp')+1]).read_text()
    prefix=re.search(r"prefix\s*=\s*'([^']+)'",text).group(1)
    xml=ET.parse(Path('out')/(prefix+'.save')/'data-file-schema.xml').getroot()
    ks=xml.findall('output/band_structure/ks_energies')
    if kind=='bands':
        Path(prefix+'_band').write_text('fixture band file')
        Path(prefix+'_band.gnu').write_text(''.join('0 '+str(i)+'\n1 '+str(i)+'\n\n' for i in range(16)))
    else:
        Path(prefix+'.tdos').write_text('\n'.join(str(i/10)+' 1' for i in range(-100,100)))
    print('JOB DONE.')
elif kind=='wannier90':
    prefix=sys.argv[1]
    assert 'restart = plot' in Path(prefix+'.win').read_text()
    Path(prefix+'_band.dat').write_text(''.join('0 '+str(i)+'\n1 '+str(i)+'\n\n' for i in range(5)))
    Path(prefix+'_band.labelinfo.dat').write_text('G 1 0\nX 2 1\n')
else:
    prefix=sys.argv[1]
    Path(prefix+'_dos.dat').write_text('\n'.join(str(i/10)+' 1' for i in range(-100,100)))
"#));
        }
        c
    }
    fn numbered_cli(&self,i:usize,args:&[&str],job:Option<u64>,label:&str)->Output {
        let mut cmd=Command::new(BINS[i]);cmd.args(["-c",self.root.join("input.toml").to_str().unwrap()]).args(args);
        self.environment(&mut cmd,job);if job.is_some(){let cfg=config::Config::load(&self.root.join("input.toml")).unwrap();cmd.env("SLURM_NTASKS",cfg.resources.mpi.to_string());}
        let o=cmd.output().unwrap();self.put(&format!("{label}.stderr"),&String::from_utf8_lossy(&o.stderr));self.put(&format!("{label}.stdout"),&String::from_utf8_lossy(&o.stdout));o
    }
    fn numbered_change(&self,change:impl FnOnce(&mut config::Config)) {
        let p=self.root.join("input.toml");let mut cfg=config::Config::load(&p).unwrap();change(&mut cfg);fs::write(p,toml::to_string_pretty(&cfg).unwrap()).unwrap();
    }
}

#[test]
fn numbered_scripts_leave_shell_options_unset_and_stop_on_environment_failure() {
    let c = Case::numbered();
    let environment = c.root.join("environment with spaces.sh");
    c.numbered_change(|cfg| cfg.programs.environment_script = Some(environment.clone()));
    fs::write(&environment, "export EPW_TEST_ENV=\"${EPW_TEST_OPTIONAL}ready\"\n").unwrap();
    success(c.numbered_cli(0, &[], None, "generate-with-environment"));
    let work = c.root.join("run");
    let mut checked = 0;
    for entry in epwflow::qe_data::save::files(&work).unwrap() {
        if !entry.relative_path.ends_with("submit.sh") { continue; }
        let script = fs::read_to_string(work.join(&entry.relative_path)).unwrap();
        let (prefix, _) = script.rsplit_once("\nexec ").unwrap();
        let run = |suffix: &str| Command::new("/bin/bash")
            .args(["-c", &format!("{prefix}\n{suffix}")])
            .env_remove("EPW_TEST_OPTIONAL")
            .output().unwrap();
        let out = run("[[ $- != *u* && $- != *e* ]] || exit 1\nif shopt -qo pipefail; then exit 1; fi\nprintf '%s' \"$EPW_TEST_ENV\"");
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(out.stdout, b"ready");
        fs::write(&environment, "return 17\n").unwrap();
        let out = run("printf should-not-run");
        assert_eq!(out.status.code(), Some(17));
        assert!(out.stdout.is_empty());
        fs::write(&environment, "export EPW_TEST_ENV=\"${EPW_TEST_OPTIONAL}ready\"\n").unwrap();
        checked += 1;
    }
    assert!(checked > 0);
}

#[test]
fn numbered_public_qe_wannier_coarse_and_fine_are_distinct_and_reusable() {
    use epwflow::foundation::{stages::{self,Step},layout};
    let c=Case::numbered();c.accounting(Some(201));
    success(c.numbered_cli(0,&["-r"],Some(201),"numbered-qe"));
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    for step in [Step::Scf,Step::Nscf,Step::Electronic,Step::Phonon] {assert!(stages::reusable(&cfg,step).unwrap());}
    assert!(c.root.join("run/1.qe-scf/scf.in").is_file());
    assert!(c.root.join("run/4.qe-phonon-nosplit/save/scf.dvscf_q2").is_file());
    let qe_calls=c.calls().0;
    c.accounting(Some(202));
    success(c.numbered_cli(1,&["--stage","wannier","-r"],Some(202),"numbered-wannier"));
    assert!(!c.root.join("run/6.epw-coarse/epw.out").exists());
    let wan_hash=store::hash_file(&c.root.join("run/5.epw-wannier/scf.ukk")).unwrap();
    let key=stages::scientific_key(&cfg,Step::Wannier).unwrap();
    c.accounting(Some(203));
    success(c.numbered_cli(1,&["--stage","coarse","--resume","-r"],Some(203),"numbered-coarse"));
    assert_eq!(c.calls().0.lines().filter(|s|*s=="pw").count(),qe_calls.lines().filter(|s|*s=="pw").count()+2); // 仅独立电子参考与路径参考。
    assert_eq!(wan_hash,store::hash_file(&c.root.join("run/5.epw-wannier/scf.ukk")).unwrap());
    assert!(fs::read_to_string(c.root.join("run/6.epw-coarse/epw_coarse.in")).unwrap().contains("wannierize = .false."));
    assert_eq!(stages::scientific_key(&cfg,Step::Wannier).unwrap(),key);
    assert!(layout::path(&cfg.work_dir,"wannier/manifest.json").is_file());
    c.accounting(Some(204));
    success(c.numbered_cli(2,&["--stage","fine","-r"],Some(204),"numbered-fine"));
    let calls=c.calls();
    c.numbered_change(|cfg|cfg.interpolation.q_grid=[4;3]);
    c.accounting(Some(205));
    success(c.numbered_cli(2,&["--stage","fine","-r"],Some(205),"numbered-fine-new-grid"));
    assert_eq!(c.calls().0,calls.0,"细网格变化不能执行 QE");
    assert_eq!(c.calls().1.matches("wannier ").count(),1,"细网格变化不能重复拟合");
    assert!(c.root.join("run/7.epw-fine/smearing-0001/submit.sh").is_file());
    let before=c.calls();
    c.script("launcher","#!/bin/sh\nshift 2\nexec \"$@\"\n");
    c.numbered_change(|cfg|{cfg.resources.mpi=2;cfg.programs.launcher=vec![c.root.join("launcher").to_string_lossy().into(),"-np".into(),"2".into()];cfg.programs.epw.args[2]="2".into();});
    c.accounting(None);
    success(c.numbered_cli(1,&["--stage","coarse","-s"],None,"numbered-resource-reuse"));
    assert_eq!(before,c.calls(),"改资源不能重算已有科学结果");
    c.numbered_change(|cfg|cfg.comparison.as_mut().unwrap().electronic_sigma_ev=0.15);
    c.accounting(Some(206));
    success(c.numbered_cli(1,&["--stage","coarse","-r"],Some(206),"numbered-post-only"));
    assert_eq!(before,c.calls(),"改展宽只能运行原生后处理和重绘");
    assert!(stages::reusable(&config::Config::load(&c.root.join("input.toml")).unwrap(),Step::Fine).unwrap());
    c.numbered_change(|cfg|cfg.comparison.as_mut().unwrap().electronic_sigma_ev=0.1);c.accounting(Some(211));
    success(c.numbered_cli(1,&["--stage","coarse","-r"],Some(211),"numbered-post-revert"));assert_eq!(before,c.calls());
    let post:Value=store::read(&c.root.join("run/.epwflow/post/coarse-sources.json")).unwrap();assert_eq!(post["electronic_sigma_ev"],0.1);
    c.accounting(Some(207));
    c.put("tc-failure","test failure");
    let failed=c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(207),"numbered-superconductivity-partial");assert!(!failed.status.success());
    let partial:Value=store::read(&c.root.join("run/8.epw-superconductivity/manifest.json")).unwrap();assert_ne!(partial["validation"]["status"],"valid");
    c.numbered_change(|cfg|{cfg.resources.mpi=3;cfg.programs.launcher[2]="3".into();cfg.programs.epw.args[2]="3".into();});
    fs::remove_file(c.root.join("tc-failure")).unwrap();c.accounting(Some(210));
    success(c.numbered_cli(2,&["--stage","superconductivity","--resume","-r"],Some(210),"numbered-superconductivity"));
    assert_eq!(before.0,c.calls().0);assert!(!c.calls().2.is_empty());
    let completed:Value=store::read(&c.root.join("run/8.epw-superconductivity/manifest.json")).unwrap();assert_eq!(partial["native_modes"]["aniso"],completed["native_modes"]["aniso"]);
    let coarse_calls=c.calls().1.matches("coarse ").count();
    c.numbered_change(|cfg|cfg.comparison.as_mut().unwrap().points_per_edge=3);c.accounting(Some(212));
    success(c.numbered_cli(1,&["--stage","coarse","-r"],Some(212),"numbered-path-density"));
    assert_eq!(coarse_calls,c.calls().1.matches("coarse ").count(),"路径后处理不能重算 EPC");
    assert_eq!(c.calls().1.matches("wannier ").count(),1);
    assert!(stages::reusable(&config::Config::load(&c.root.join("input.toml")).unwrap(),Step::Fine).unwrap());
    let before=c.calls();
    c.numbered_change(|cfg|cfg.wannier.outer_window_ev.as_mut().unwrap()[0]=1.4);
    c.accounting(Some(208));
    success(c.numbered_cli(1,&["--stage","wannier","-r"],Some(208),"numbered-new-window"));
    assert_eq!(before.0,c.calls().0,"能窗变化不能重算 QE");
    assert_eq!(c.calls().1.matches("wannier ").count(),2);
    let wan=store::Identity::capture(&c.root.join("run/5.epw-wannier/scf.ukk")).unwrap();
    c.numbered_change(|cfg|cfg.phonon.q_grid=[2,1,2]);
    c.accounting(Some(209));
    success(c.numbered_cli(1,&["--stage","coarse","-r"],Some(209),"numbered-new-coarse-q"));
    wan.verify().unwrap();assert_eq!(c.calls().1.matches("wannier ").count(),2,"粗 q 不能重复拟合");
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    assert!(stages::reusable(&cfg,Step::Coarse).unwrap());assert!(!stages::reusable(&cfg,Step::Fine).unwrap());
    c.numbered_change(|cfg|{cfg.resources.mpi=16;cfg.programs.launcher[2]="16".into();cfg.programs.epw.args[2]="16".into();});
    success(c.numbered_cli(2,&["--stage","fine","--check"],None,"fine-resources-exceed-cached-coarse-grid"));
    // 每个真正计算的叶目录可直接找到提交脚本。
    for entry in epwflow::qe_data::save::files(&cfg.work_dir).unwrap() {
        if entry.relative_path.starts_with(".epwflow"){continue}
        let path=cfg.work_dir.join(&entry.relative_path);
        if path.extension().is_some_and(|e|e=="in") && path.file_name().unwrap()!="elepdos.in" {
            assert!(path.parent().unwrap().join("submit.sh").is_file(),"缺少脚本 {}",path.display());
        }
    }
}

#[test]
fn numbered_empty_pools_and_slurm_mismatch_stop_before_native_execution() {
    let c=Case::numbered();
    c.script("launcher","#!/bin/sh\nshift 2\nexec \"$@\"\n");
    c.numbered_change(|cfg|{cfg.qe.nscf.k_grid=[4,4,2];cfg.resources.mpi=64;cfg.programs.launcher=vec![c.root.join("launcher").to_string_lossy().into(),"-np".into(),"64".into()];cfg.programs.epw.args=vec!["--io-contract".into(),"-nk".into(),"64".into()];});
    let o=c.numbered_cli(1,&[],None,"numbered-empty-pool");assert!(!o.status.success());
    let error=String::from_utf8_lossy(&o.stderr);assert!(error.contains("k 点数=32")&&error.contains("pool=64")&&error.contains("空 pool"),"{error}");
    assert_eq!(c.calls().0,"");
    c.numbered_change(|cfg|{cfg.resources.mpi=32;cfg.programs.launcher[2]="32".into();cfg.programs.epw.args[2]="32".into();});
    success(c.numbered_cli(1,&[],None,"numbered-legal-generation"));
    c.accounting(Some(201));
    let o=c.bash(c.root.join("run/5.epw-wannier/submit.sh"),Some(201),"manual-slurm-mismatch");
    assert!(!o.status.success());assert_eq!(c.calls().0,"");
}

#[test]
fn numbered_submission_freezes_config_and_blocks_duplicate_queued_job() {
    let c=Case::numbered();c.accounting(Some(201));
    success(c.numbered_cli(0,&["-s"],None,"numbered-submit"));
    let active:Value=store::read(&c.root.join("run/.epwflow/active.json")).unwrap();
    let frozen=PathBuf::from(active["config"]["path"].as_str().unwrap());let before=fs::read(&frozen).unwrap();
    c.numbered_change(|cfg|cfg.resources.wall_seconds=300);
    let o=c.numbered_cli(0,&["-s","--resume"],None,"numbered-duplicate");assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("活动任务"));assert_eq!(fs::read(frozen).unwrap(),before);
    assert_eq!(fs::read_to_string(c.root.join("submissions")).unwrap().lines().count(),1);
    let script=PathBuf::from(active["script" ]["path"].as_str().unwrap());
    c.put("input.toml","invalid pending edits must not affect submitted job");
    let mut cmd=Command::new("/bin/bash");cmd.arg(script);c.environment(&mut cmd,Some(201));cmd.current_dir("/").env("SLURM_NTASKS","1");
    success(cmd.output().unwrap());
    let finished:Value=store::read(&c.root.join("run/.epwflow/active.json")).unwrap();assert_eq!(finished["state"],"succeeded");
}

#[test]
fn numbered_three_ph_modes_deliver_verified_save_and_local_scripts() {
    use epwflow::foundation::{stages::Step,config::PhMode};
    for mode in [PhMode::Nosplit,PhMode::Coords,PhMode::Index] {
        let c=Case::numbered();c.numbered_change(|cfg|cfg.phonon.mode=mode);c.accounting(Some(201));
        success(c.numbered_cli(0,&["-r"],Some(201),"three-ph-modes"));
        let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
        let ph=Step::Phonon.directory(&cfg);
        let m=epwflow::qe_data::manifest::QeManifest::load(&ph.join("save/manifest.json")).unwrap();m.verify().unwrap();
        assert_eq!(m.qpoints.len(),2);assert!(ph.join("save/scf.dvscf_q2").is_file());
        for q in m.qpoints {assert_eq!(q.local_index,if mode==PhMode::Coords{1}else{q.global_index});}
        if mode!=PhMode::Nosplit {assert!(ph.join("init/submit.sh").is_file());}
        assert_eq!(c.calls().0.lines().filter(|s|*s=="pw").count(),3,"SCF/NSCF/能带各一次，PH 不增加 SCF");
    }
}

#[test]
fn numbered_import_verifies_legacy_source_and_materializes_without_qe() {
    use epwflow::foundation::stages::{self,Step};
    let c=Case::numbered();c.accounting(Some(201));
    success(c.cli(0,&["-c","inputepw.toml","-s"],"legacy-source-submit"));
    success(c.bash(c.workflow()["script"]["path"].as_str().unwrap(),Some(201),"legacy-source-qe"));
    c.accounting(None);
    let source=c.root.join("run");let before=epwflow::qe_data::save::files(&source).unwrap();let calls=c.calls();
    c.numbered_change(|cfg|cfg.work_dir=c.root.join("numbered"));
    success(c.numbered_cli(0,&["--import-work",source.to_str().unwrap()],None,"numbered-import"));
    assert_eq!(before,epwflow::qe_data::save::files(&source).unwrap());assert_eq!(calls,c.calls());
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    for step in [Step::Scf,Step::Nscf,Step::Phonon] {assert!(stages::reusable(&cfg,step).unwrap());}
    assert!(Step::Phonon.directory(&cfg).join("scf.phdos.dat").is_file());
    success(c.numbered_cli(0,&["--import-work",source.to_str().unwrap()],None,"numbered-import-repeat"));
    assert_eq!(before,epwflow::qe_data::save::files(&source).unwrap());
}

#[test]
fn numbered_failed_coarse_resume_changes_resources_and_preserves_qe_and_references() {
    let c=Case::numbered();c.accounting(Some(201));
    success(c.numbered_cli(1,&["--stage","wannier","-r"],Some(201),"fit"));
    c.put("fail-coarse","test");c.accounting(Some(202));
    assert!(!c.numbered_cli(1,&["--stage","coarse","-r"],Some(202),"failed-coarse").status.success());
    let calls=c.calls().0;
    let reference=store::Identity::capture(&c.root.join("run/6.epw-coarse/quality-sources/reference.json")).unwrap();
    c.script("launcher","#!/bin/sh\nshift 2\nexec \"$@\"\n");
    c.numbered_change(|cfg|{cfg.resources.mpi=2;cfg.programs.launcher=vec![c.root.join("launcher").to_string_lossy().into(),"-np".into(),"2".into()];cfg.programs.epw.args[2]="2".into();});
    fs::remove_file(c.root.join("fail-coarse")).unwrap();c.accounting(Some(203));
    success(c.numbered_cli(1,&["--stage","coarse","--resume","-r"],Some(203),"coarse-recovered"));
    reference.verify().unwrap();assert_eq!(c.calls().0.lines().filter(|s|*s=="pw").count(),calls.lines().filter(|s|*s=="pw").count()+1,"只新增路径比较，已成功 QE 和独立参考保留");
}
