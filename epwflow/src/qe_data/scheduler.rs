use super::super::foundation::{budget, config, snapshot, stage::Backend, store};
use super::{
    collect_ph, compare,
    manifest::{Fingerprints, QeManifest, ScfRecord, Validation},
    nscf, phonon, qe_acceptance,
    qe_config::QeSettings,
    reuse, scf,
};
use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

/// 在共享预算账本上预留本运行资源；超限拒绝（FR-010）。
fn charge_budget(binding: &snapshot::RunBinding, cfg: &config::Config) -> Result<String> {
    let r = &cfg.resources;
    budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |ledger| {
        budget::reserve(
            ledger,
            &format!("qe-data:{}", binding.run_id),
            r.nodes,
            r.mpi,
            r.wall_seconds,
            r.storage_bytes,
        )
    })
    .context("budget_denied: qe-data 预算预留失败")
}

/// 执行完整 QE 数据链到 qe-data 交付（FR-001/003~010）：
/// 一次 SCF → 完整 NSCF → PH 初始化 → 分 q → 归集 → 校验 → 发布清单。
pub fn run(
    binding: &snapshot::RunBinding,
    cfg: &config::Config,
    qe: &QeSettings,
    backend: Backend,
    launcher: &[String],
    work: &Path,
) -> Result<QeManifest> {
    let _token = charge_budget(binding, cfg)?;
    let result = run_registered(binding, cfg, qe, backend, launcher, work)?;
    budget::with_ledger(
        &cfg.budget.ledger,
        &cfg.budget.campaign_id,
        budget::reconcile,
    )?;
    Ok(result)
}

