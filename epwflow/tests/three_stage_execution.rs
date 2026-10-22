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
        for kind in ["pw", "ph", "q2r", "matdyn"] {
            c.script(
                &format!("{kind}.x"),
                &format!(
                    "#!/bin/sh\nexec /usr/bin/python3 {} {kind} \"$@\"\n",
                    shell_quote(c.root.join("qe_native_io.py").to_str().unwrap())
                ),
            );
        }
        c.script("epw.x", &format!(r#"#!/usr/bin/python3
import io,pathlib,re,runpy,sys
root=pathlib.Path({root})
text=pathlib.Path(sys.argv[sys.argv.index('-inp')+1]).read_text() if '-inp' in sys.argv else sys.stdin.read()
sys.stdin=io.StringIO(text)
if (root/'fail-coarse').exists() and re.search(r'(?m)^\s*epwread\s*=\s*\.false\.',text) and not re.search(r'(?m)^\s*wannierize\s*=\s*\.true\.',text):
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
    fn allocated_cli(&self, i: usize, args: &[&str], job: u64, label: &str) -> Output {
        // 历史 -r 只接受已登记分配；通过真实预算 CLI 登记本测试的手工分配。
        let cfg=config::Config::load(&self.root.join("inputepw.toml")).unwrap();
        let ledger=cfg.budget.ledger.to_string_lossy().into_owned();
        let reserve=self.cli(0,&["--budget","reserve","--ledger",&ledger,"--label",label,
            "--nodes",&cfg.resources.nodes.to_string(),"--mpi",&cfg.resources.mpi.to_string(),
            "--wall-seconds",&cfg.resources.wall_seconds.to_string(),"--storage-bytes",&cfg.resources.storage_bytes.to_string()],"manual-allocation-reserve");
        assert!(reserve.status.success(),"{}",String::from_utf8_lossy(&reserve.stderr));
        let text=String::from_utf8(reserve.stdout).unwrap();
        let token=text.trim().strip_prefix("reserved: token=").unwrap();
        success(self.cli(0,&["--budget","attach","--ledger",&ledger,"--token",token,"--job",&job.to_string()],"manual-allocation-attach"));
        let mut cmd = Command::new(BINS[i]);
        cmd.args(args);
        self.output(cmd, Some(job), label)
    }
    fn bash(&self, path: impl AsRef<std::path::Path>, job: Option<u64>, label: &str) -> Output {
        let mut cmd = Command::new("/bin/bash");
        cmd.arg(path.as_ref());
        self.output(cmd, job, label)
    }
    fn workflow(&self) -> Value {
        store::read(&self.root.join("run/epw-state/workflow.json")).unwrap()
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
fn legacy_submit_requires_explicit_migration_without_submitting_a_wrapper() {
    let c = Case::new();
    for bin in 0..3 {
        let out = c.cli(bin, &["-s"], "legacy-native-migration");
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("migration_required"));
    }
    assert!(!c.root.join("submissions").exists());
    assert_eq!(c.calls(), (String::new(), String::new(), String::new()));
}

#[test]
fn public_chain_resumes_partial_solver_then_only_unfinished_delivery() {
    let c = Case::new();
    success(c.cli(0, &["--all"], "generate"));
    let generated = c.root.join("run/epw-state/generated");
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
    c.accounting(Some(204));
    let o = c.allocated_cli(2, &["--resume", "-w", "run", "-r"], 204, "superconductivity-conflict");
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
    c.accounting(Some(205));
    success(c.allocated_cli(2, &["--resume", "-w", "run", "-r"], 205, "resume-delivery"));
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
        3
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
        // 日志归档用例预建目录；新默认生成只准备当前就绪任务。
        fs::create_dir_all(c.root.join("run/4.qe-phonon-nosplit")).unwrap();
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
            .replace("kept=list(range(8,17));nk=coarse_nk", "kept=list(range(8,17));nk=coarse_nk\n    if wannier_only:\n        Path(prefix+'.bvec').write_text('native bvec\\n'+str(nk)+' 1\\n'+'1 0 0 1\\n'*nk)\n        Path(prefix+'.mmn').write_text('(0,0)\\n'*(9*9*nk))\n    if reuse:\n        assert Path(prefix+'.bvec').read_text().splitlines()[1].split()==[str(nk),'1'], '粗网格缺同一拟合 bvec'\n        assert len(Path(prefix+'.mmn').read_text().splitlines())==9*9*nk, '粗网格缺同一拟合 mmn'")
            .replace("parent/'nscf/out'", "parent/'2.qe-nscf/out'")
            .replace("ROOT/'run/nscf/out'", "ROOT/'run/2.qe-nscf/out'");
        c.put("epw_native_io.py",&epw);
        for kind in ["bands","dos","wannier90","postw90"] {
            c.script(&format!("{kind}.x"),&format!(r#"#!/usr/bin/python3
from pathlib import Path
import re,sys,xml.etree.ElementTree as ET
kind={kind:?}
if kind in ['wannier90','postw90']:
    prefix=sys.argv[1]
    win=Path(prefix+'.win').read_text()
    mesh=re.search(r'(?m)^mp_grid\s*=\s*(\d+)\s+(\d+)\s+(\d+)',win)
    assert mesh, '独立程序必须有 mp_grid'
    for block in ['unit_cell_cart','atoms_cart','kpoints']:
        assert 'begin '+block in win and 'end '+block in win, block+' 缺失'
    points=re.search(r'begin kpoints\s*\n(.*?)end kpoints',win,re.S).group(1).strip().splitlines()
    assert len(points)==int(mesh[1])*int(mesh[2])*int(mesh[3]), '独立 k 点数量不符'
    assert re.search(r'(?m)^num_bands\s*=\s*9\s*$',win), '应使用排除后的 9 带'
    assert 'dos_smr_index' not in win and 'dos_smr_type = gauss' in win, '应使用原生输入关键字，不能写内部变量'
    eig=[line.split() for line in Path(prefix+'.eig').read_text().splitlines()]
    assert len(eig)==len(points)*9 and all(int(row[0])==i%9+1 and int(row[1])==i//9+1 for i,row in enumerate(eig)), 'eig 必须绑定原 NSCF 点序和排除后带序'
    if kind=='wannier90' and (Path(__file__).parent/'post-failure').exists():
        Path(prefix+'.wout').write_text('overwritten failed plot log')
        Path(prefix+'.werr').write_text('Error: standalone post failed despite STOP zero')
        print('Error: standalone post failed despite STOP zero')
        sys.exit(0)
if kind in ['bands','dos']:
    text=Path(sys.argv[sys.argv.index('-inp')+1]).read_text() if '-inp' in sys.argv else sys.stdin.read()
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
    assert re.search(r'(?m)^write_bvec\s*=\s*\.false\.\s*$',win), '重绘不能重写已绑定的 bvec'
    Path(prefix+'_band.dat').write_text(''.join('0 '+str(i)+'\n1 '+str(i)+'\n\n' for i in range(5)))
    Path(prefix+'_band.labelinfo.dat').write_text('G 1 0\nX 2 1\n')
else:
    prefix=sys.argv[1]
    assert re.search(r'(?m)^num_elec_per_state\s*=\s*2\s*$',win), '非磁标量 DOS 应显式计入两重自旋'
    Path(prefix+'.wpout').write_text('Number of electrons per state : 2 |\n')
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
fn numbered_wannier_post_zero_exit_error_preserves_fit_and_resumes() {
    let c = Case::numbered();
    c.put("post-failure", "simulate serial Fortran STOP");
    c.accounting(Some(201));
    let failed = c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(201), "post-zero-failed");
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("standalone post failed"));
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    assert!(epwflow::foundation::stages::reusable(&cfg, epwflow::foundation::stages::Step::Wannier).unwrap());
    let receipt = fs::read(c.root.join("run/5.epw-wannier/completion.json")).unwrap();
    let wout = store::hash_file(&c.root.join("run/5.epw-wannier/scf.wout")).unwrap();
    let before = c.calls();
    fs::remove_file(c.root.join("post-failure")).unwrap();
    c.accounting(Some(202));
    success(c.numbered_cli(1, &["--stage", "wannier", "--resume", "-r"], Some(202), "post-zero-resume"));
    assert_eq!(before, c.calls(), "后处理续算不能重跑 QE 或 EPW 拟合");
    assert_eq!(receipt, fs::read(c.root.join("run/5.epw-wannier/completion.json")).unwrap());
    assert_eq!(wout, store::hash_file(&c.root.join("run/5.epw-wannier/scf.wout")).unwrap());
    assert!(!c.root.join("run/5.epw-wannier/scf.werr").exists());
    assert!(c.root.join("run/5.epw-wannier/electronic-bands.png").is_file());
    let norm: Value = store::read(&c.root.join("run/5.epw-wannier/electronic-dos-normalization.json")).unwrap();
    assert_eq!(norm["postw90_multiplier"], 1);
    assert_eq!(norm["postw90_num_elec_per_state"], 2);
    let csv = fs::read_to_string(c.root.join("run/5.epw-wannier/electronic-dos.csv")).unwrap();
    assert!(csv.lines().filter(|line| line.starts_with("Wannier,")).all(|line| line.rsplit(',').next().unwrap().parse::<f64>().unwrap() == 1.0),
        "原生 DOS 已计入自旋，CSV 不能再次放大");
}

#[test]
fn numbered_scripts_leave_shell_options_unset_and_source_environment_directly() {
    let c = Case::numbered();
    let environment = c.root.join("environment with spaces.sh");
    c.numbered_change(|cfg| cfg.programs.environment_script = Some(environment.clone()));
    fs::write(&environment, "export EPW_TEST_ENV=\"${EPW_TEST_OPTIONAL}ready\"\n").unwrap();
    success(c.numbered_cli(0, &[], None, "generate-with-environment"));
    let work = c.root.join("run");
    let mut checked = 0;
    for entry in epwflow::qe_data::save::files(&work).unwrap() {
        if entry.relative_path.extension().is_none_or(|ext| ext != "sh") { continue; }
        let script = fs::read_to_string(work.join(&entry.relative_path)).unwrap();
        let (prefix, invocation) = script.trim_end().rsplit_once('\n').unwrap();
        assert!(!invocation.starts_with("exec "));
        let run = |suffix: &str| Command::new("/bin/bash")
            .args(["-c", &format!("{prefix}\n{suffix}")])
            .env_remove("EPW_TEST_OPTIONAL")
            .output().unwrap();
        let out = run("[[ $- != *u* && $- != *e* ]] || exit 1\nif shopt -qo pipefail; then exit 1; fi\nprintf '%s' \"$EPW_TEST_ENV\"");
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(out.stdout, b"ready");
        fs::write(&environment, "return 17\n").unwrap();
        let out = run("printf source-returned");
        assert_eq!(out.status.code(), Some(0));
        assert_eq!(out.stdout, b"source-returned");
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
    assert_eq!(c.calls().0.lines().filter(|s|*s=="pw").count(),qe_calls.lines().filter(|s|*s=="pw").count()+1); // 仅常规 bands 路径，不再有抽样参考 NSCF。
    assert_eq!(wan_hash,store::hash_file(&c.root.join("run/5.epw-wannier/scf.ukk")).unwrap());
    for suffix in ["bvec", "mmn"] {
        let file = format!("scf.{suffix}");
        assert_eq!(store::hash_file(&c.root.join("run/5.epw-wannier").join(&file)).unwrap(),
            store::hash_file(&c.root.join("run/6.epw-coarse").join(&file)).unwrap());
    }
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
    let post:Value=store::read(&c.root.join("run/epw-state/post/coarse-sources.json")).unwrap();assert_eq!(post["electronic_sigma_ev"],0.1);
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
        if entry.relative_path.starts_with("epw-state"){continue}
        let path=cfg.work_dir.join(&entry.relative_path);
        if path.extension().is_some_and(|e|e=="in") && path.file_name().unwrap()!="elepdos.in" {
            assert!(path.parent().unwrap().join("submit.sh").is_file(),"缺少脚本 {}",path.display());
        }
    }
}

#[test]
fn numbered_large_differences_continue_to_native_fine_and_both_solvers_but_reject_damage() {
    use epwflow::foundation::{layout, stages::{self, Step}};
    let c = Case::numbered();
    c.put("epw-failure", "large-difference");
    c.accounting(Some(201));
    let coarse = c.numbered_cli(1, &["--stage", "coarse", "-r"], Some(201), "warning-coarse");
    assert!(!String::from_utf8_lossy(&coarse.stderr).contains("quality_warning:"));
    success(coarse);
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let manifest = layout::path(&cfg.work_dir, "wannier/manifest.json");
    let value: Value = store::read(&manifest).unwrap();
    assert!(value.get("quality").is_none());
    assert!(stages::reusable(&cfg, Step::Coarse).unwrap());
    assert!(!Step::Coarse.directory(&cfg).join("quality-sources").exists());
    assert!(!cfg.work_dir.join("quality-report.json").exists());
    let comparison = epwflow::interpolation::comparison_data::load(
        &layout::path(&cfg.work_dir, "interpolation/comparison"),
    ).unwrap();
    assert!(comparison.differences.unwrap().phonon_path.maximum.unwrap().absolute_difference > 50.);
    let calls = c.calls();
    c.accounting(Some(202));
    success(c.numbered_cli(2, &["--stage", "fine", "-r"], Some(202), "warning-fine"));
    assert_eq!(calls.0, c.calls().0, "续算不能重算 QE");
    let value: Value = store::read(&layout::path(&cfg.work_dir, "interpolation/manifest.json")).unwrap();
    assert!(value.get("quality").is_none());
    assert!(!value["spectra"].as_array().unwrap().is_empty());
    // 大差值可续算，篡改原生数据仍须拒绝。
    let band = Step::Coarse.directory(&cfg).join("epwdata.fmt");
    let saved = fs::read(&band).unwrap();
    fs::write(&band, "tampered").unwrap();
    let before_calls = c.calls();
    c.accounting(Some(203));
    assert!(!c.numbered_cli(2, &["--stage", "superconductivity", "-r"], Some(203), "warning-damaged").status.success());
    assert_eq!(before_calls, c.calls());
    fs::write(&band, saved).unwrap();
    c.accounting(Some(204));
    success(c.numbered_cli(2, &["--stage", "superconductivity", "--resume", "-r"], Some(204), "warning-both-solvers"));
    assert_eq!(calls.0, c.calls().0);
    let modes: Value = store::read(&Step::Superconductivity.directory(&cfg).join("manifest.json")).unwrap();
    assert!(modes["native_modes"].get("iso").is_some());
    assert!(modes["native_modes"].get("aniso").is_some());
    assert!(!c.calls().2.is_empty());
    let results = epwflow::delivery::check_results(
        &Step::Superconductivity.directory(&cfg).join("results"),
    ).unwrap();
    assert!(results.get("quality_status").is_none());
    assert!(results.get("quality_warning").is_none());
    assert!(results.get("differences").is_some());
    assert_eq!(results["workflow_complete"], true,
        "差值大小不改变执行完成状态");
}

#[test]
fn numbered_empty_pools_and_slurm_mismatch_stop_before_native_execution() {
    let c = Case::numbered();
    c.script("launcher", "#!/bin/sh\nshift 2\nexec \"$@\"\n");
    c.numbered_change(|cfg| {
        cfg.qe.nscf.k_grid=[4,4,2]; cfg.resources.mpi=64;
        cfg.programs.launcher=vec![c.root.join("launcher").to_string_lossy().into(),"-np".into(),"64".into()];
        cfg.programs.epw.args=vec!["--io-contract".into(),"-nk".into(),"64".into()];
    });
    let out=c.numbered_cli(1,&[],None,"numbered-empty-pool");
    assert!(!out.status.success());
    let error=String::from_utf8_lossy(&out.stderr);
    assert!(error.contains("k 点数=32") && error.contains("pool=64") && error.contains("空 pool"),"{error}");
    assert_eq!(c.calls().0, "");
    c.numbered_change(|cfg| { cfg.resources.mpi=32; cfg.programs.launcher[2]="32".into(); cfg.programs.epw.args[2]="32".into(); });
    success(c.numbered_cli(1,&[],None,"numbered-legal-generation"));
    let active:Value=store::read(&c.root.join("run/epw-state/native-active.json")).unwrap();
    let script=fs::read_to_string(active["script"]["path"].as_str().unwrap()).unwrap();
    assert!(script.contains("#SBATCH --ntasks=32"));
    assert!(script.contains("launcher") && script.contains("pw.x"));
    assert_eq!(c.calls().0, "");
}

#[test]
fn numbered_submission_freezes_config_and_blocks_duplicate_queued_job() {
    let c=Case::numbered(); c.direct_setup();
    success(c.numbered_cli(0,&["--stage","electronic","-s"],None,"numbered-submit"));
    let active=c.direct_task();
    let script=PathBuf::from(active["script"]["path"].as_str().unwrap());
    let before=fs::read(&script).unwrap();
    let control=store::Lock::acquire(&c.root.join("run/epw-state/control")).unwrap();
    let blocked=c.numbered_cli(0,&["--stage","electronic","-s"],None,"numbered-control-busy");
    assert!(!blocked.status.success());
    assert!(c.root.join("run/epw-state/control.lock/owner.json").is_file());
    drop(control);
    c.numbered_change(|cfg|cfg.resources.wall_seconds=300);
    assert!(!c.numbered_cli(0,&["--stage","electronic","-s","--resume"],None,"numbered-duplicate").status.success());
    assert_eq!(fs::read(&script).unwrap(),before);
    assert_eq!(fs::read_to_string(c.root.join("submissions")).unwrap().lines().count(),1);
    c.put("input.toml","invalid pending edits must not affect submitted native job");
    assert_eq!(c.direct_execute(),0);
    // 作业只执行 PW；正式完成状态必须等待公开 CLI 校验。
    assert_eq!(c.direct_task()["state"],"submitted");
}

#[test]
fn numbered_guiding_centres_invalidates_only_the_wannier_branch() {
    use epwflow::foundation::stages::{self, Step};
    let c = Case::numbered();
    let before = config::Config::load(&c.root.join("input.toml")).unwrap();
    let mut after = before.clone();
    after.wannier.guiding_centres = Some(true);
    for step in [Step::Scf, Step::Nscf, Step::Electronic, Step::Phonon] {
        assert_eq!(stages::scientific_key(&before, step).unwrap(), stages::scientific_key(&after, step).unwrap());
    }
    for step in [Step::Wannier, Step::Coarse, Step::Fine, Step::Superconductivity] {
        assert_ne!(stages::scientific_key(&before, step).unwrap(), stages::scientific_key(&after, step).unwrap());
    }
}

#[test]
fn numbered_phonon_post_retry_reuses_verified_responses_and_restores_dyn0() {
    let c = Case::numbered();
    c.put("post-failure", "q2r fails after PH collection");
    c.accounting(Some(201));
    let failed = c.numbered_cli(0, &["-r"], Some(201), "ph-post-failed");
    assert!(!failed.status.success());
    let ph = c.root.join("run/4.qe-phonon-nosplit");
    let saved = fs::read(ph.join("save/scf.dyn0.xml")).unwrap();
    assert!(!saved.is_empty());
    let before = c.calls();
    let manifest = fs::read(c.root.join("run/epw-state/qe-data/manifest.json")).unwrap();
    // 复现旧版本的自复制截断；已校验的归集副本仍可恢复原生后处理。
    fs::write(ph.join("scf.dyn0"), "").unwrap();
    fs::remove_file(c.root.join("post-failure")).unwrap();
    c.accounting(Some(202));
    success(c.numbered_cli(0, &["--resume", "-r"], Some(202), "ph-post-resume"));
    assert_eq!(saved, fs::read(ph.join("scf.dyn0")).unwrap());
    assert_eq!(manifest, fs::read(c.root.join("run/epw-state/qe-data/manifest.json")).unwrap());
    assert_eq!(before.0.lines().filter(|s| *s == "ph" || *s == "pw").collect::<Vec<_>>(),
        c.calls().0.lines().filter(|s| *s == "ph" || *s == "pw").collect::<Vec<_>>(),
        "后处理恢复不能重跑 PW/PH");
    assert!(ph.join("completion.json").is_file());
}

#[test]
fn numbered_phonon_partial_rejects_damaged_save_without_native_execution() {
    let c = Case::numbered();
    c.put("post-failure", "q2r failure");
    c.accounting(Some(201));
    assert!(!c.numbered_cli(0, &["-r"], Some(201), "ph-damage-initial").status.success());
    fs::write(c.root.join("run/4.qe-phonon-nosplit/save/scf.dvscf_q2"), "damaged").unwrap();
    let before = c.calls();
    c.accounting(Some(202));
    let out = c.numbered_cli(0, &["--resume", "-r"], Some(202), "ph-damage-resume");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("QE 产物变化"));
    assert_eq!(before, c.calls());
}

#[test]
fn numbered_stage_archive_preserves_the_open_slurm_log() {
    use std::io::Write;
    use std::os::unix::fs::MetadataExt;
    let c = Case::numbered();
    success(c.numbered_cli(0, &[], None, "log-prepare"));
    let path = c.root.join("run/4.qe-phonon-nosplit/slurm-201.out");
    let mut log = fs::File::create(&path).unwrap();
    log.write_all(b"before archive\n").unwrap();
    c.accounting(Some(201));
    success(c.numbered_cli(0, &["-r"], Some(201), "log-run"));
    log.write_all(b"after archive\n").unwrap();
    assert_eq!(log.metadata().unwrap().ino(), fs::metadata(&path).unwrap().ino());
    assert_eq!(fs::read_to_string(path).unwrap(), "before archive\nafter archive\n");
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
    // 清除夹具为日志测试预建的空目录，旧快照要求全新目标。
    fs::remove_dir_all(c.root.join("run")).unwrap();
    success(c.cli(0,&["-c","inputepw.toml"],"legacy-source-generate"));
    success(c.bash(c.root.join("run/epw-state/generated/qe-data.sh"),Some(201),"legacy-source-qe"));
    c.accounting(None);
    // 模拟真实历史契约：旧配置带质量小节，其字节指纹仍严格绑定。
    let historical = c.root.join("run/input-init.toml");
    let mut text = fs::read_to_string(&historical).unwrap();
    text.push_str("\n[interpolation.quality]\nreference_manifest = 'removed-reference.json'\nelectron_max_ev = 0.05\n");
    fs::write(&historical, text).unwrap();
    assert!(config::Config::load(&historical).is_err(), "普通配置不接受已删除的小节");
    let binding_path = c.root.join("run/run-binding.json");
    let mut binding: Value = store::read(&binding_path).unwrap();
    binding["config_sha256"] = store::hash_file(&historical).unwrap().into();
    let removed_program=c.root.join("historical-dynmat.x");
    fs::write(&removed_program,"historical program bytes").unwrap();
    binding["programs"]["dynmat"]=serde_json::to_value(store::Identity::capture(&removed_program).unwrap()).unwrap();
    fs::remove_file(&removed_program).unwrap();
    store::write(&binding_path, &binding).unwrap();
    fs::create_dir_all(c.root.join("run/quality-sources")).unwrap();
    c.put("run/quality-sources/unused-reference", "historical bytes");
    let source=c.root.join("run");let before=epwflow::qe_data::save::files(&source).unwrap();let calls=c.calls();
    c.numbered_change(|cfg|cfg.work_dir=c.root.join("numbered"));
    success(c.numbered_cli(0,&["--import-work",source.to_str().unwrap()],None,"numbered-import"));
    assert_eq!(before,epwflow::qe_data::save::files(&source).unwrap());assert_eq!(calls,c.calls());
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    for step in [Step::Scf,Step::Nscf,Step::Phonon] {assert!(stages::reusable(&cfg,step).unwrap());}
    assert!(Step::Phonon.directory(&cfg).join("scf.phdos.dat").is_file());
    assert!(!cfg.work_dir.join("quality-sources").exists());
    assert!(!cfg.work_dir.join("6.epw-coarse/quality-sources").exists());
    success(c.numbered_cli(0,&["--import-work",source.to_str().unwrap()],None,"numbered-import-repeat"));
    assert_eq!(before,epwflow::qe_data::save::files(&source).unwrap());
}

#[test]
fn numbered_failed_coarse_resume_changes_resources_and_preserves_qe() {
    let c=Case::numbered();c.accounting(Some(201));
    success(c.numbered_cli(1,&["--stage","wannier","-r"],Some(201),"fit"));
    c.put("fail-coarse","test");c.accounting(Some(202));
    assert!(!c.numbered_cli(1,&["--stage","coarse","-r"],Some(202),"failed-coarse").status.success());
    let calls=c.calls().0;
    let reference=store::Identity::capture(&c.root.join("run/epw-state/qe-data/manifest.json")).unwrap();
    c.script("launcher","#!/bin/sh\nshift 2\nexec \"$@\"\n");
    c.numbered_change(|cfg|{cfg.resources.mpi=2;cfg.programs.launcher=vec![c.root.join("launcher").to_string_lossy().into(),"-np".into(),"2".into()];cfg.programs.epw.args[2]="2".into();});
    fs::remove_file(c.root.join("fail-coarse")).unwrap();c.accounting(Some(203));
    success(c.numbered_cli(1,&["--stage","coarse","--resume","-r"],Some(203),"coarse-recovered"));
    reference.verify().unwrap();assert_eq!(c.calls().0.lines().filter(|s|*s=="pw").count(),calls.lines().filter(|s|*s=="pw").count()+1,"只新增路径比较，已成功 QE 保留");
}


// 0020：原生 bands 后处理失败与 PW 保存树的粒度恢复。
impl Case {
    fn fail_bands(&self) {
        let p = self.root.join("bands.x");
        let s = fs::read_to_string(&p).unwrap();
        let probe = format!(r#"
root=Path({root})
with (root/'electronic-calls').open('a') as f: f.write(kind+' '+str(sys.argv[1:])+'\n')
assert sys.argv[1:3]==['-pd','.true.'], 'bands 必须启用 pencil 分解'
assert not any(a in sys.argv for a in ['-nk','-npool']), '不得继承 PW/EPW pool'
if (root/'fail-bands').exists():
    print('Error in routine fft_type_set (6): there are processes with no planes',file=sys.stderr)
    sys.exit(6)
"#, root=serde_json::to_string(self.root.to_str().unwrap()).unwrap());
        self.script("bands.x", &s.replace("kind=\"bands\"", &format!("kind=\"bands\"\n{probe}")));
        let p=self.root.join("dos.x");let s=fs::read_to_string(p).unwrap();
        self.script("dos.x", &s.replace("kind=\"dos\"", "kind=\"dos\"\nassert sys.argv[1:3]==['-pd','.true.'] and '-nk' not in sys.argv"));
        self.put("fail-bands", "fail");
    }
    fn begin_failed_electronic(&self) {
        self.accounting(Some(201));
        let o=self.numbered_cli(1,&["--stage","wannier","-r"],Some(201),"failed-electronic");
        assert!(!o.status.success());
        assert!(String::from_utf8_lossy(&o.stderr).contains("QE bands 退出失败"),"{}",String::from_utf8_lossy(&o.stderr));
        assert!(self.root.join("run/3.qe-electronic/receipts/pw-bands.json").is_file());
    }
}

#[test]
fn numbered_electronic_retry_preserves_pw_with_32_mpi_and_then_changed_resources() {
    let c=Case::numbered();c.fail_bands();
    c.script("launcher", "#!/bin/sh\n[ \"$1\" = -np ] || exit 11\n[ \"$2\" = \"$SLURM_NTASKS\" ] || exit 12\nshift 2\nexec \"$@\"\n");
    c.numbered_change(|cfg|{
        cfg.qe.nscf.k_grid=[4,4,2]; cfg.resources.mpi=32;
        cfg.programs.launcher=vec![c.root.join("launcher").to_string_lossy().into(),"-np".into(),"32".into()];
        cfg.programs.epw.args[2]="32".into();
    });
    c.begin_failed_electronic();
    let before=c.calls();assert_eq!(before.0.lines().filter(|l|*l=="pw").count(),3);
    let save=c.root.join("run/3.qe-electronic/qe_eband/out");
    let saved=epwflow::qe_data::save::files(&save).unwrap();
    c.numbered_change(|cfg|{cfg.resources.mpi=16;cfg.programs.launcher[2]="16".into();cfg.programs.epw.args[2]="16".into();});
    fs::remove_file(c.root.join("fail-bands")).unwrap();c.accounting(Some(202));
    success(c.numbered_cli(1,&["--stage","wannier","--resume","-r"],Some(202),"electronic-recovered"));
    assert_eq!(before.0,c.calls().0,"恢复不得重跑 SCF／NSCF／PW bands");
    assert_eq!(saved,epwflow::qe_data::save::files(&save).unwrap());
    assert_eq!(fs::read_to_string(c.root.join("electronic-calls")).unwrap().lines().count(),2);
    let calls=c.calls();c.accounting(Some(203));
    success(c.numbered_cli(1,&["--stage","wannier","--resume","-r"],Some(203),"electronic-idempotent"));
    assert_eq!(calls,c.calls());
    c.numbered_change(|cfg|cfg.comparison.as_mut().unwrap().electronic_sigma_ev=0.12);c.accounting(Some(204));
    success(c.numbered_cli(1,&["--stage","wannier","--resume","-r"],Some(204),"electronic-dos-update"));
    assert_eq!(calls,c.calls(),"仅 DOS 设置不得重算 PW 或 Wannier");
    assert_eq!(fs::read_to_string(c.root.join("electronic-calls")).unwrap().lines().count(),2);
}

#[test]
fn numbered_electronic_receipt_rejects_corrupted_save_and_adopts_verified_old_pw() {
    let c=Case::numbered();c.fail_bands();c.begin_failed_electronic();
    let calls=c.calls();
    let prefix=config::Config::load(&c.root.join("input.toml")).unwrap().qe.prefix;
    let wfc=c.root.join(format!("run/3.qe-electronic/qe_eband/out/{prefix}.save/wfc1.dat"));
    let original=fs::read(&wfc).unwrap();fs::write(&wfc,"corrupted").unwrap();
    c.accounting(Some(202));
    let o=c.numbered_cli(1,&["--stage","wannier","--resume","-r"],Some(202),"corrupted-pw");
    assert!(!o.status.success());assert!(String::from_utf8_lossy(&o.stderr).contains("文件或链接目标已改变"));
    assert_eq!(calls,c.calls());fs::write(&wfc,original).unwrap();
    // 模拟 0018：有零退出码和冻结尝试，没有新原生任务收据。
    fs::remove_file(c.root.join("run/3.qe-electronic/receipts/pw-bands.json")).unwrap();
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    epwflow::foundation::workspace_electronic::inspect_legacy(&cfg).unwrap();
    let density=wfc.parent().unwrap().join("charge-density.dat");
    let original_density=fs::read(&density).unwrap();fs::write(&density,b"wrong density").unwrap();
    assert!(epwflow::foundation::workspace_electronic::inspect_legacy(&cfg).is_err());
    fs::write(density,original_density).unwrap();
    fs::remove_file(&wfc).unwrap();
    assert!(epwflow::foundation::workspace_electronic::inspect_legacy(&cfg).is_err());
    fs::write(&wfc,b"fixture-wfc").unwrap();
    let active:Value=store::read(&c.root.join("run/epw-state/active.json")).unwrap();
    let frozen=PathBuf::from(active["config"]["path"].as_str().unwrap()).parent().unwrap().join("POSCAR");
    let original_source=fs::read(&frozen).unwrap();fs::write(&frozen,"changed source").unwrap();
    assert!(epwflow::foundation::workspace_electronic::inspect_legacy(&cfg).is_err());fs::write(frozen,original_source).unwrap();
    let exit=c.root.join("run/3.qe-electronic/qe_eband/eband.exitcode");
    fs::write(&exit,"6").unwrap();assert!(epwflow::foundation::workspace_electronic::inspect_legacy(&cfg).is_err());fs::write(exit,"0").unwrap();
    let input=c.root.join("run/3.qe-electronic/qe_eband/eband.in");let original=fs::read(&input).unwrap();
    fs::write(&input,"&CONTROL calculation='bands' /\n").unwrap();assert!(epwflow::foundation::workspace_electronic::inspect_legacy(&cfg).is_err());fs::write(input,original).unwrap();
    fs::remove_file(c.root.join("fail-bands")).unwrap();c.accounting(Some(203));
    success(c.numbered_cli(1,&["--stage","wannier","--resume","-r"],Some(203),"legacy-pw-recovered"));
    assert_eq!(calls.0,c.calls().0);
}

#[test]
fn numbered_electronic_script_change_is_rejected_and_fresh_attempt_recovers() {
    let c=Case::numbered(); c.direct_setup();
    success(c.numbered_cli(1,&["--stage","wannier","-s"],None,"submit-before-edit"));
    let task=c.direct_task();
    let script=PathBuf::from(task["script"]["path"].as_str().unwrap());
    let original=fs::read(&script).unwrap();
    fs::write(&script,"#!/bin/bash\n# manually edited\nexit 0\n").unwrap();
    let out=c.numbered_cli(1,&["--stage","wannier","-s"],None,"edited-script");
    assert!(!out.status.success());
    assert_eq!(c.calls(),(String::new(),String::new(),String::new()));
    fs::write(&script,original).unwrap();
    assert_eq!(c.direct_execute(),0);
    success(c.numbered_cli(1,&["--stage","wannier","-s"],None,"next-native-task"));
    assert_ne!(c.direct_task()["id"],task["id"]);
}

#[test]
#[ignore = "需显式设置本机 MgB2 配置；只读验证，不提交或执行科学程序"]
fn numbered_real_legacy_pw_is_read_only() {
    let input=std::env::var_os("DFTFLOW_0020_REAL_CONFIG").expect("显式本机配置");
    let cfg=config::Config::load(std::path::Path::new(&input)).unwrap();
    let before=epwflow::qe_data::save::files(&cfg.work_dir.join("3.qe-electronic/qe_eband")).unwrap();
    let record=epwflow::foundation::workspace_electronic::inspect_legacy(&cfg).unwrap();
    assert_eq!(before,epwflow::qe_data::save::files(&cfg.work_dir.join("3.qe-electronic/qe_eband")).unwrap());
    println!("{}",serde_json::to_string_pretty(&record).unwrap());
}


// 0021：从公开提交及实际脚本覆盖 DOS 失败、旧作业拒绝和新尝试恢复。
#[test]
fn numbered_dos_failure_public_resubmit_preserves_pw_and_bands() {
    direct_submit::dos_failure_preserves_verified_pw_and_bands();
}

#[test]
fn numbered_deleted_wannier_directory_rebuilds_without_repeating_upstream() {
    use epwflow::foundation::stages::{self, Step};
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(0, &["-r"], Some(201), "initial-phonon"));
    c.accounting(Some(202));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(202), "initial-fit"));
    let before = c.calls();
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    for f in epwflow::qe_data::save::files(&cfg.work_dir).unwrap() {
        assert!(!f.relative_path.parent().unwrap().components().any(|part| part.as_os_str().to_string_lossy().starts_with(".")), "不得创建隐藏运行目录: {:?}", f.relative_path);
    }
    let upstream = fs::read(stages::receipt(&cfg.work_dir, Step::Nscf)).unwrap();
    // 构造已交付下游的真实 Completion 格式；依赖检查仍走生产实现。
    for step in [Step::Coarse, Step::Fine, Step::Superconductivity] {
        let dir = step.directory(&cfg);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("native-output"), step.name()).unwrap();
        let r = stages::Completion {
            schema_version: 1, step,
            scientific_key: stages::scientific_key(&cfg, step).unwrap(),
            upstream: stages::upstream(&cfg, step).unwrap(),
            outputs: vec![store::Identity::capture(&dir.join("native-output")).unwrap()], provenance: None,
        };
        store::write(&stages::receipt(&cfg.work_dir, step), &r).unwrap();
        assert!(stages::reusable(&cfg, step).unwrap(), "删除前的下游应可复用");
    }
    fs::remove_dir_all(c.root.join("run/5.epw-wannier")).unwrap();
    c.accounting(None);
    let files = epwflow::qe_data::save::files(&cfg.work_dir).unwrap();
    success(c.numbered_cli(1, &["--status", "--json"], None, "deleted-status"));
    assert_eq!(files, epwflow::qe_data::save::files(&cfg.work_dir).unwrap(), "状态查询必须只读");
    for step in [Step::Wannier, Step::Coarse, Step::Fine, Step::Superconductivity] {
        assert!(!stages::reusable(&cfg, step).unwrap());
    }
    c.script("sbatch", "#!/bin/sh\nprintf '203\\n'\n");
    success(c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "deleted-submit"));
    assert!(c.root.join("run/5.epw-wannier/epw_wannier.in").is_file());
    assert_eq!(before, c.calls(), "提交不能提前执行科学程序");
    c.accounting(Some(204));
    success(c.numbered_cli(1, &["--stage", "wannier", "--resume", "-r"], Some(204), "deleted-rebuild"));
    assert_eq!(before.0, c.calls().0, "完整上游 QE 不能重跑");
    assert_eq!(upstream, fs::read(stages::receipt(&cfg.work_dir, Step::Nscf)).unwrap());
    assert_eq!(c.calls().1.matches("wannier ").count(), before.1.matches("wannier ").count() + 1);
    assert!(stages::reusable(&cfg, Step::Wannier).unwrap());
    for step in [Step::Coarse, Step::Fine, Step::Superconductivity] {
        assert!(!stages::receipt(&cfg.work_dir, step).exists(), "旧下游不能在重算后复活");
    }
}

