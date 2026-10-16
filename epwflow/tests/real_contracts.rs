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
