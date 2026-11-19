//! 0015 的真实接口契约回归。仅运行隔离替身，不调用 QE/EPW 或真实 Slurm。
//! 已知未修复项显式 ignore；用 --ignored 单独运行，失败不代表科学验收通过。
#![cfg(unix)]

#[path = "support/cli.rs"]
mod stage_cli;

#[path = "support/native_fixture.rs"]
mod native_fixture;
use epwflow::foundation::{config::Config, process, stage::Backend, store};
use epwflow::interpolation::adapter;
use epwflow::qe_data::{nscf, qe_config::QeSettings, reuse, scf};
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output},
};

struct Case {
    root: PathBuf,
}

impl Case {
    fn execution(&self, args: &[&str], job: u64) -> Output {
        stage_cli::command(args, &self.root)
            .current_dir(&self.root)
            .env("EPWFLOW_LOCAL_STATE", self.root.join("state"))
            .env("PATH", format!("{}:/usr/bin:/bin", self.root.display()))
            .env("SLURM_JOB_ID", job.to_string())
            .output()
            .unwrap()
    }

    fn workflow(&self) -> Value {
        serde_json::from_slice(&fs::read(self.root.join("run/epw-state/workflow.json")).unwrap())
            .unwrap()
    }

    fn scheduled(&self) {
        self.script(
            "sbatch",
            &format!(
                "#!/bin/sh\nprintf 'submit\\n' >> '{}'\nprintf '123;test-cluster\\n'\n",
                self.root.join("submissions").display()
            ),
        );
        self.script("sacct", "#!/bin/sh\nprintf '123|RUNNING|0|1|\\n'\n");
        self.script("launcher", "#!/bin/sh\nshift 2\nexec \"$@\"\n");
    }