#[test]
fn numbered_missing_checkpoint_rebuilds_but_modified_or_corrupt_evidence_is_rejected() {
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(201), "fit-for-missing-file"));
    c.accounting(None);
    let chk = c.root.join("run/5.epw-wannier/scf.chk");
    let original = fs::read(&chk).unwrap();
    fs::write(&chk, "modified checkpoint").unwrap();
    let bad = c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "modified-checkpoint");
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("已改变"));
    fs::write(&chk, original).unwrap();
    let receipt = c.root.join("run/5.epw-wannier/completion.json");
    let raw = fs::read(&receipt).unwrap();
    fs::write(&receipt, "corrupt receipt").unwrap();
    assert!(!c.numbered_cli(1, &["--status"], None, "corrupt-receipt").status.success());
    fs::write(receipt, raw).unwrap();
    fs::remove_file(chk).unwrap();
    success(c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "missing-checkpoint-submit"));
}

#[test]
fn numbered_deleted_stage_never_bypasses_queued_or_unknown_submission() {
    let c=Case::numbered(); c.direct_setup();
    success(c.numbered_cli(1,&["--stage","wannier","-s"],None,"queued-wannier"));
    let active=c.root.join("run/epw-state/native-active.json");
    let before=fs::read(&active).unwrap();
    let task=c.direct_task();
    fs::remove_dir_all(task["directory"].as_str().unwrap()).unwrap();
    assert!(!c.numbered_cli(1,&["--stage","wannier","-s"],None,"queued-deleted-stage").status.success());
    assert_eq!(before,fs::read(active).unwrap());
    assert_eq!(fs::read_to_string(c.root.join("submissions")).unwrap().lines().count(),1);
}

