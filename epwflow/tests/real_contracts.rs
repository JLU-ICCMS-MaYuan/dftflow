//! 0015 的真实接口契约回归。仅运行隔离替身，不调用 QE/EPW 或真实 Slurm。
//! 已知未修复项显式 ignore；用 --ignored 单独运行，失败不代表科学验收通过。
#![cfg(unix)]

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
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("epw-real-contracts-{}", store::token()));
        fs::create_dir(&root).unwrap();
        let c = Self { root };
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
        Command::new(env!("CARGO_BIN_EXE_epwflow"))
            .current_dir(&self.root)
            .env("EPWFLOW_LOCAL_STATE", self.root.join("state"))
            .env("XDG_STATE_HOME", self.root.join("xdg-state"))
            // 调度查询只允许命中本例替身；不存在可继承的真实 Slurm 命令。
            .env("PATH", &self.root)
            .env_remove("SLURM_JOB_ID")
            .env_remove("SLURM_JOBID")
            .args(args)
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
        "隔离回归失败：\n{}\n{}",
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
#[ignore = "B01: run/submit 尚未加载配置，修复后取消 ignore"]
fn b01_run_and_submit_validate_missing_configuration() {
    let c = Case::new();
    for cmd in ["run", "submit"] {
        let out = c.cli(&[cmd, "-c", "absent-input.toml"]);
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success());
        assert!(
            err.contains("absent-input.toml"),
            "{cmd} 应报告实际配置路径：{err}"
        );
        assert!(!err.contains("not_implemented"), "{cmd} 尚未接通：{err}");
    }
}

#[test]
#[ignore = "B01: --until 未接入，修复后取消 ignore"]
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
    assert!(!out.status.success(), "缺失配置不得提交成功");
    assert!(
        err.contains("absent-input.toml"),
        "应通过参数解析后检查配置：{err}"
    );
    assert!(
        !err.contains("unexpected argument"),
        "停止边界未识别：{err}"
    );
}

#[test]
#[ignore = "B09: QE 原生 XML 结构解析失败，修复后取消 ignore"]
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
        .expect("应接受真实 schema 中的文本坐标与相邻 eigenvalues");
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
#[ignore = "B04: 空清单被导入成功，修复后取消 ignore"]
fn b04_import_rejects_empty_manifest_despite_valid_label() {
    let c = Case::new();
    c.empty_manifest();
    let result = reuse::import_from_run(&c.root.join("source"), &c.root.join("target"));
    assert!(
        result.is_err(),
        "没有任何 SCF/电子态/PH 数据不能导入成功：{result:?}"
    );
    assert!(
        !c.root.join("target/imported/import-report.json").exists(),
        "不得发布成功报告"
    );
}

#[test]
#[ignore = "B08: 空 F2 清单可绑定 F3，修复后取消 ignore"]
fn b08_interpolation_rejects_empty_f2_manifest() {
    let c = Case::new();
    let path = c.empty_manifest();
    let result = adapter::bind_f2(&path, &c.config());
    assert!(result.is_err(), "F3 不得仅凭 valid 接受空 F2：{result:?}");
}

#[test]
#[ignore = "B05: SCF 阶段没有消费已捕获 stdout，修复后取消 ignore"]
fn b05_scf_consumes_stdout_and_rejects_nonconvergence() {
    let c = Case::new();
    c.script(
        "pw.x",
        "#!/bin/sh\nprintf 'convergence NOT achieved after 100 iterations\\nJOB DONE.\\n'\n",
    );
    let cfg = c.config();
    let qe = QeSettings::from_config(&cfg).unwrap();
    let result = scf::run_scf(&qe, &c.root.join("work"), &[], Backend::Local);
    let err = format!("{:#}", result.expect_err("未收敛必须失败"));
    assert!(
        !err.contains("缺少 scf.out"),
        "原生 stdout 已产生，不能因日志衔接失败掩盖收敛检查：{err}"
    );
    assert!(
        err.contains("收敛") || err.to_lowercase().contains("converg"),
        "应诊断数值未收敛：{err}"
    );
}