/// 由统一入口在已登记分配内调用；整份分配只计费一次。
pub fn run_registered(
    binding: &snapshot::RunBinding,
    _cfg: &config::Config,
    qe: &QeSettings,
    backend: Backend,
    launcher: &[String],
    work: &Path,
) -> Result<QeManifest> {
    // 1. 一次 SCF 与完整快照。
    let scf_res = scf::run_scf(qe, work, launcher, backend)?;
    // 2. 完整均匀 NSCF。
    let states = nscf::run_nscf(qe, work, launcher, backend)?;
    // nosplit 直接产生全网格；其余模式初始化成功后才展开独立 q 任务。
    let (qpoints, responses) = match qe.config.phonon.mode {
        super::super::foundation::config::PhMode::Nosplit => {
            phonon::run_nosplit(qe, work, launcher, backend)?
        }
        _ => {
            let qpoints = phonon::run_init(qe, work, launcher, backend)?;
            let mut responses = Vec::new();
            for (i, q) in qpoints.iter().enumerate() {
                responses.push(phonon::run_q(qe, work, launcher, backend, q, i + 1)?);
            }
            (qpoints, responses)
        }
    };
    // 5. 响应数据完整归集。
    let collected = collect_ph::collect(
        work,
        &qpoints,
        &responses,
        &qe.config.qe.prefix,
        &phonon::geometry(qe, work)?,
    )?;
    collect_ph::write_q_index_map(work, &qpoints)?;

    // 6. 构建指纹（来源不可变，来自绑定）。
    let fingerprints = Fingerprints {
        structure_sha256: binding.structure.sha256.clone(),
        pseudo_sha256: binding
            .pseudos
            .iter()
            .map(|(k, v)| (k.clone(), v.sha256.clone()))
            .collect(),
        program_sha256: binding
            .programs
            .iter()
            .map(|(k, v)| (k.clone(), v.sha256.clone()))
            .collect(),
    };
    let scf_record = ScfRecord {
        total_energy_ry: scf_res.energy_ry,
        attempt_count: scf_res.attempt_count,
        success_count: scf_res.success_count,
        snapshot_dir: scf_res.snapshot_dir.clone(),
    };

    // 7. 本地完整性校验（非验收替换）。
    let mut validation = Validation {
        status: "valid".to_string(),
        checks: Vec::new(),
        units: BTreeMap::new(),
        thresholds: BTreeMap::new(),
        evidence: Vec::new(),
    };
    validation
        .units
        .insert("scf_energy".to_string(), "Ry".to_string());
    validation
        .units
        .insert("kpoints".to_string(), "count".to_string());
    validation
        .units
        .insert("qpoints".to_string(), "count".to_string());
    validation.check(
        "scf_energy_finite",
        scf_res.energy_ry.is_finite(),
        &format!("E={}", scf_res.energy_ry),
    );
    validation.check(
        "nscf_complete",
        !states.kpoints.is_empty() && states.nbnd > 0,
        &format!("nbnd={} k={}", states.nbnd, states.kpoints.len()),
    );
    validation.check(
        "qpoints_present",
        !qpoints.is_empty() && qpoints.len() == collected.total_q,
        &format!("nq={}", qpoints.len()),
    );
    validation.check(
        "phonon_complete",
        collected.responses.iter().all(|r| r.perturbations > 0),
        "所有响应势扰动数已确定",
    );
    validation.check(
        "artifact_integrity",
        !collected.artifacts.is_empty(),
        &format!("artifacts={}", collected.artifacts.len()),
    );

    let mut artifacts = collected.artifacts;
    for (stage, root) in [
        ("scf", scf_res.snapshot_dir.clone()),
        ("nscf", work.join("nscf/out")),
    ] {
        for f in super::save::files(&root)? {
            let path = root.join(&f.relative_path);
            artifacts.push(super::manifest::Artifact {
                role: format!("{stage}_{}", f.relative_path.display()),
                relative_path: path.strip_prefix(work)?.into(),
                byte_size: f.byte_size,
                sha256: f.sha256,
                producer_stage: stage.into(),
                input_digest: String::new(),
                format_version: "qe7.4.1-portable".into(),
            });
        }
    }
    let identity = work.join("scf/snapshot-files.json");
    artifacts.push(super::manifest::Artifact {
        role: "scf_snapshot_identity".into(),
        relative_path: identity.strip_prefix(work)?.into(),
        byte_size: std::fs::metadata(&identity)?.len(),
        sha256: store::hash_file(&identity)?,
        producer_stage: "scf".into(),
        input_digest: String::new(),
        format_version: "1".into(),
    });
    let manifest = QeManifest {
        schema_version: 2,
        artifact_root: Some(work.canonicalize()?),
        artifacts,
        run_id: binding.run_id.clone(),
        fingerprints,
        scf: Some(scf_record),
        states: Some(states),
        qpoints,
        phonon_files: collected.responses,
        validation: validation.clone(),
    };

    // 8. 仅当全部必需本地校验通过才发布可供下一阶段消费的清单。
    if manifest.validation.all_passed() {
        manifest.verify()?;
        let out = work.join("qe-data").join("manifest.json");
        store::write(&out, &manifest)?;
    } else {
        for c in &manifest.validation.checks {
            eprintln!("qe-data validation failed: {} = {}", c.name, c.passed);
        }
        bail!(
            "data_invalid: qe-data 本地校验未通过，拒绝发布清单: {}",
            validation.status
        );
    }

    Ok(manifest)
}

/// 阶段边界：仅执行到 qe-data 交付；后续阶段未接入时明确报错（FR-001）。
pub fn unsupported_beyond_qe_data() -> Result<()> {
    bail!("not_implemented: 全流程后续阶段（F3–F5）尚未接入，submit/run 越过 qe-data 不可用");
}

/// qe-data 阶段自检（FR-010 依赖/预算/失败边界）：核对绑定来源未变。
pub fn verify_binding(binding: &snapshot::RunBinding) -> Result<()> {
    binding
        .verify()
        .context("input_conflict: 绑定来源在执行前已被修改")
}

/// 内部最小参考数值对照（FR-011）的便捷入口：把当前清单与参考清单比较。
pub fn diff_with_reference(
    current: &QeManifest,
    reference: &QeManifest,
) -> compare::ComparisonReport {
    compare::compare_manifests(current, reference)
}

/// 把导入作为可复用阶段登记（FR-009），随后返回导入报告路径信息。
pub fn register_import(source_work: &Path, work: &Path) -> Result<reuse::ImportReport> {
    reuse::import_from_run(source_work, work)
}

/// 记录真实验收证据（FR-012），落盘到 work/qe-data/acceptance.json。
pub fn record_acceptance(work: &Path, record: &qe_acceptance::AcceptanceRecord) -> Result<()> {
    ensure!(
        record.require_evidence().is_ok(),
        "data_invalid: 验收记录缺少真实证据，拒绝落盘冒充"
    );
    qe_acceptance::write(&work.join("qe-data").join("acceptance.json"), record)
}