impl Case {
    fn hide_numbered_state(&self) {
        use epwflow::foundation::stages::{self, Step};
        let cfg = config::Config::load(&self.root.join("input.toml")).unwrap();
        for step in Step::ALL {
            let receipt = stages::receipt(&cfg.work_dir, step);
            if receipt.is_file() {
                let dest = cfg.work_dir.join("epw-state/steps").join(format!("{}.json", step.name()));
                fs::create_dir_all(dest.parent().unwrap()).unwrap();
                fs::rename(receipt, dest).unwrap();
            }
        }
        let electronic = cfg.work_dir.join("3.qe-electronic/receipts");
        if electronic.exists() { fs::rename(electronic, cfg.work_dir.join("epw-state/electronic")).unwrap(); }
        fs::rename(cfg.work_dir.join("epw-state"), cfg.work_dir.join(".epwflow")).unwrap();
    }
}

#[test]
fn numbered_legacy_migration_preserves_history_budget_and_reuses_without_hidden_tree() {
    use epwflow::foundation::stages::{self, Step};
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(0, &["-r"], Some(201), "migration-qe"));
    c.accounting(Some(202));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(202), "migration-fit"));
    c.accounting(None);
    c.hide_numbered_state();
    let old = c.root.join("run/.epwflow");
    let original = epwflow::qe_data::save::files(&old).unwrap();
    let ledger = fs::read(c.root.join("fixture-ledger.json")).unwrap();
    let blocked = c.numbered_cli(1, &["--status"], None, "migration-required");
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("--migrate-state"));
    assert!(!c.root.join("run/epw-state").exists(), "只读查询不迁移");
    success(c.numbered_cli(1, &["--migrate-state"], None, "migration"));
    assert_eq!(original, epwflow::qe_data::save::files(&old).unwrap());
    assert_eq!(ledger, fs::read(c.root.join("fixture-ledger.json")).unwrap());
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    for s in [Step::Scf, Step::Nscf, Step::Phonon, Step::Wannier] {
        assert!(stages::reusable(&cfg, s).unwrap(), "迁移应复用 {}", s.name());
    }
    let report = c.root.join("run/epw-state/migration.json");
    let mut interrupted: Value = store::read(&report).unwrap();
    interrupted["complete"] = json!(false);
    store::write(&report, &interrupted).unwrap();
    assert!(!c.numbered_cli(1, &["--status"], None, "migration-incomplete-status").status.success());
    success(c.numbered_cli(1, &["--migrate-state"], None, "migration-finish-interrupted-publish"));
    assert_eq!(original, epwflow::qe_data::save::files(&old).unwrap());
    assert_eq!(ledger, fs::read(c.root.join("fixture-ledger.json")).unwrap());
    fs::rename(old, c.root.join("saved-legacy-state")).unwrap();
    let before = c.calls();
    success(c.numbered_cli(0, &["--status"], None, "migration-independent-status"));
    success(c.numbered_cli(1, &["--stage", "wannier"], None, "migration-independent-generate"));
    assert_eq!(before, c.calls());
    assert!(!c.root.join("run/.epwflow").exists());
    success(c.numbered_cli(1, &["--migrate-state"], None, "migration-idempotent"));
}