#[test]
#[ignore = "B06: 未知作业被直接结算并释放并发，修复后取消 ignore"]
fn b06_unknown_scheduler_preserves_active_allocation() {
    let c = Case::new();
    let ledger = "campaign/ledger.json";
    let out = c.cli(&[
        "budget",
        "reserve",
        "--ledger",
        ledger,
        "--label",
        "isolated-test",
        "--nodes",
        "1",
        "--mpi",
        "1",
        "--wall-seconds",
        "60",
        "--storage-bytes",
        "1",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    let token = text.trim().strip_prefix("reserved: token=").unwrap();
    let out = c.cli(&[
        "budget", "attach", "--ledger", ledger, "--token", token, "--job", "424242",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let path = c.root.join(ledger);
    let before: Value = store::read(&path).unwrap();
    let _ = c.cli(&["budget", "reconcile", "--ledger", ledger]);
    let after: Value = store::read(&path).unwrap();
    assert_eq!(
        after["allocations"], before["allocations"],
        "调度查询失败不能释放未知作业"
    );
    assert_eq!(after["lanes"], before["lanes"], "并发占用必须保留");
    assert_eq!(
        after["spent_node_seconds"], before["spent_node_seconds"],
        "不能猜测实际消耗"
    );
}

#[test]
#[ignore = "B07: SCF/NSCF/PH 的 prefix 与配置不一致，修复后取消 ignore"]
fn b07_qe_stages_use_bound_prefix() {
    let c = Case::new();
    let cfg = c.config();
    let qe = QeSettings::from_config(&cfg).unwrap();
    let re = regex::Regex::new(r#"(?i)prefix\s*=\s*['\"]([^'\"]+)['\"]"#).unwrap();
    for (stage, input) in [
        ("scf", qe.scf_input("./tmp")),
        ("nscf", qe.nscf_input("./tmp")),
        ("ph_init", qe.phonon_init_input("./tmp")),
    ] {
        let prefix = re.captures(&input).expect("输入必须声明 prefix");
        assert_eq!(&prefix[1], cfg.qe.prefix, "{stage} 必须使用已绑定 prefix");
    }
}

#[test]
#[ignore = "B02/B08: 无原生谱及质量参考仍发布插值结果，修复后取消 ignore"]
fn b02_rejects_interpolation_without_native_spectrum_or_quality_evidence() {
    use epwflow::foundation::config::Program;
    use epwflow::interpolation::scheduler;

    if isolated_worker("b02_rejects_interpolation_without_native_spectrum_or_quality_evidence") {
        return;
    }
    let c = Case::new();
    let f2 = c.empty_manifest();
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
        "来源必须只读"
    );
    assert!(
        result.is_err(),
        "无原生谱和有效数据不能发布插值成功：{result:?}"
    );
    assert!(
        !work.join("interpolation/manifest.json").exists(),
        "不得发布合成有效谱"
    );
}

#[test]
#[ignore = "B03: 未调用原生 EPW 即发布超导结果，修复后取消 ignore"]
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
    let iso = cfg.superconductivity.iso.as_mut().unwrap();
    iso.max_k = 10.0;
    iso.precision_k = 0.1;
    cfg.superconductivity.gap_temperatures_k = vec![2.0, 5.0, 8.0];
    let work = c.root.join("superconductivity-run");
    // 明确是用来检测错误放行的合成清单，没有原生求解或矩阵元来源。
    let f3 = json!({
        "schema_version": 1, "run_id": "untrusted-fixture", "f2_digest": "x", "wannier_digest": "y",
        "coarse": null,
        "fine": {"kpoints": 8, "qpoints": 8, "energies": [1.0, 2.0, 3.0, 4.0],
            "frequencies": [40.3, 80.6, 120.9, 161.2], "weights": [0.25, 0.25, 0.25, 0.25],
            "kqmap": 4, "layout": "epw", "ephmat_shards": 1, "artifacts": []},
        "spectra": [{"smearing_id": "fixture", "energy_mev": [5.0, 10.0, 15.0, 20.0],
            "a2f": [0.192, 0.192, 0.192, 0.192], "lambda_integral": 0.8, "lambda_sum": 0.8, "cutoff_ev": 0.2}],
        "quality": null, "mode_readiness": {"iso": true},
        "validation": {"status": "valid", "checks": [], "units": {}, "thresholds": {}, "evidence": []}
    });
    store::write(&work.join("interpolation/manifest.json"), &f3).unwrap();
    let result = scheduler::run(&cfg, &work, &[], Backend::Local);
    assert!(
        result.is_err(),
        "没有成功的原生 EPW 求解不得交付 Tc/能隙：{result:?}"
    );
    if let Ok(manifest) = fs::read_to_string(work.join("superconductivity/manifest.json")) {
        let value: Value = serde_json::from_str(&manifest).unwrap();
        assert_ne!(
            value["validation"]["status"], "valid",
            "可保留失败证据，不得发布有效结果"
        );
    }
}

#[test]
#[ignore = "B08: Final State 被当作 Wannier 收敛证据，修复后取消 ignore"]
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
        "明确未收敛的日志不能被 Final State 覆盖：{result:?}"
    );
}
