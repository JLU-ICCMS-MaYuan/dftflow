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
        c.script("squeue", "#!/bin/sh\nexit 0\n");
        c.script("sbatch", &format!("#!/bin/bash\nset -eu\nprintf '%s\\n' \"$*\" >> {calls}\nn=$(wc -l < {calls})\nprintf '%s\\n' \"$((200+n))\"\n", calls=shell_quote(c.root.join("submissions").to_str().unwrap())));
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
            &(201..=205)
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