#[test]
fn numbered_migration_rejects_activity_and_retains_a_deleted_stage_as_pending() {
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(201), "migration-before-delete"));
    c.hide_numbered_state();
    let active_path = c.root.join("run/.epwflow/active.json");
    let before = fs::read(&active_path).unwrap();
    assert!(!c.numbered_cli(1, &["--migrate-state"], None, "migration-active").status.success());
    assert_eq!(before, fs::read(&active_path).unwrap());
    assert!(!c.root.join("run/epw-state").exists());
    c.put("accounting", "");
    assert!(!c.numbered_cli(1, &["--migrate-state"], None, "migration-unknown").status.success());
    c.accounting(None);
    let lock = store::Lock::acquire(&c.root.join("run/.epwflow/control")).unwrap();
    assert!(!c.numbered_cli(1, &["--migrate-state"], None, "migration-locked").status.success());
    assert!(c.root.join("run/.epwflow/control.lock/owner.json").is_file());
    drop(lock);
    fs::remove_dir_all(c.root.join("run/5.epw-wannier")).unwrap();
    // 201 已被原分配登记；迁移后模拟调度器必须返回新的作业号。
    c.script("sbatch", "#!/bin/sh\nprintf '202\\n'\n");
    success(c.numbered_cli(1, &["--migrate-state"], None, "migration-after-delete"));
    success(c.numbered_cli(1, &["--stage", "wannier", "-s"], None, "migration-deleted-submit"));
}

