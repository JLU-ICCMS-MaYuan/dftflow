//! 电子参考的原生任务收据；后处理失败不重算已校验的 PW 能带。
use super::{
    config::{Config, Program},
    stages::{self, Step},
    store::{self, Identity},
    workspace_science::{post_program, qe_run},
};
use crate::qe_data::{qe_config::QeSettings, save};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};
mod pw_reuse;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeReceipt {
    version: u32,
    key: String,
    input: Identity,
    outputs: Vec<Identity>,
    save_root: Option<PathBuf>,
    provenance: Vec<Identity>,
}

impl NativeReceipt {
    fn verify(&self) -> Result<()> {
        ensure!(
            self.version == 1 && !self.outputs.is_empty(),
            "data_invalid: Lack of evidence for electronic mission receipts"
        );
        self.input.verify()?;
        for id in self.outputs.iter().chain(&self.provenance) {
            id.verify()?;
        }
        if let Some(root) = &self.save_root {
            let mut expected = self
                .outputs
                .iter()
                .filter(|id| !qf_io::qe_payload::unchecked(&id.path))
                .filter_map(|id| id.path.strip_prefix(root).ok().map(Path::to_path_buf))
                .collect::<Vec<_>>();
            expected.sort();
            let actual = save::files(root)?
                .into_iter()
                .map(|f| f.relative_path)
                .collect::<Vec<_>>();
            ensure!(
                actual == expected,
                "input_conflict: Change in the collection of electronic preservation tree files {}",
                root.display()
            );
        }
        Ok(())
    }
    fn output_key(&self) -> Result<String> {
        Ok(store::hash_bytes(&serde_json::to_vec(
            &self
                .outputs
                .iter()
                .map(|id| (&id.path, &id.sha256, id.bytes))
                .collect::<Vec<_>>(),
        )?))
    }
}

