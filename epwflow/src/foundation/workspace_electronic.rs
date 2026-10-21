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
            "data_invalid: 电子任务收据缺证据"
        );
        self.input.verify()?;
        for id in self.outputs.iter().chain(&self.provenance) {
            id.verify()?;
        }
        if let Some(root) = &self.save_root {
            let mut expected = self
                .outputs
                .iter()
                .filter_map(|id| id.path.strip_prefix(root).ok().map(Path::to_path_buf))
                .collect::<Vec<_>>();
            expected.sort();
            let actual = save::files(root)?
                .into_iter()
                .map(|f| f.relative_path)
                .collect::<Vec<_>>();
            ensure!(
                actual == expected,
                "input_conflict: 电子保存树文件集合发生变化 {}",
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

fn receipt_path(cfg: &Config, task: &str) -> PathBuf {
    cfg.work_dir
        .join("3.qe-electronic/receipts")
        .join(format!("{task}.json"))
}
fn read_receipt(cfg: &Config, task: &str) -> Result<Option<NativeReceipt>> {
    let path = receipt_path(cfg, task);
    if path.is_file() {
        Ok(Some(store::read(&path)?))
    } else {
        Ok(None)
    }
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
            let pw = read_receipt(cfg, "pw-bands")?.context("data_invalid: bands 缺 PW 收据")?;
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
                cfg.comparison
                    .clone()
                    .unwrap_or_default()
                    .electronic_sigma_ev
                    / 13.605693122994
            ),
            json!(stages::upstream(cfg, Step::Electronic)?),
        ),
        _ => anyhow::bail!("configuration: 未知电子任务 {task}"),
    };
    Ok(store::hash_bytes(&serde_json::to_vec(&(
        1,
        task,
        stages::scientific_key(cfg, Step::Electronic)?,
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
    if !receipt.input.path.try_exists()? { return Ok(false); }
    if receipt.key != key(cfg, task)? {
        return Ok(false);
    }
    if !store::identity_available(&receipt.input)? { return Ok(false); }
    let mut available = true;
    for id in receipt.outputs.iter().chain(&receipt.provenance) {
        if !store::identity_available(id)? { available = false; }
    }
    if !available { return Ok(false); }
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

pub(super) fn compatible_partial(cfg: &Config) -> Result<bool> {
    if let Some(receipt) = read_receipt(cfg, "pw-bands")? {
        if receipt.key != key(cfg, "pw-bands")? {
            return Ok(false);
        }
        if !store::identity_available(&receipt.input)? { return Ok(false); }
        for id in &receipt.outputs {
            if !store::identity_available(id)? { return Ok(false); }
        }
        receipt.verify()?;
    }
    Ok(true)
}

pub(super) fn archive(cfg: &Config, history: &Path) -> Result<()> {
    let path = cfg.work_dir.join("3.qe-electronic/receipts");
    if path.exists() {
        fs::create_dir_all(history)?;
        fs::rename(path, history.join("native-receipts"))?;
    }
    Ok(())
}

fn capture(
    cfg: &Config,
    task: &str,
    input: &Path,
    paths: &[PathBuf],
    save_root: Option<PathBuf>,
    provenance: Vec<Identity>,
) -> Result<NativeReceipt> {
    let mut outputs = paths
        .iter()
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
    ["out", "err", "exitcode"]
        .iter()
        .map(|ext| dir.join(format!("{stem}.{ext}")))
        .collect()
}

/// 重试只归档该原生任务日志及输出；不触碰成功 PW 的保存树。
fn archive_task(cfg: &Config, task: &str, paths: &[PathBuf]) -> Result<()> {
    let history = cfg
        .work_dir
        .join("epw-state/history")
        .join(format!("electronic-{task}-{}", store::token()));
    for path in paths
        .iter()
        .chain(std::iter::once(&receipt_path(cfg, task)))
    {
        if path.exists() {
            fs::create_dir_all(&history)?;
            fs::rename(
                path,
                history.join(path.file_name().context("data_invalid: 缺归档文件名")?),
            )?;
        }
    }
    Ok(())
}

fn validate_pw(cfg: &Config) -> Result<()> {
    let dir = Step::Electronic.directory(cfg).join("qe_eband");
    save::verify_snapshot(&cfg.work_dir)?;
    let snapshot = super::layout::path(&cfg.work_dir, "scf/snapshot");
    let density = PathBuf::from(format!("{}.save/charge-density.dat", cfg.qe.prefix));
    ensure!(
        store::hash_file(&snapshot.join(&density))?
            == store::hash_file(&dir.join("out").join(&density))?,
        "input_conflict: PW 能带电荷密度与已校验 SCF 快照不一致"
    );
    ensure!(
        fs::read_to_string(dir.join("eband.exitcode"))?.trim() == "0",
        "data_invalid: PW 能带缺零退出码"
    );
    qf_io::qe_completion::checked_log(&dir.join("eband.out"), false)?;
    let qe = QeSettings::from_config(cfg)?;
    let geo = crate::qe_data::xml::validate_save(&dir.join("out"), &cfg.qe.prefix, &qe, None)?;
    let states = crate::qe_data::nscf::parse_xml(
        &dir.join(format!("out/{}.save/data-file-schema.xml", cfg.qe.prefix)),
    )?;
    let points: Vec<[f64; 3]> = store::read(&dir.join("path-points.json"))?;
    ensure!(
        states.nbnd == cfg.qe.nscf.nbnd as usize && states.kpoints.len() == points.len(),
        "data_invalid: PW 能带带数或 k 点数不符"
    );
    for (actual, expected) in states.kpoints.iter().zip(points) {
        ensure!(
            geo.crystal(*actual)
                .iter()
                .zip(expected)
                .all(|(a, b)| (a - b).abs() < 1e-8),
            "data_invalid: PW 能带实际 k 点与路径不符"
        );
    }
    Ok(())
}

#[derive(Deserialize)]
struct LegacyAttempt {
    config: Identity,
    sources: Vec<Identity>,
    state: String,
    job: Option<u64>,
}

/// 只读核验旧 PW 结果，供现场核验及正常入口的一次性接入共用。
pub fn inspect_legacy(cfg: &Config) -> Result<NativeReceipt> {
    let request: String = store::read(&cfg.work_dir.join("epw-state/requests/electronic.json"))?;
    ensure!(
        request == stages::scientific_key(cfg, Step::Electronic)?,
        "input_conflict: 旧电子请求与当前科学参数不相容"
    );
    stages::upstream(cfg, Step::Electronic)?;
    let dir = Step::Electronic.directory(cfg).join("qe_eband");
    let input = fs::read_to_string(dir.join("eband.in"))?;
    let points: Vec<[f64; 3]> = store::read(&dir.join("path-points.json"))?;
    let mut provenance = None;
    for entry in fs::read_dir(cfg.work_dir.join("epw-state/attempts"))? {
        let file = entry?.path().join("attempt.json");
        let Ok(attempt) = store::read::<LegacyAttempt>(&file) else {
            continue;
        };
        if attempt.job.is_none() || !["failed", "succeeded"].contains(&attempt.state.as_str()) {
            continue;
        }
        attempt.config.verify()?;
        let old = Config::load(&attempt.config.path)?;
        if old.work_dir != cfg.work_dir
            || stages::scientific_key(&old, Step::Electronic)? != request
            || scientific_args(&old.programs.pw) != scientific_args(&cfg.programs.pw)
        {
            continue;
        }
        let expected = QeSettings::from_config(&old)?
            .path_input("./out", &points)?;
        if input != expected {
            continue;
        }
        let mut ids = vec![attempt.config];
        for path in std::iter::once(&old.structure)
            .chain(old.pseudos.values())
            .chain(old.kpath.iter())
            .chain(std::iter::once(&old.programs.pw.path))
        {
            let id = attempt
                .sources
                .iter()
                .find(|id| &id.path == path)
                .context("data_invalid: 旧尝试缺科学来源身份")?;
            id.verify()?;
            ids.push(id.clone());
        }
        // 点表本身必须与原冻结 KPATH 生成结果一致，不能信任手工修改的辅助 JSON。
        ensure!(
            points == path_points(&old)?,
            "input_conflict: 旧 PW 路径点表不符"
        );
        provenance = Some(ids);
        break;
    }
    let provenance = provenance.context(
        "data_invalid: 旧 PW 能带缺匹配的终态冻结来源；保留现场并停止，不能自动重算成功 PW",
    )?;
    validate_pw(cfg)?;
    let mut paths = logs(&dir, "eband");
    paths.push(dir.join("path-points.json"));
    capture(
        cfg,
        "pw-bands",
        &dir.join("eband.in"),
        &paths,
        Some(dir.join(format!("out/{}.save", cfg.qe.prefix))),
        provenance,
    )
}

fn prepare_pw(cfg: &Config) -> Result<()> {
    if let Some(receipt) = read_receipt(cfg, "pw-bands")? {
        ensure!(
            receipt.key == key(cfg, "pw-bands")?,
            "input_conflict: PW 能带原生选项或上游身份不相容"
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
        store::write(&receipt_path(cfg, "pw-bands"), &receipt)?;
        println!("reuse: electronic/pw-bands（旧冻结来源已校验）");
    }
    Ok(())
}
fn path_points(cfg: &Config) -> Result<Vec<[f64; 3]>> {
    let path = cfg.kpath.as_ref().context("required: 需要 KPATH.in")?;
    let segments =
        qf_input::electronic::parse_kpath_segments(path.to_str().context("KPATH 非 UTF-8")?)?;
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
    let input = qe
        .path_input("./out", &points)?;
    if !super::native_jobs::enabled() { prepare_pw(cfg)?; }
    if !receipt_path(cfg, "pw-bands").is_file() && !super::native_jobs::continuing(&bands, "eband")? {
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
    dos_inputs(cfg)?;
    Ok(())
}

fn dos_inputs(cfg: &Config) -> Result<()> {
    let dos = Step::Electronic.directory(cfg).join("qe_eledos");
    fs::create_dir_all(&dos)?;
    let shared: qf_config::Config = serde_json::from_value(
        json!({"qe_eledos":{"CONTROL":{"outdir":"./out"},"eledos":{"emin":-100,"emax":100,"DeltaE":0.01}}}),
    )?;
    qf_input::electronic::write_eledos_inputs(dos.to_str().unwrap(), &shared, &cfg.qe.prefix)?;
    let file = dos.join("eletdos.in");
    let text = fs::read_to_string(&file)?;
    let sigma = cfg
        .comparison
        .clone()
        .unwrap_or_default()
        .electronic_sigma_ev
        / 13.605693122994;
    fs::write(
        file,
        text.replacen("&dos", &format!("&dos\n ngauss=0, degauss={sigma:.15},"), 1),
    )?;
    Ok(())
}

pub(super) fn run_leaf(cfg: &Config, name: &str) -> Result<()> {
    let dir = Step::Electronic.directory(cfg).join(name);
    match name {
        "qe_eband" => {
            inputs(cfg, &QeSettings::from_config(cfg)?)?;
            if reusable(cfg, "pw-bands")? {
                println!("reuse: electronic/pw-bands");
            } else {
                let mut old = logs(&dir, "eband");
                old.push(dir.join("out"));
                if !super::native_jobs::continuing(&dir, "eband")? {
                    archive_task(cfg, "pw-bands", &old)?;
                    save::stage_scf(&cfg.work_dir, &dir.join("out"), &cfg.qe.prefix)?;
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
                store::write(&receipt_path(cfg, "pw-bands"), &receipt)?;
            }
            if reusable(cfg, "bands")? {
                println!("reuse: electronic/bands");
            } else {
                let mut paths = logs(&dir, "bands");
                paths.extend([
                    dir.join(format!("{}_band", cfg.qe.prefix)),
                    dir.join(format!("{}_band.gnu", cfg.qe.prefix)),
                ]);
                if !super::native_jobs::continuing(&dir, "bands")? { archive_task(cfg, "bands", &paths)?; }
                qe_run(
                    cfg,
                    &dir,
                    &post_program(cfg, "bands")?,
                    "elebanddata.in",
                    "bands",
                )?;
                for p in &paths {
                    if p.extension().is_none_or(|ext| ext != "err") {
                        save::nonempty(p)?;
                    }
                }
                // 后处理不得改写它读取的 portable PW 数据。
                read_receipt(cfg, "pw-bands")?
                    .context("data_invalid: 缺 PW 收据")?
                    .verify()?;
                let receipt = capture(
                    cfg,
                    "bands",
                    &dir.join("elebanddata.in"),
                    &paths,
                    None,
                    vec![],
                )?;
                store::write(&receipt_path(cfg, "bands"), &receipt)?;
            }
        }
        "qe_eledos" => {
            dos_inputs(cfg)?;
            if reusable(cfg, "dos")? {
                println!("reuse: electronic/dos");
                return Ok(());
            }
            let mut paths = logs(&dir, "dos");
            paths.push(dir.join(format!("{}.tdos", cfg.qe.prefix)));
            if !super::native_jobs::continuing(&dir, "dos")? { archive_task(cfg, "dos", &paths)?; }
            let source = super::layout::path(&cfg.work_dir, "nscf/out");
            let out = dir.join("out");
            if !out.exists() {
                save::copy_tree(&source, &out)?;
            }
            ensure!(
                save::files(&source)? == save::files(&out)?,
                "input_conflict: DOS 副本与 NSCF 来源不一致"
            );
            qe_run(cfg, &dir, &post_program(cfg, "dos")?, "eletdos.in", "dos")?;
            save::nonempty(&dir.join(format!("{}.tdos", cfg.qe.prefix)))?;
            let receipt = capture(
                cfg,
                "dos",
                &dir.join("eletdos.in"),
                &paths,
                Some(out.join(format!("{}.save", cfg.qe.prefix))),
                vec![],
            )?;
            store::write(&receipt_path(cfg, "dos"), &receipt)?;
        }
        _ => anyhow::bail!("configuration: 未知电子子阶段 {name}"),
    }
    Ok(())
}