#[test]
fn numbered_old_wannier_receipt_rebuilds_unbound_velocity_files_and_rejects_tampering() {
    use epwflow::foundation::stages::{self, Completion, Step};
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(201), "velocity-initial"));
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let receipt = stages::receipt(&cfg.work_dir, Step::Wannier);
    let mut original: Completion = store::read(&receipt).unwrap();
    original.outputs.retain(|id| !matches!(id.path.extension().and_then(|x| x.to_str()), Some("bvec" | "mmn")));
    store::write(&receipt, &original).unwrap();
    assert!(!stages::reusable(&cfg, Step::Wannier).unwrap());
    assert!(stages::reusable(&cfg, Step::Scf).unwrap());
    assert!(stages::reusable(&cfg, Step::Nscf).unwrap());
    let qe = c.calls().0;
    c.accounting(Some(202));
    success(c.numbered_cli(1, &["--stage", "wannier", "-r"], Some(202), "velocity-rebuild"));
    assert_eq!(c.calls().0, qe, "旧拟合收据升级不能重算 QE");
    assert_eq!(c.calls().1.matches("wannier ").count(), 2);
    let current: Completion = store::read(&receipt).unwrap();
    for suffix in ["bvec", "mmn"] {
        assert!(current.outputs.iter().any(|id| id.path == Step::Wannier.directory(&cfg).join(format!("scf.{suffix}"))));
    }
    assert!(stages::reusable(&cfg, Step::Wannier).unwrap());
    fs::write(Step::Wannier.directory(&cfg).join("scf.mmn"), "(9,9)\n").unwrap();
    let calls = c.calls();
    c.accounting(Some(203));
    assert!(!c.numbered_cli(1, &["--stage", "coarse", "-r"], Some(203), "velocity-tampered").status.success());
    assert_eq!(c.calls(), calls, "已绑定文件篡改必须在原生执行前拒绝");
}