    fn qe_fixture(&self) {
        let input = fs::read_to_string(self.root.join("input.toml")).unwrap();
        let mut cfg:Config=toml::from_str(&input).unwrap();
        native_fixture::phonon(&mut cfg,epwflow::foundation::config::PhMode::Nosplit);
        cfg.wannier.frozen_window_ev=Some([-0.5,0.5]);
        self.put("input.toml",&toml::to_string_pretty(&cfg).unwrap());
        // 此替身通过选窗能力预检，但不生成任何原生产物；来源完整性仍须拒绝。
        self.script(
            "epw.x",
            "#!/bin/sh\n# dis_proj_min dis_proj_max dis_froz_proj\nexit 0\n",
        );
        self.put("KPATH.in", "0 0 0 G\n0.5 0 0 X\n");
        self.put("qe-native.py", include_str!("fixtures/qe_native_io.py"));
        for kind in ["pw", "ph", "q2r", "matdyn"] {
            self.script(
                &format!("{kind}.x"),
                &format!(
                    "#!/bin/sh\nexec /usr/bin/python3 '{}' {kind} \"$@\"\n",
                    self.root.join("qe-native.py").display()
                ),
            );
        }
    }
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("epw-real-contracts-{}", store::token()));
        fs::create_dir(&root).unwrap();
        let c = Self { root };
        c.script("scontrol", include_str!("fixtures/slurm_controller.py"));
        c.put("input.toml", include_str!("fixtures/input.toml"));
        c.put("POSCAR", "MgB2\n1\n3 0 0\n-1.5 2.598 0\n0 0 3.5\nMg B\n1 2\nDirect\n0 0 0\n0.3333 0.6667 0.5\n0.6667 0.3333 0.5\n");
        for name in ["Mg.upf", "B.upf"] {
            c.put(
                name,
                "isolated interface fixture; not a scientific pseudopotential\n",
            );
        }
        for name in [
            "pw.x", "ph.x", "epw.x", "launcher", "sbatch", "sacct", "squeue",
        ] {
            c.script(name, "#!/bin/sh\nexit 91\n");
        }
        c
    }

    fn put(&self, name: &str, content: &str) {
        let path = self.root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn script(&self, name: &str, content: &str) {
        self.put(name, content);
        fs::set_permissions(self.root.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn cli(&self, args: &[&str]) -> Output {
        stage_cli::command(args, &self.root)
            .current_dir(&self.root)
            .env("EPWFLOW_LOCAL_STATE", self.root.join("state"))
            .env("XDG_STATE_HOME", self.root.join("xdg-state"))
            // 调度查询只允许命中本例替身；不存在可继承的真实 Slurm 命令。
            .env("PATH", &self.root)
            .env_remove("SLURM_JOB_ID")
            .env_remove("SLURM_JOBID")
            .output()
            .unwrap()
    }

    fn config(&self) -> Config {
        Config::load(&self.root.join("input.toml")).unwrap()
    }

    fn empty_manifest(&self) -> PathBuf {
        let manifest = json!({
            "schema_version": 1, "run_id": "empty-should-be-rejected",
            "fingerprints": {"structure_sha256": "", "pseudo_sha256": {}, "program_sha256": {}},
            "scf": null, "states": null, "qpoints": [], "phonon_files": [],
            "validation": {"status": "valid", "checks": [], "units": {}, "thresholds": {}, "evidence": []}
        });
        let path = self.root.join("source/qe-data/manifest.json");
        store::write(&path, &manifest).unwrap();
        path
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

// 调用带预算注册的库函数时使用独立测试进程，避免修改并行用例的环境变量。
fn isolated_worker(name: &str) -> bool {
    if std::env::var("EPWFLOW_REGRESSION_WORKER").as_deref() == Ok(name) {
        return false;
    }
    let c = Case::new();
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--ignored", "--nocapture"])
        .env("EPWFLOW_REGRESSION_WORKER", name)
        .env("EPWFLOW_LOCAL_STATE", c.root.join("isolated-state"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "Isolation has failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    true
}

#[test]
fn process_captures_stdout_without_fixture_writing_stage_log() {
    let c = Case::new();
    let report = process::run(
        &[
            "/bin/sh".into(),
            "-c".into(),
            "printf 'native-style stdout\\n'".into(),
        ],
        &c.root,
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(report.stdout_path).unwrap(),
        "native-style stdout\n"
    );
}

#[test]
fn b01_run_and_submit_validate_missing_configuration() {
    let c = Case::new();
    for cmd in ["run", "submit"] {
        let out = c.cli(&[cmd, "-c", "absent-input.toml"]);
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success());
        assert!(
            err.contains("absent-input.toml"),
            "{cmd} The actual configuration path should be reported:{err}"
        );
        assert!(!err.contains("not_implemented"), "{cmd} It's not connected yet:{err}");
    }
}

#[test]
fn b01_until_is_a_recognized_execution_boundary() {
    let c = Case::new();
    let out = c.cli(&[
        "submit",
        "-c",
        "absent-input.toml",
        "--until",
        "interpolation",
    ]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "Missing configuration cannot be submitted successfully");
    assert!(
        err.contains("absent-input.toml"),
        "Check configuration through parameter resolution:{err}"
    );
    assert!(
        !err.contains("unexpected argument"),
        "Stop border unrecognized:{err}"
    );
}



#[test]
fn b01_resource_run_checks_slurm_without_budget_registration() {
    let c = Case::new();
    c.scheduled();
    c.qe_fixture();
    c.script("scontrol", "#!/bin/sh\nexit 91\n");
    let o = c.cli(&["run", "-c", "input.toml", "--until", "qe-data"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("allocation"));
    let o = c.execution(&["run", "-c", "input.toml", "--until", "qe-data"], 123);
    assert!(!o.status.success());
    let error = String::from_utf8_lossy(&o.stderr);
    assert!(error.contains("scontrol"), "{error}");
    assert!(!c.root.join("campaign").exists());
    assert!(!c.root.join("scientific-calls").exists());
}



#[test]
fn b01_until_enum_rejects_invalid_boundary_without_mutation() {
    let c = Case::new();
    let o = c.cli(&["submit", "-c", "input.toml", "--until", "fake-stage"]);
    assert!(!o.status.success());
    assert!(!c.root.join("run").exists());
    assert!(!c.root.join("state").exists());
}



#[test]
fn b09_reads_qe_schema_text_coordinates_and_sibling_eigenvalues() {
    let c = Case::new();
    // 根据已盘点的 QE 7.4.1 XML 层级构造最小格式样本。
    // 数值为合成值，仅验证格式/维度，不代表 MgB2 能量或物理参考。
    c.put(
        "data-file-schema.xml",
        r#"<?xml version="1.0"?>
<qes:espresso xmlns:qes="http://www.quantum-espresso.org/ns/qes/qes-1.0">
 <output><band_structure>
  <lsda>false</lsda><noncolin>false</noncolin><spinorbit>false</spinorbit>
  <nbnd>2</nbnd><nelec>2.0</nelec><nks>2</nks><fermi_energy>0.25</fermi_energy>
  <ks_energies><k_point weight="1.0">0.0 0.0 0.0</k_point><npw>12</npw>
   <eigenvalues size="2">-0.5 0.5</eigenvalues><occupations size="2">1.0 0.0</occupations>
  </ks_energies>
  <ks_energies><k_point weight="1.0">0.5 0.0 0.0</k_point><npw>14</npw>
   <eigenvalues size="2">-0.4 0.6</eigenvalues><occupations size="2">1.0 0.0</occupations>
  </ks_energies>
 </band_structure></output>
</qes:espresso>"#,
    );
    let states = nscf::parse_xml(&c.root.join("data-file-schema.xml"))
        .expect("Should accept the truth schema text coordinates and adjacent eigenvalues");
    assert_eq!(states.kpoints, vec![[0.0, 0.0, 0.0], [0.5, 0.0, 0.0]]);
    assert_eq!(states.nbnd, 2);
    assert_eq!(states.weights.len(), 2);
    assert_eq!(states.eigenvalues.len(), 2);
    assert!(states
        .eigenvalues
        .iter()
        .all(|v| v.len() == 2 && v.iter().all(|x| x.is_finite())));
    // 单位与完整均匀网格的独立核对仍需额外测试，不由该格式用例证明。
}

#[test]
fn b04_import_rejects_empty_manifest_despite_valid_label() {
    let c = Case::new();
    c.empty_manifest();
    let result = reuse::import_from_run(&c.root.join("source"), &c.root.join("target"));
    assert!(
        result.is_err(),
        "Nothing SCF/Electronic State/PH Data cannot be imported successfully:{result:?}"
    );
    assert!(
        !c.root.join("target/imported/import-report.json").exists(),
        "Success reports are not published"
    );
}

#[test]
fn b08_interpolation_rejects_empty_f2_manifest() {
    let c = Case::new();
    let path = c.empty_manifest();
    let result = adapter::bind_f2(&path, &c.config());
    assert!(result.is_err(), "F3 You can't just do it valid Accept Empty F2:{result:?}");
}

#[test]
fn b05_scf_consumes_stdout_and_rejects_nonconvergence() {
    let c = Case::new();
    c.script(
        "pw.x",
        "#!/bin/sh\nprintf 'convergence NOT achieved after 100 iterations\\nJOB DONE.\\n'\n",
    );
    let cfg = c.config();
    let qe = QeSettings::from_config(&cfg).unwrap();
    let result = scf::run_scf(&qe, &c.root.join("work"), &[], Backend::Local);
    let err = format!("{:#}", result.expect_err("Failure to recover must fail"));
    assert!(fs::read_to_string(c.root.join("work/scf/scf.out")).unwrap().contains("convergence NOT achieved"));
    assert!(
        !err.contains("Missing scf.out"),
        "Native stdout Created. Do not cover up check for log link failure:{err}"
    );
    assert!(
        err.contains("Unfinished") || err.to_lowercase().contains("converg"),
        "The value of the diagnosis should not be diminished:{err}"
    );
}


#[test]
fn b07_qe_stages_use_bound_prefix() {
    let c = Case::new();
    let cfg = c.config();
    let qe = QeSettings::from_config(&cfg).unwrap();
    let re = regex::Regex::new(r#"(?i)prefix\s*=\s*['\"]([^'\"]+)['\"]"#).unwrap();
    for (stage, input) in [
        ("scf", qe.scf_input("./tmp").unwrap()),
        ("nscf", qe.nscf_input("./tmp").unwrap()),
        ("ph_init", qe.phonon_init_input("./tmp").unwrap()),
    ] {
        let prefix = re.captures(&input).expect("Enter must be declared prefix");
        assert_eq!(&prefix[1], cfg.qe.prefix, "{stage} Must use bounded prefix");
    }
}

#[test]
fn b02_rejects_interpolation_without_native_spectrum() {
    use epwflow::foundation::config::Program;
    use epwflow::interpolation::scheduler;

    if isolated_worker("b02_rejects_interpolation_without_native_spectrum") {
        return;
    }
    let c = Case::new();
    let f2 = c.empty_manifest();
    // 非空的自述检查也不能代替真实文件/谱/质量证据；避免仅由空清单防护截断本用例。
    let mut declared: Value = store::read(&f2).unwrap();
    declared["validation"]["checks"] =
        json!([{"name":"fixture_claim", "passed":true, "detail":"not scientific evidence"}]);
    store::write(&f2, &declared).unwrap();
    let source_before = store::hash_file(&f2).unwrap();
    // 故意沿用旧测试接受的简化文件，证明它们不足以作为真实 EPW 结果。
    // 不提供原生谱、完整矩阵元、合法 F2 或独立质量参考。
    c.script(
        "epw.x",
        r#"#!/bin/sh
case "$2" in
  *epw_coarse.in)
    printf 'fixture only\n' > epw.out
    printf 'not a native matrix\n' > coarse.ephmat ;;
  *epw_fine.in)
    printf 'fixture only\n' > epw.out
    printf '100 200 300\n' > fine_frequencies.dat
    printf '1 2 3\n' > fine_eigenvalues.dat
    printf 'not a map\n' > fine_kqmap.dat
    printf 'not a native matrix\n' > fine_ephmat_shard.dat ;;
  *) exit 92 ;;
esac
"#,
    );
    c.script(
        "wannier90.x",
        "#!/bin/sh\nprintf 'Convergence reached\\nSpread = 1.5\\n' > scf.wout\n",
    );
    c.script("pw2wannier90.x", "#!/bin/sh\nexit 0\n");
    let mut cfg = c.config();
    cfg.programs.wannier90 = Some(Program {
        path: c.root.join("wannier90.x"),
        args: vec![],
    });
    cfg.programs.pw2wannier90 = Some(Program {
        path: c.root.join("pw2wannier90.x"),
        args: vec![],
    });
    let work = c.root.join("interpolation-run");
    let result = scheduler::run(&cfg, Backend::Local, &[], &work, &c.root.join("source"));
    assert_eq!(
        store::hash_file(&f2).unwrap(),
        source_before,
        "Source must read only"
    );
    assert!(
        result.is_err(),
        "No native spectra and valid data could not be published successfully:{result:?}"
    );
    assert!(
        !work.join("interpolation/manifest.json").exists(),
        "No Synthetic Valid Synthetic Synthetic Synthesis may be published"
    );
}

#[test]
fn b03_rejects_superconductivity_without_successful_native_solver() {
    use epwflow::foundation::config::Mode;
    use epwflow::superconductivity::scheduler;

    let c = Case::new();
    // 若调用 EPW 则明确失败；任何已有谱都不能绕过原生求解与收敛证据。
    c.script(
        "epw.x",
        "#!/bin/sh\nprintf 'called\\n' > native-solver-called\nexit 91\n",
    );
    let mut cfg = c.config();
    cfg.superconductivity.mode = Mode::Iso;
    cfg.superconductivity.task = epwflow::foundation::config::SuperconductivityTask::Gap;
    cfg.superconductivity.gap_temperatures_k = vec![2.0, 5.0, 8.0];
    let work = c.root.join("superconductivity-run");
    // 明确是用来检测错误放行的合成清单，没有原生求解或矩阵元来源。
    let f3 = json!({
        "schema_version": 2, "run_id": "untrusted-fixture", "f2_digest": "x", "wannier_digest": "y",
        "coarse": null,
        "fine": {"kpoints": 8, "qpoints": 8, "energies": [1.0, 2.0, 3.0, 4.0],
            "frequencies": [40.3, 80.6, 120.9, 161.2], "weights": [0.25, 0.25, 0.25, 0.25],
            "kqmap": 4, "layout": "epw", "ephmat_shards": 1, "artifacts": []},
        "spectra": [{"smearing_id": "fixture", "energy_mev": [5.0, 10.0, 15.0, 20.0],
            "a2f": [0.192, 0.192, 0.192, 0.192], "lambda_integral": 0.8, "lambda_sum": 0.8, "cutoff_ev": 0.2}],
        "validation": {"status": "valid", "checks": [{"name":"fixture_claim", "passed":true, "detail":"not scientific evidence"}], "units": {}, "thresholds": {}, "evidence": []}
    });
    store::write(&work.join("interpolation/manifest.json"), &f3).unwrap();
    let result = scheduler::run_configured(&cfg, &work, &[], Backend::Local, false);
    assert!(
        result.is_err(),
        "Unsuccessful natives EPW Solution cannot be delivered Tc/ Can't stop :{result:?}"
    );
    if let Ok(manifest) = fs::read_to_string(work.join("superconductivity/manifest.json")) {
        let value: Value = serde_json::from_str(&manifest).unwrap();
        assert_ne!(
            value["validation"]["status"], "valid",
            "Evidence of failure may be retained and no valid result may be published"
        );
    }
}

#[test]
fn b08_wannier_final_state_does_not_override_nonconvergence() {
    use epwflow::interpolation::wannier;
    let c = Case::new();
    // 合成冲突日志：最终状态与有限展宽不是收敛证据。
    c.put(
        "nonconverged.wout",
        "Convergence not reached\nFinal State\nSpread = 1.5\n",
    );
    let result = wannier::parse_wannier_log(&c.root.join("nonconverged.wout"));
    assert!(
        result.is_err(),
        "Unequivocal logs are not allowed Final State Overwrite:{result:?}"
    );
}

#[test]
fn shared_qe_inputs_keep_physics_and_full_mesh() {
    let c = Case::new();
    let cfg = c.config();
    let qe = QeSettings::from_config(&cfg).unwrap();
    let input = qe.scf_input("./tmp").unwrap();
    let species = input.split("ATOMIC_SPECIES\n").nth(1).unwrap();
    let rows: Vec<_> = species
        .lines()
        .take(2)
        .map(|l| l.split_whitespace().collect::<Vec<_>>())
        .collect();
    assert_eq!(rows[0][0], "Mg");
    assert!((rows[0][1].parse::<f64>().unwrap() - 24.305).abs() < 1e-10);
    assert!((rows[1][1].parse::<f64>().unwrap() - 10.811).abs() < 1e-10);
    assert_eq!(rows[0][2], "Mg.upf");
    assert!(input.contains(&format!("pseudo_dir = '{}'", cfg.pseudos["Mg"].parent().unwrap().display())));
    let nscf = qe.nscf_input("./tmp").unwrap();
    let system = nscf
        .split("&SYSTEM\n")
        .nth(1)
        .unwrap()
        .split("/\n")
        .next()
        .unwrap();
    // Native EPW 5.9 requires consistent symmetry metadata even though the
    // electronic input must still enumerate the complete uniform mesh.
    assert!(system.contains("nosym = .false.") && system.contains("noinv = .false."));
    let k = nscf.split("K_POINTS crystal\n").nth(1).unwrap();
    let count = cfg.qe.nscf.k_grid.iter().product::<u32>() as usize;
    assert_eq!(k.lines().next().unwrap().parse::<usize>().unwrap(), count);
    assert_eq!(k.lines().skip(1).count(), count);
    let init = qe.phonon_init_input("./tmp").unwrap();
    assert!(init.contains("nq1 = 2") && init.ends_with("/\n"));
    let point = epwflow::qe_data::manifest::QPoint {
        global_index: 2,
        local_index: 1,
        directory: "02".into(),
        crystal: [0., 0.5, 0.],
        cartesian: [0., -0.577350269189585, 0.],
        alat: 5.8,
        reciprocal_basis: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        source_dyn0: "MgB2.dyn0".into(),
    };
    let mut coords_cfg=cfg.clone();native_fixture::phonon(&mut coords_cfg,epwflow::foundation::config::PhMode::Coords);
    let coords_qe=QeSettings::from_config(&coords_cfg).unwrap();
    let coords = coords_qe
        .phonon_q_input(
            "./tmp",
            &epwflow::foundation::config::PhMode::Coords,
            Some(&point),
            2,
        )
        .unwrap();
    let (head, tail) = coords.split_once("/\n").unwrap();
    assert!(head.contains("ldisp = .false.") && head.contains("only_init = .false."));
    assert!(!head.contains("nq1") && !head.contains("xq("));
    let parsed: Vec<f64> = tail
        .split_whitespace()
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(parsed.as_slice(), &point.cartesian);
    let index = qe
        .phonon_q_input(
            "./tmp",
            &epwflow::foundation::config::PhMode::Index,
            None,
            2,
        )
        .unwrap();
    assert!(index.contains("start_q = 2") && index.contains("last_q = 2"));
    assert!(index.contains("only_init = .false."));
    let mut nosplit_cfg=cfg.clone();native_fixture::phonon(&mut nosplit_cfg,epwflow::foundation::config::PhMode::Nosplit);
    let nosplit = QeSettings::from_config(&nosplit_cfg).unwrap().phonon_nosplit_input("./tmp").unwrap();
    assert!(nosplit.contains("ldisp = .true.") && nosplit.contains("only_init = .false."));
    assert!(!nosplit.contains("last_q"));
}

#[test]
fn qe_species_layout_keeps_explicit_files_and_rejects_native_filename_limit() {
    let c = Case::new();
    let mut cfg = c.config();
    for (el, folder) in [("Mg", "first"), ("B", "second")] {
        let dir = c.root.join(folder);
        fs::create_dir(&dir).unwrap();
        let path = dir.join(format!("{el}.upf"));
        fs::copy(&cfg.pseudos[el], &path).unwrap();
        cfg.pseudos.insert(el.into(), path);
    }
    cfg.set_native(&[epwflow::foundation::native_config::NSCF,"pw","ATOMIC_SPECIES"],json!("Mg 24.305 'first/Mg.upf'\nB 10.811 'second/B.upf'")).unwrap();
    let qe = QeSettings::from_config(&cfg).unwrap();
    let input = qe.nscf_input("./out").unwrap();
    assert!(input.contains(&format!("pseudo_dir = '{}'", c.root.display())));
    assert!(input.contains("'first/Mg.upf'") && input.contains("'second/B.upf'"));
    let long = c.root.join(format!("{}.upf", "x".repeat(81)));
    fs::copy(&cfg.pseudos["Mg"], &long).unwrap();
    cfg.pseudos.insert("Mg".into(), long);
    assert!(QeSettings::from_config(&cfg).err().unwrap().to_string().contains("80 bytes"));
}

#[test]
fn shared_qe_input_honors_explicit_reference_atomic_masses() {
    let c = Case::new();
    let mut cfg = c.config();
    cfg.atomic_masses.insert("B".into(), 10.811);
    let qe = QeSettings::from_config(&cfg).unwrap();
    let input = qe.scf_input("./tmp").unwrap();
    let species = input.split("ATOMIC_SPECIES\n").nth(1).unwrap();
    let b = species
        .lines()
        .find(|line| line.split_whitespace().next() == Some("B"))
        .unwrap();
    assert_eq!(
        b.split_whitespace().nth(1).unwrap().parse::<f64>().unwrap(),
        10.811
    );
    cfg.atomic_masses.insert("B".into(), f64::NAN);
    assert!(cfg.validate().is_err());
    cfg.atomic_masses.insert("B".into(), 10.811);
    cfg.atomic_masses.insert("Pb".into(), 207.2);
    assert!(cfg.validate().is_err());
}

#[test]
fn strict_scheduler_identity_and_accounting() {
    use epwflow::foundation::slurm::{parse_accounting, parse_job_id};
    for bad in [
        "0",
        "-1",
        "12 extra",
        "12\n13",
        "Submitted batch job 12",
        "12;cluster;other",
    ] {
        assert!(parse_job_id(bad).is_err(), "{bad}");
    }
    let job = parse_job_id("12;cluster-a\n").unwrap();
    assert_eq!(job.job_id, 12);
    assert_eq!(job.cluster.as_deref(), Some("cluster-a"));
    let done = parse_accounting(12, "12|COMPLETED|37|1|\n12.batch|COMPLETED|37|1|\n").unwrap();
    assert!(done.terminal);
    assert_eq!(done.node_seconds, 37);
    let running = parse_accounting(12, "12|RUNNING|8|1|\n").unwrap();
    assert!(!running.terminal);
    for bad in [
        "",
        "12.batch|COMPLETED|37|1|",
        "12|MYSTERY|0|0|",
        "12|COMPLETED|3|0|",
        "12|COMPLETED|3|1|\n12|FAILED|2|1|",
    ] {
        assert!(parse_accounting(12, bad).is_err(), "{bad}");
    }
}


#[test]
fn prepare_rejects_changed_bound_configuration_without_rewriting() {
    let c = Case::new();
    assert!(c.cli(&["prepare", "-c", "input.toml"]).status.success());
    let snap = c.root.join("run/input-init.toml");
    let original = fs::read(&snap).unwrap();
    let config = fs::read_to_string(c.root.join("input.toml")).unwrap();
    c.put(
        "input.toml",
        &config.replace("muc = 0.1", "muc = 0.12"),
    );
    let out = c.cli(&["prepare", "-c", "input.toml"]);
    assert!(!out.status.success());
    assert_eq!(fs::read(&snap).unwrap(), original);
    fs::write(&snap, b"tampered configuration").unwrap();
    let out = c.cli(&["resume", "-w", "run", "-r"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("input_conflict"));
}


// 0026：旧提交器专属生命周期用例由 three_stage_execution/support/direct_submit
// 的原生提交、防重、未知预算、失败恢复及逐步全链契约取代；-r 与科学解析回归保留。
#[test]
fn legacy_submit_is_rejected_before_submission_or_scientific_execution() {
    let c=Case::new(); c.scheduled();
    let out=c.cli(&["submit","-c","input.toml"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("migration_required"));
    assert!(!c.root.join("submissions").exists());
    assert!(!c.root.join("scientific-calls").exists());
}
