//! 首条 nosplit 主路径直接调用 qeflow 执行库，EPW 只拥有严格快照和交付检查。
use super::{
    manifest::{PhResponse, QPoint},
    phonon,
    qe_config::QeSettings,
    qmap, save,
    scf::{self, ScfResult},
    xml,
};
use crate::foundation::{
    config::{Config, PhMode},
    slurm::shell_quote,
    stage::Backend,
    store::{self, Identity},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt, path::Path};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    digest: String,
    files: Vec<Identity>,
}
impl Request {
    fn verify(&self, digest: &str) -> Result<()> {
        ensure!(
            self.schema_version == 1 && self.digest == digest,
            "input_conflict: 共享声子请求发生变化"
        );
        for id in &self.files {
            id.verify()?;
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    energy_ry: f64,
    log: Identity,
    input: Identity,
}
impl Checkpoint {
    fn result(&self, qe: &QeSettings, work: &Path) -> Result<ScfResult> {
        self.log.verify()?;
        self.input.verify()?;
        save::verify_snapshot(work)?;
        xml::validate_save(
            &crate::foundation::layout::path(&work, "scf/snapshot"),
            &qe.config.qe.prefix,
            qe,
            Some(self.energy_ry),
        )?;
        Ok(ScfResult {
            energy_ry: self.energy_ry,
            snapshot_dir: crate::foundation::layout::path(&work, "scf/snapshot"),
            attempt_count: 1,
            success_count: 1,
        })
    }
}

pub fn run(
    cfg: &Config,
    qe: &QeSettings,
    backend: Backend,
    launcher: &[String],
    work: &Path,
) -> Result<(ScfResult, Vec<QPoint>, Vec<PhResponse>)> {
    ensure!(
        backend == Backend::Local,
        "allocation: 共享声子链只能在已登记分配内同步执行"
    );
    ensure!(
        cfg.phonon.mode == PhMode::Nosplit,
        "execution_failed: 首条共享声子主路径仅支持 phonon.mode=nosplit"
    );
    let kpath = Identity::capture(
        cfg.kpath
            .as_ref()
            .context("required: 共享声子流程需要 kpath 或配置目录中的 KPATH.in")?,
    )?;
    let dir = crate::foundation::layout::path(&work, "qe-phonon");
    let request_path = dir.join("shared-request.json");
    let digest = store::hash_bytes(&serde_json::to_vec(
        &json!({"config": cfg, "launcher": launcher}),
    )?);
    let existing: Option<Request> = if dir.exists() {
        let request: Request =
            store::read(&request_path).context("input_conflict: 未登记的共享声子目录，保留现场")?;
        request.verify(&digest)?;
        Some(request)
    } else {
        None
    };
    let post_dir = cfg
        .programs
        .ph
        .path
        .parent()
        .context("configuration: PH 缺少程序目录")?;
    let q2r =
        Identity::capture(&post_dir.join("q2r.x")).context("environment: PH 同目录缺少 q2r.x")?;
    let matdyn = Identity::capture(&post_dir.join("matdyn.x"))
        .context("environment: PH 同目录缺少 matdyn.x")?;
    let pseudo_ids: BTreeMap<_, _> = cfg
        .pseudos
        .iter()
        .map(|(el, path)| Ok((el.clone(), Identity::capture(path)?)))
        .collect::<Result<_>>()?;
    if existing.is_none() {
        fs::create_dir_all(dir.join("bin"))?;
        fs::create_dir(dir.join("pp"))?;
        for (el, id) in &pseudo_ids {
            let target = dir.join("pp").join(format!("{el}.UPF"));
            fs::copy(&id.path, &target)?;
            ensure!(
                store::hash_file(&target)? == id.sha256,
                "input_conflict: 赝势复制不完整"
            );
        }
        for (name, program, args) in [
            (
                "pw.x",
                &cfg.programs.pw.path,
                cfg.programs.pw.args.as_slice(),
            ),
            (
                "ph.x",
                &cfg.programs.ph.path,
                cfg.programs.ph.args.as_slice(),
            ),
            ("q2r.x", &q2r.path, &[][..]),
            ("matdyn.x", &matdyn.path, &[][..]),
        ] {
            let mut argv = launcher.to_vec();
            argv.push(program.to_string_lossy().into_owned());
            argv.extend_from_slice(args);
            let text = format!(
                "#!/bin/bash\nset -euo pipefail\nexec {} \"$@\"\n",
                argv.iter()
                    .map(|s| shell_quote(s))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            let path = dir.join("bin").join(name);
            store::atomic_bytes(&path, text.as_bytes())?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
        }
        fs::copy(&kpath.path, dir.join("KPATH.in"))?;
    }
    let mut input = json!({
        "atomic_masses": cfg.atomic_masses,
        "pp_dir": dir.join("pp"),
        "qe": {"executable_path": shell_quote(&dir.join("bin").to_string_lossy()), "lattice_mode": "cell_parameters"},
        "slurm": {"header": ""},
        "qe_phonon": {
            "retain_epw_data": true, "work_dir": dir, "prefix": cfg.qe.prefix,
            "scf": {"calculation": "scf", "outdir": "out", "prefix": cfg.qe.prefix,
                "kpoints": cfg.qe.scf.k_grid, "k_shift": cfg.qe.scf.k_shift,
                "ecutwfc": cfg.qe.ecutwfc_ry, "ecutrho": cfg.qe.ecutrho_ry,
                "occupations": cfg.qe.occupations, "conv_thr": cfg.qe.scf.conv_thr_ry},
            "ph": {"qpoints": cfg.phonon.q_grid, "tr2_ph": cfg.phonon.tr2_ph},
            "q2r": {"zasr": "simple"},
            "phband": {"kpath_file": "KPATH.in"},
            "phdos": {"qpoints": cfg.interpolation.q_grid}
        }
    });
    if let Some(smearing) = &cfg.qe.smearing {
        input["qe_phonon"]["scf"]["smearing"] = serde_json::to_value(smearing)?;
    }
    if let Some(degauss) = cfg.qe.degauss_ry {
        input["qe_phonon"]["scf"]["degauss"] = json!(degauss);
    }
    let config = dir.join("input.qeflow.toml");
    if existing.is_none() {
        store::atomic_bytes(&config, toml::to_string_pretty(&input)?.as_bytes())?;
        let mut files = vec![
            Identity::capture(&config)?,
            Identity::capture(&dir.join("KPATH.in"))?,
        ];
        for name in ["pw.x", "ph.x", "q2r.x", "matdyn.x"] {
            files.push(Identity::capture(&dir.join("bin").join(name))?);
        }
        for element in cfg.pseudos.keys() {
            files.push(Identity::capture(
                &dir.join("pp").join(format!("{element}.UPF")),
            )?);
        }
        store::write(
            &request_path,
            &Request {
                schema_version: 1,
                digest: digest.clone(),
                files,
            },
        )?;
    }
    let checkpoint = dir.join("scf-checkpoint.json");
    let mut scf_result = if checkpoint.exists() {
        Some(store::read::<Checkpoint>(&checkpoint)?.result(qe, work)?)
    } else {
        None
    };
    qeflow::run_nosplit_with_scf_checkpoint(
        config.to_string_lossy().into_owned(),
        cfg.structure.to_string_lossy().into_owned(),
        &mut |root| {
            if scf_result.is_some() {
                store::read::<Checkpoint>(&checkpoint)?.result(qe, work)?;
                return Ok(());
            }
            let energy = scf::parse_scf_energy(&fs::read_to_string(root.join("scf.out"))?)?;
            xml::validate_save(&root.join("out"), &cfg.qe.prefix, qe, Some(energy))?;
            let snapshot_dir = save::snapshot(&root.join("out"), work)?;
            store::write(
                &checkpoint,
                &Checkpoint {
                    energy_ry: energy,
                    log: Identity::capture(&root.join("scf.out"))?,
                    input: Identity::capture(&root.join("scf.in"))?,
                },
            )?;
            scf_result = Some(ScfResult {
                energy_ry: energy,
                snapshot_dir,
                attempt_count: 1,
                success_count: 1,
            });
            Ok(())
        },
    )
    .context("execution_failed: qeflow 声子流程失败")?;
    store::read::<Request>(&request_path)?.verify(&digest)?;
    kpath.verify()?;
    q2r.verify()?;
    matdyn.verify()?;
    for id in pseudo_ids.values() {
        id.verify()?;
    }
    let archive = qf_phonon::epw::export_nosplit(&dir, &dir.join("out"), &cfg.qe.prefix)?;
    let export: qf_phonon::epw::Manifest = store::read(&archive.join("manifest.json"))?;
    let geometry = phonon::geometry(qe, work)?;
    let points = qmap::parse_dyn0(
        &dir.join(format!("{}.dyn0", cfg.qe.prefix)),
        &geometry,
        cfg.phonon.q_grid,
        false,
    )?;
    ensure!(
        points.len() == export.qpoints,
        "data_invalid: 归档与严格 q 映射不符"
    );
    let control = dir
        .join("out/_ph0")
        .join(format!("{}.phsave/control_ph.xml", cfg.qe.prefix));
    super::native_ph::control(&control, &points)?;
    let responses = points
        .iter()
        .map(|q| phonon::response(qe, &dir, q, &control, &geometry))
        .collect::<Result<_>>()?;
    store::write(
        &dir.join("shared-source-identities.json"),
        &json!({"schema_version": 1, "kpath": kpath, "q2r": q2r, "matdyn": matdyn, "pseudos": pseudo_ids}),
    )?;
    Ok((
        scf_result.context("data_invalid: 共享声子链缺少 SCF 检查点")?,
        points,
        responses,
    ))
}