#[test]
fn numbered_coarse_retry_after_overwriting_the_driver_keeps_qe_results() {
    use std::io::Write;
    let c = Case::numbered();
    let driver = c.root.join("mutable-epw-wannier");
    fs::copy(BINS[1], &driver).unwrap();
    let run = |job, label| {
        let mut cmd = Command::new(&driver);
        cmd.args(["-c", c.root.join("input.toml").to_str().unwrap(), "--stage", "coarse", "-r"])
            .env("SLURM_NTASKS", "1");
        c.output(cmd, Some(job), label)
    };
    c.put("fail-coarse", "intentional native failure");
    c.accounting(Some(201));
    let initial = run(201, "driver-before-upgrade");
    assert!(!initial.status.success());
    assert!(String::from_utf8_lossy(&initial.stderr).contains("EPW 退出状态 Some(17)"));
    let before = c.calls().0;
    fs::OpenOptions::new().append(true).open(&driver).unwrap().write_all(b"upgraded-driver-marker").unwrap();
    c.accounting(Some(202));
    let retry = run(202, "driver-after-upgrade");
    assert!(!retry.status.success());
    assert!(String::from_utf8_lossy(&retry.stderr).contains("EPW 退出状态 Some(17)"), "{}", String::from_utf8_lossy(&retry.stderr));
    assert_eq!(before, c.calls().0);
}