fn read_receipt(cfg: &Config, task: &str) -> Result<Option<NativeReceipt>> {
    let label = format!("Electronic substep · {task}");
    let Some(outputs) = super::readable_state::completed_files(cfg, Step::Electronic, &label)?
    else {
        return Ok(None);
    };
    let Some(previous) = super::readable_state::checkpoint_config(cfg, Step::Electronic, &label)?
    else {
        return Ok(None);
    };
    if key(&previous, task)? != key(cfg, task)? {
        return Ok(None);
    }
    let dir = Step::Electronic.directory(cfg).join(if task == "dos" {
        "qe_eledos"
    } else {
        "qe_eband"
    });
    let input = dir.join(match task {
        "pw-bands" => "eband.in",
        "bands" => "elebanddata.in",
        _ => "eletdos.in",
    });
    if !input.is_file() {
        return Ok(None);
    }
    let r = NativeReceipt {
        version: 1,
        key: key(cfg, task)?,
        input: Identity::capture(&input)?,
        outputs,
        save_root: if task == "bands" {
            None
        } else if task=="dos" {
            Some(super::layout::path(&cfg.work_dir,"nscf/out").join(format!("{}.save",cfg.qe.prefix)))
        } else {
            Some(dir.join(format!("out/{}.save", cfg.qe.prefix)))
        },
        provenance: vec![],
    };
    r.verify()?;
    match task {
        "pw-bands" => validate_pw(cfg)?,
        "bands" => {
            qf_io::qe_completion::checked_log(&super::scientific_logs::output(&dir, "elebanddata"), false)?;
            save::nonempty(&dir.join(format!("{}_band", cfg.qe.prefix)))?;
            validate_table(&dir.join(format!("{}_band.gnu", cfg.qe.prefix)), false)?;
        }
        "dos" => {
            qf_io::qe_completion::checked_log(&super::scientific_logs::output(&dir, "eletdos"), false)?;
            validate_table(&dir.join(format!("{}.tdos", cfg.qe.prefix)), true)?;
            let saved=fs::read_to_string(dir.join("eletdos.in"))?;
            let source=super::layout::path(&cfg.work_dir,"nscf/out");
            let relative=super::workspace_data::relative(&dir,&source)?;
            ensure!(saved.contains(relative.to_string_lossy().as_ref()) || saved.contains(source.to_string_lossy().as_ref()),
                "input_conflict: DOS does not reference its native NSCF source");
        }
        _ => unreachable!(),
    }
    Ok(Some(r))
}
fn validate_table(path: &Path, increasing: bool) -> Result<()> {
    let mut previous = None;
    let mut count = 0;
    for line in fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
    {
        let row = line
            .split_whitespace()
            .map(str::parse::<f64>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(
            row.len() >= 2 && row.iter().all(|v| v.is_finite()),
            "data_invalid: Illegal electronic data {}",
            path.display()
        );
        if increasing {
            ensure!(
                previous.is_none_or(|p| row[0] > p),
                "data_invalid: DOS The energy axis is not increasing"
            );
        }
        previous = Some(row[0]);
        count += 1;
    }
    ensure!(count > 1, "data_invalid: Electronic data is empty {}", path.display());
    Ok(())
}
fn persist_receipt(cfg: &Config, task: &str, r: &NativeReceipt) -> Result<()> {
    super::readable_state::complete_files(
        cfg,
        Step::Electronic,
        &format!("Electronic substep · {task}"),
        &r.outputs
            .iter()
            .map(|id| id.path.clone())
            .collect::<Vec<_>>(),
    )
}

/// 只剔除已支持的 QE 资源参数；其余原生选项参与 PW 任务身份。
fn scientific_args(program: &Program) -> Vec<String> {
    let flags = [
        "-nk",
        "-npool",
        "--npool",
        "-npools",
        "-ni",
        "-nimage",
        "--nimage",
        "-nt",
        "-ntg",
        "-ntask_groups",
        "-nb",
        "-nband",
        "-nbgrp",
        "-nd",
        "-ndiag",
        "-northo",
        "-pd",
        "-use_pd",
        "-pencil_decomposition",
        "-use_pencil_decomposition",
    ];
    let mut result = vec![];
    let mut args = program.args.iter();
    while let Some(arg) = args.next() {
        if flags.contains(&arg.as_str()) {
            args.next();
        } else {
            result.push(arg.clone());
        }
    }
    result
}

fn key(cfg: &Config, task: &str) -> Result<String> {
    let (program, input, upstream) = match task {
        "pw-bands" => (
            cfg.programs.pw.clone(),
            String::new(),
            json!(stages::upstream(cfg, Step::Electronic)?),
        ),
        "bands" => {
            let pw = read_receipt(cfg, "pw-bands")?.context("data_invalid: bands Missing PW Receipts")?;
            (
                post_program(cfg, "bands")?,
                fs::read_to_string(
                    Step::Electronic
                        .directory(cfg)
                        .join("qe_eband/elebanddata.in"),
                )?,
                json!(pw.output_key()?),
            )
        }
        "dos" => (
            post_program(cfg, "dos")?,
            format!(
                "sigma_ry={:.15}",
                cfg.native_value::<f64>(&[
                    super::native_config::ELECTRONIC,
                    "qe_eledos",
                    "dos",
                    "DOS",
                    "degauss"
                ])?
            ),
            json!(stages::upstream(cfg, Step::Electronic)?),
        ),
        _ => anyhow::bail!("configuration: Unknown electronic task {task}"),
    };
    Ok(store::hash_bytes(&serde_json::to_vec(&(
        1,
        task,
        cfg.native_value::<serde_json::Value>(&match task {
            "pw-bands" => vec![super::native_config::ELECTRONIC, "qe_eband", "pw"],
            "bands" => vec![super::native_config::ELECTRONIC, "qe_eband", "bands"],
            _ => vec![super::native_config::ELECTRONIC, "qe_eledos", "dos"],
        })?,
        if task == "pw-bands" {
            Some((
                cfg.native_value::<u32>(&[
                    super::native_config::ELECTRONIC,
                    "workflow",
                    "points_per_edge",
                ])?,
                cfg.kpath
                    .as_ref()
                    .map(|p| store::hash_file(p))
                    .transpose()?,
            ))
        } else {
            None
        },
        upstream,
        store::hash_file(&program.path)?,
        if task == "pw-bands" {
            scientific_args(&program)
        } else {
            program.args.clone()
        },
        input,
    ))?))
}

fn reusable(cfg: &Config, task: &str) -> Result<bool> {
    let Some(receipt) = read_receipt(cfg, task)? else {
        return Ok(false);
    };
    if !receipt.input.path.try_exists()? {
        return Ok(false);
    }
    if receipt.key != key(cfg, task)? {
        return Ok(false);
    }
    if !store::identity_available(&receipt.input)? {
        return Ok(false);
    }
    let mut available = true;
    for id in receipt.outputs.iter().chain(&receipt.provenance) {
        if !store::identity_available(id)? {
            available = false;
        }
    }
    if !available {
        return Ok(false);
    }
    receipt.verify()?;
    Ok(true)
}

pub(super) fn completed(cfg: &Config) -> Result<bool> {
    for task in ["pw-bands", "bands", "dos"] {
        if !reusable(cfg, task)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn capture(
    cfg: &Config,
    task: &str,
    input: &Path,
    paths: &[PathBuf],
    save_root: Option<PathBuf>,
    provenance: Vec<Identity>,
) -> Result<NativeReceipt> {
    // 新原生任务只绑定合并的 .out；旧收据和非原生执行仍要求独立 .err。
    let merged_stderr = if super::native_jobs::enabled() {
        super::native_jobs::active_task(cfg)?
            .filter(|task| {
                task.schema_version == 3 && Some(task.directory.as_path()) == input.parent()
            })
            .map(|task| task.directory.join(format!("{}.err", task.stem)))
    } else {
        None
    };
    let mut outputs = paths
        .iter()
        .filter(|path| merged_stderr.as_ref() != Some(*path))
        .map(|p| Identity::capture(p))
        .collect::<Result<Vec<_>>>()?;
    if let Some(root) = &save_root {
        for file in save::files(root)? {
            outputs.push(Identity::capture(&root.join(file.relative_path))?);
        }
    }
    let r = NativeReceipt {
        version: 1,
        key: key(cfg, task)?,
        input: Identity::capture(input)?,
        outputs,
        save_root,
        provenance,
    };
    r.verify()?;
    Ok(r)
}

fn logs(dir: &Path, stem: &str) -> Vec<PathBuf> {
    ["out", "err"]
        .iter()
        .map(|ext| dir.join(format!("{stem}.{ext}")))
        .collect()
}

/// 重试只覆盖该原生任务日志及输出；不触碰成功 PW 的保存树。
fn archive_task(_cfg: &Config, _task: &str, paths: &[PathBuf]) -> Result<()> {
    for p in paths {
        if p.is_dir() {
            fs::remove_dir_all(p)?;
        } else if p.exists() {
            fs::remove_file(p)?;
        }
    }
    Ok(())
}

fn validate_pw(cfg: &Config) -> Result<()> {
    let dir = Step::Electronic.directory(cfg).join("qe_eband");
    save::verify_snapshot(&cfg.work_dir)?;
    qf_io::qe_completion::checked_log(&dir.join("eband.out"), false)?;
    let qe = QeSettings::from_config(cfg)?;
    let actual_input = fs::read_to_string(dir.join("eband.in"))?;
    let requested = crate::interpolation::native_sampling::pw(&actual_input)?;
    pw_reuse::validate(&actual_input, &qe.path_input("./out", &requested)?,
        &dir.join(format!("out/{}.save", cfg.qe.prefix)), &cfg.pseudos)?;
    let geo = crate::qe_data::xml::validate_save(&dir.join("out"), &cfg.qe.prefix, &qe, None)?;
    let states = crate::qe_data::nscf::parse_xml(&dir.join(format!("out/{}.save/data-file-schema.xml", cfg.qe.prefix)))?;
    let points: Vec<[f64;3]> = store::read(&dir.join("path-points.json"))?;
    ensure!(points.len()==requested.len()
        && points.iter().flatten().zip(requested.iter().flatten()).all(|(a,b)|(a-b).abs()<1e-10),
        "input_conflict: saved QE path differs from its own input");
    let reference = crate::interpolation::electronic_sampling::plan(cfg)?;
    crate::interpolation::comparison::native_path(&reference.path, &states.kpoints, &reference.reciprocal)?;
    ensure!(
        states.nbnd == cfg.qe.nscf.nbnd as usize && states.kpoints.len() == points.len(),
        "data_invalid: PW Can carry a number or k The points don't match"
    );
    for (actual, expected) in states.kpoints.iter().zip(points) {
        ensure!(
            geo.crystal(*actual)
                .iter()
                .zip(expected)
                .all(|(a, b)| (a - b).abs() < 1e-8),
            "data_invalid: PW It's practical k Point does not match the path"
        );
    }
    Ok(())
}

/// 只读检查现有原生 PW 结果，不通过历史尝试或请求目录追溯。
pub fn inspect_legacy(cfg: &Config) -> Result<NativeReceipt> {
    validate_pw(cfg)?;
    let dir = Step::Electronic.directory(cfg).join("qe_eband");
    let mut paths = logs(&dir, "eband");
    paths.retain(|p| p.exists());
    paths.push(dir.join("path-points.json"));
    capture(
        cfg,
        "pw-bands",
        &dir.join("eband.in"),
        &paths,
        Some(dir.join(format!("out/{}.save", cfg.qe.prefix))),
        vec![],
    )
}

fn prepare_pw(cfg: &Config) -> Result<()> {
    if let Some(receipt) = read_receipt(cfg, "pw-bands")? {
        ensure!(
            receipt.key == key(cfg, "pw-bands")?,
            "input_conflict: PW It's not compatible with native options or upstream status"
        );
        return receipt.verify();
    }
    let dir = Step::Electronic.directory(cfg).join("qe_eband");
    let successful = dir.join("eband.exitcode").is_file()
        && fs::read_to_string(dir.join("eband.exitcode"))?.trim() == "0";
    let done = dir.join("eband.out").is_file()
        && qf_io::qe_completion::checked_log(&dir.join("eband.out"), false).is_ok();
    if successful || done {
        let receipt = inspect_legacy(cfg)?;
        persist_receipt(cfg, "pw-bands", &receipt)?;
        super::readable_state::report_reuse(cfg, super::stages::Step::Electronic, Some("pw-bands (validated legacy snapshot)"));
    }
    Ok(())
}
fn path_points(cfg: &Config) -> Result<Vec<[f64; 3]>> {
    let path = cfg.kpath.as_ref().context("required: Yes KPATH.in")?;
    let segments =
        qf_input::electronic::parse_kpath_segments(path.to_str().context("KPATH Not UTF-8")?)?;
    let intervals = cfg.comparison.clone().unwrap_or_default().points_per_edge;
    let mut points = vec![];
    for segment in segments {
        for pair in segment.windows(2) {
            for n in 0..intervals {
                points.push(std::array::from_fn(|i| {
                    pair[0].0[i]
                        + (pair[1].0[i] - pair[0].0[i]) * f64::from(n) / f64::from(intervals)
                }));
            }
        }
        points.push(segment.last().unwrap().0);
    }
    Ok(points)
}

pub(super) fn inputs(cfg: &Config, qe: &QeSettings) -> Result<()> {
    let root = Step::Electronic.directory(cfg);
    let bands = root.join("qe_eband");
    let dos = root.join("qe_eledos");
    fs::create_dir_all(&bands)?;
    fs::create_dir_all(&dos)?;
    let points = path_points(cfg)?;
    let input = qe.path_input("./out", &points)?;
    if !super::native_jobs::enabled() {
        prepare_pw(cfg)?;
    }
    if read_receipt(cfg, "pw-bands")?.is_none() && !super::native_jobs::continuing(&bands, "eband")?
    {
        fs::write(bands.join("eband.in"), input)?;
        store::write(&bands.join("path-points.json"), &points)?;
    }
    fs::write(
        bands.join("elebanddata.in"),
        format!(
            "&bands\n prefix='{}'\n outdir='./out'\n filband='{}_band'\n lsym=.false.\n/\n",
            cfg.qe.prefix, cfg.qe.prefix
        ),
    )?;
    cfg.configure_file(
        &[super::native_config::ELECTRONIC, "qe_eband", "bands"],
        &bands.join("elebanddata.in"),
    )?;
    dos_inputs(cfg)?;
    Ok(())
}

fn dos_inputs(cfg: &Config) -> Result<()> {
    let dos = Step::Electronic.directory(cfg).join("qe_eledos");
    fs::create_dir_all(&dos)?;
    let shared: qf_config::Config = serde_json::from_value(
        json!({"qe_eledos":{"CONTROL":{"outdir":super::workspace_data::relative(&dos,&super::layout::path(&cfg.work_dir,"nscf/out"))?.to_string_lossy()},"eledos":{"emin":-100,"emax":100,"DeltaE":0.01}}}),
    )?;
    qf_input::electronic::write_eledos_inputs(dos.to_str().unwrap(), &shared, &cfg.qe.prefix)?;
    let file = dos.join("eletdos.in");
    let text = fs::read_to_string(&file)?;
    let sigma: f64 = cfg.native_value(&[
        super::native_config::ELECTRONIC,
        "qe_eledos",
        "dos",
        "DOS",
        "degauss",
    ])?;
    fs::write(
        file,
        text.replacen("&dos", &format!("&dos\n ngauss=0, degauss={sigma:.15},"), 1),
    )?;
    cfg.configure_file(
        &[super::native_config::ELECTRONIC, "qe_eledos", "dos"],
        &dos.join("eletdos.in"),
    )?;
    Ok(())
}

pub(super) fn run_leaf(cfg: &Config, name: &str) -> Result<()> {
    let dir = Step::Electronic.directory(cfg).join(name);
    match name {
        "qe_eband" => {
            inputs(cfg, &QeSettings::from_config(cfg)?)?;
            if reusable(cfg, "pw-bands")? {
                super::readable_state::report_reuse(cfg, super::stages::Step::Electronic, Some("pw-bands"));
            } else {
                let mut old = logs(&dir, "eband");
                old.push(dir.join("out"));
                if !super::native_jobs::continuing(&dir, "eband")? {
                    archive_task(cfg, "pw-bands", &old)?;
                    save::stage_ground_state(&cfg.work_dir, &dir.join("out"), &cfg.qe.prefix)?;
                }
                qe_run(cfg, &dir, &cfg.programs.pw, "eband.in", "eband")?;
                validate_pw(cfg)?;
                let mut paths = logs(&dir, "eband");
                paths.push(dir.join("path-points.json"));
                let receipt = capture(
                    cfg,
                    "pw-bands",
                    &dir.join("eband.in"),
                    &paths,
                    Some(dir.join(format!("out/{}.save", cfg.qe.prefix))),
                    vec![],
                )?;
                persist_receipt(cfg, "pw-bands", &receipt)?;
            }
            // 路径本征值一经核验就画范围图，不等待 bands.x、DOS 或投影。
            super::band_ranges::generate(cfg, Step::Electronic)?;
            if reusable(cfg, "bands")? {
                super::readable_state::report_reuse(cfg, super::stages::Step::Electronic, Some("bands"));
            } else {
                let mut paths = logs(&dir, "elebanddata");
                paths.extend([
                    dir.join(format!("{}_band", cfg.qe.prefix)),
                    dir.join(format!("{}_band.gnu", cfg.qe.prefix)),
                ]);
                if !super::native_jobs::continuing(&dir, "elebanddata")? {
                    archive_task(cfg, "bands", &paths)?;
                }
                qe_run(
                    cfg,
                    &dir,
                    &post_program(cfg, "bands")?,
                    "elebanddata.in",
                    "elebanddata",
                )?;
                let actual = super::scientific_logs::output(&dir, "elebanddata");
                paths.splice(..2, ["out", "err"].map(|ext| actual.with_extension(ext)));
                for p in &paths {
                    if p.extension().is_none_or(|ext| ext != "err") {
                        save::nonempty(p)?;
                    }
                }
                // 后处理不得改写它读取的 portable PW 数据。
                read_receipt(cfg, "pw-bands")?
                    .context("data_invalid: Missing PW Receipts")?
                    .verify()?;
                let receipt = capture(
                    cfg,
                    "bands",
                    &dir.join("elebanddata.in"),
                    &paths,
                    None,
                    vec![],
                )?;
                persist_receipt(cfg, "bands", &receipt)?;
            }
        }
        "qe_eledos" => {
            dos_inputs(cfg)?;
            if reusable(cfg, "dos")? {
                super::readable_state::report_reuse(cfg, super::stages::Step::Electronic, Some("dos"));
                return super::workspace_qe_plot::electronic(cfg, name);
            }
            let mut paths = logs(&dir, "eletdos");
            paths.push(dir.join(format!("{}.tdos", cfg.qe.prefix)));
            if !super::native_jobs::continuing(&dir, "eletdos")? {
                archive_task(cfg, "dos", &paths)?;
            }
            let out = super::layout::path(&cfg.work_dir,"nscf/out");
            qe_run(cfg, &dir, &post_program(cfg, "dos")?, "eletdos.in", "eletdos")?;
            let actual = super::scientific_logs::output(&dir, "eletdos");
            paths.splice(..2, ["out", "err"].map(|ext| actual.with_extension(ext)));
            save::nonempty(&dir.join(format!("{}.tdos", cfg.qe.prefix)))?;
            let receipt = capture(
                cfg,
                "dos",
                &dir.join("eletdos.in"),
                &paths,
                Some(out.join(format!("{}.save", cfg.qe.prefix))),
                vec![],
            )?;
            persist_receipt(cfg, "dos", &receipt)?;
        }
        _ => anyhow::bail!("configuration: Unknown electron subphase {name}"),
    }
    super::workspace_qe_plot::electronic(cfg, name)
}