#[test]
fn numbered_comparison_upgrade_reuses_native_coarse_and_rejects_damaged_science() {
    use std::io::Write;
    let c = Case::numbered();
    let driver = c.root.join("mutable-comparison-driver");
    fs::copy(BINS[1], &driver).unwrap();
    let run = |job, label| {
        let mut cmd = Command::new(&driver);
        cmd.args(["-c", c.root.join("input.toml").to_str().unwrap(), "--stage", "coarse", "-r"])
            .env("SLURM_NTASKS", "1");
        c.output(cmd, Some(job), label)
    };
    c.put("bad-path-frequencies", "intentional nonfinite frequency");
    c.accounting(Some(201));
    let first = run(201, "comparison-parse-failed");
    assert!(!first.status.success());
    assert!(String::from_utf8_lossy(&first.stderr).contains("非有限数"), "{}", String::from_utf8_lossy(&first.stderr));
    let request = c.root.join("run/6.epw-coarse/comparison-native/request.json");
    let coarse = c.root.join("run/epw-state/native-attempts/coarse.json");
    let protected_calls = || c.calls().0.lines().filter(|l| !matches!(*l, "q2r" | "matdyn")).collect::<Vec<_>>().join("\n");
    let before = (store::hash_file(&request).unwrap(), store::hash_file(&coarse).unwrap(), protected_calls());
    let coarse_calls = c.calls().1.lines().filter(|l| l.starts_with("coarse ")).count();
    fs::OpenOptions::new().append(true).open(&driver).unwrap().write_all(b"comparison-driver-upgrade").unwrap();
    let matrix = c.root.join("run/6.epw-coarse/epwdata.fmt");
    let raw = fs::read(&matrix).unwrap(); fs::write(&matrix, "damaged matrix").unwrap();
    c.accounting(Some(202));
    assert!(!run(202, "comparison-damaged-coarse").status.success());
    assert_eq!(before.2, protected_calls());
    fs::write(matrix, raw).unwrap();
    fs::remove_file(c.root.join("bad-path-frequencies")).unwrap();
    c.accounting(Some(203));
    success(run(203, "comparison-upgraded-resume"));
    assert_eq!(before, (store::hash_file(&request).unwrap(), store::hash_file(&coarse).unwrap(), protected_calls()));
    assert_eq!(coarse_calls, c.calls().1.lines().filter(|l| l.starts_with("coarse ")).count(), "粗 EPC 不得重算");
}

#[test]
fn numbered_native_retry_preserves_open_logs_and_ignores_legacy_log_inputs_only() {
    use std::{io::Write, os::unix::fs::MetadataExt};
    let c = Case::numbered();
    c.put("fail-coarse", "intentional failure");
    c.accounting(Some(201));
    let initial = c.numbered_cli(1, &["--stage", "coarse", "-r"], Some(201), "log-input-initial");
    assert!(String::from_utf8_lossy(&initial.stderr).contains("EPW 退出状态 Some(17)"));
    let dir = c.root.join("run/6.epw-coarse");
    let old_log = dir.join("slurm-201.err"); fs::write(&old_log, "before failure\n").unwrap();
    let receipt = c.root.join("run/epw-state/native-attempts/coarse.json");
    let mut legacy: Value = store::read(&receipt).unwrap();
    legacy["inputs"].as_array_mut().unwrap().push(serde_json::to_value(store::Identity::capture(&old_log).unwrap()).unwrap());
    let scratch = dir.join("out/scf.wfc1"); fs::write(&scratch, "before native").unwrap();
    legacy["inputs"].as_array_mut().unwrap().push(serde_json::to_value(store::Identity::capture(&scratch).unwrap()).unwrap());
    fs::write(&scratch, "native changed working wavefunction").unwrap();
    store::write(&receipt, &legacy).unwrap();
    fs::OpenOptions::new().append(true).open(&old_log).unwrap().write_all(b"normal failure appended\n").unwrap();
    let log = dir.join("slurm-202.out");
    let mut handle = fs::OpenOptions::new().create(true).append(true).open(&log).unwrap();
    handle.write_all(b"before archive\n").unwrap();
    let before = c.calls().0;
    c.accounting(Some(202));
    let retry = c.numbered_cli(1, &["--stage", "coarse", "-r"], Some(202), "log-input-retry");
    assert!(String::from_utf8_lossy(&retry.stderr).contains("EPW 退出状态 Some(17)"), "{}", String::from_utf8_lossy(&retry.stderr));
    assert_eq!(before, c.calls().0, "重试不能重复独立参考或上游 QE");
    handle.write_all(b"after archive\n").unwrap();
    assert_eq!(handle.metadata().unwrap().ino(), fs::metadata(&log).unwrap().ino());
    assert_eq!(fs::read_to_string(&log).unwrap(), "before archive\nafter archive\n");
    let current: Value = store::read(&receipt).unwrap();
    assert!(current["inputs"].as_array().unwrap().iter().all(|x| !x["path"].as_str().unwrap().contains("/slurm-")));
    fs::write(dir.join("save/scf.dvscf_q2"), "damaged scientific input").unwrap();
    c.accounting(Some(203));
    let corrupt = c.numbered_cli(1, &["--stage", "coarse", "-r"], Some(203), "log-input-corrupt-science");
    assert!(!corrupt.status.success());
    assert!(String::from_utf8_lossy(&corrupt.stderr).contains("scf.dvscf_q2"));
    assert_eq!(before, c.calls().0);
}

#[path = "support/direct_submit.rs"]
mod direct_submit;

#[path = "support/phonon_chain.rs"]
mod phonon_chain;

#[path = "support/qe_stage_plots.rs"]
mod qe_stage_plots;

#[test]
fn gap_points_report_invalid_data_and_resume_failed_point_without_repeating_success() {
    for (mode,fault,status) in [(config::Mode::Iso,"unconverged","not_converged"),(config::Mode::Aniso,"nan","invalid"),(config::Mode::Iso,"missing","missing")] {
        let c=Case::numbered();
        c.numbered_change(|cfg|{let s=&mut cfg.superconductivity;s.task=config::SuperconductivityTask::Gap;s.mode=mode;s.gap_temperatures_k=vec![20.,30.,40.];s.mixing_beta=-0.3;s.iso=None;s.aniso=None;});
        c.put("gap-fault",fault);c.accounting(Some(201));
        success(c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(201),"gap-invalid-data"));
        let root=c.root.join("run/8.epw-superconductivity");
        let r=epwflow::superconductivity::points::Run::load(&root.join("manifest.json")).unwrap();
        assert!(r.execution_complete);assert!(!r.data_complete);assert_eq!(r.attempts.len(),3);
        for a in r.attempts{let record=epwflow::superconductivity::verify::record(&a.record.unwrap().path).unwrap();assert!(record.gap.is_none());assert_eq!(record.gap_status.unwrap().data_status,status);}
        let (_,data)=epwflow::delivery::native::load(&root.join("results")).unwrap();assert!(data.gaps.is_empty()&&data.states.is_empty());
        assert_eq!(c.calls().2.lines().count(),3);
    }
    let c=Case::numbered();c.numbered_change(|cfg|{let s=&mut cfg.superconductivity;s.task=config::SuperconductivityTask::Gap;s.mode=config::Mode::Iso;s.gap_temperatures_k=vec![20.,30.,40.];s.iso=None;s.aniso=None;});
    c.accounting(Some(201));c.put("gap-fault","no-ending");
    assert!(!c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(201),"gap-no-ending").status.success());
    fs::remove_file(c.root.join("gap-fault")).unwrap();c.accounting(Some(202));
    success(c.numbered_cli(2,&["--stage","superconductivity","--resume","-r"],Some(202),"gap-failed-resume"));
    let r=epwflow::superconductivity::points::Run::load(&c.root.join("run/8.epw-superconductivity/manifest.json")).unwrap();
    assert!(r.execution_complete&&r.data_complete);assert_eq!(r.attempts.len(),4);assert_eq!(r.attempts[0].status,"failed");
    assert_eq!(c.calls().2.lines().count(),4);
}

#[test]
fn gap_resume_keeps_successful_temperature_and_preserves_failed_attempt() {
    let c=Case::numbered();c.numbered_change(|cfg|{let s=&mut cfg.superconductivity;s.task=config::SuperconductivityTask::Gap;s.mode=config::Mode::Iso;s.gap_temperatures_k=vec![20.,30.,40.];s.iso=None;s.aniso=None;});
    c.put("gap-fail30","failure");c.accounting(Some(201));
    assert!(!c.numbered_cli(2,&["--stage","superconductivity","-r"],Some(201),"gap-fail30").status.success());
    let path=c.root.join("run/8.epw-superconductivity/manifest.json");
    let first=epwflow::superconductivity::points::Run::load(&path).unwrap();assert_eq!(first.attempts.len(),2);assert_eq!(first.attempts[0].status,"valid");assert_eq!(first.attempts[1].status,"failed");
    let id=first.attempts[0].record.clone().unwrap();
    fs::remove_file(c.root.join("gap-fail30")).unwrap();c.accounting(Some(202));
    success(c.numbered_cli(2,&["--stage","superconductivity","--resume","-r"],Some(202),"gap-resume30"));
    let last=epwflow::superconductivity::points::Run::load(&path).unwrap();assert_eq!(last.attempts.len(),4);assert!(last.execution_complete&&last.data_complete);id.verify().unwrap();assert_eq!(last.attempts[0].record.as_ref().unwrap().sha256,id.sha256);
    let calls:Vec<Value>=c.calls().2.lines().map(|l|serde_json::from_str(l).unwrap()).collect();assert_eq!(calls.iter().map(|v|v["temperature_k"].as_f64().unwrap()).collect::<Vec<_>>(),[20.,30.,30.,40.]);
}

#[path = "support/multitemperature.rs"]
mod multitemperature;
