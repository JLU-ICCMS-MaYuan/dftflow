//! Issue 0006：一份 SCF 快照、两种 PH 拆分、Slurm 自动依赖链。
use super::*;
use super::phonon_init::checked_log;
use anyhow::{bail, ensure};
use qf_phonon::index_map::{QIndexEntry, QIndexMap};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

pub(super) fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.0);
    }
}
fn lock(ws: &str) -> Result<Lock> {
    let p = Path::new(ws).join(".chain-submit.lock");
    fs::create_dir(&p)
        .with_context(|| format!("已有提交正在执行，或需核对残留锁: {}", p.display()))?;
    Ok(Lock(p))
}
fn atomic_json(path: &Path, value: &Value) -> Result<()> {
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}
fn jobs(ws: &str) -> Result<BTreeMap<String, String>> {
    let p = Path::new(ws).join("chain_jobs.json");
    if !p.exists() {
        return Ok(BTreeMap::new());
    }
    Ok(serde_json::from_slice(&fs::read(p)?)?)
}
fn save_jobs(ws: &str, records: &BTreeMap<String, String>) -> Result<()> {
    atomic_json(
        &Path::new(ws).join("chain_jobs.json"),
        &serde_json::to_value(records)?,
    )
}
fn active(id: &str) -> Result<bool> {
    let state = qf_runner::slurm_job_state(id)?;
    if state == "INVALID_DEPENDENCY" {
        let out = std::process::Command::new("scancel").arg(id).output()?;
        ensure!(out.status.success(), "取消失效依赖作业失败: {id}");
        return Ok(false);
    }
    Ok(matches!(
        state.as_str(),
        "PENDING"
            | "RUNNING"
            | "CONFIGURING"
            | "COMPLETING"
            | "SUSPENDED"
            | "RESIZING"
            | "REQUEUED"
    ))
}
fn outdir(ctx: &PhononCtx) -> Result<PathBuf> {
    let s = ctx
        .cfg
        .qe_phonon
        .scf
        .get("outdir")
        .and_then(Value::as_str)
        .unwrap_or("./tmp");
    let p = PathBuf::from(s);
    ensure!(
        !p.is_absolute()
            && !p
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            && p != Path::new("."),
        "分 q 模式 outdir 必须是独立相对目录: {s}"
    );
    Ok(p)
}
fn context(config: &str, input: Option<&str>, mode: &str) -> Result<PhononCtx> {
    ensure!(mode == "index" || mode == "coords", "未知模式 {mode}");
    let mut cfg = qf_config::load(Some(config), Some(config))?;
    let base = Path::new(config).parent().unwrap();
    if let Some(pp) = cfg.pp_dir.as_mut() {
        *pp = base.join(&*pp).to_string_lossy().into_owned();
    }
    let ws = base.join(
        cfg.qe_phonon
            .work_dir
            .as_deref()
            .unwrap_or(&format!("qe_phonon_split_{mode}")),
    );
    fs::create_dir_all(&ws)?;
    let ws = fs::canonicalize(ws)?;
    let target = ws.join("POSCAR");
    let source = input.map(PathBuf::from).unwrap_or_else(|| {
        if target.exists() {
            target.clone()
        } else {
            base.join("POSCAR")
        }
    });
    if fs::canonicalize(&source).ok() != fs::canonicalize(&target).ok() {
        // 多 q 检查可能并行，原子替换避免另一进程读到截断中的 POSCAR。
        let temporary = target.with_extension(format!("{}.tmp", std::process::id()));
        fs::copy(&source, &temporary)?;
        fs::rename(&temporary, &target)?;
    }
    let structure = qf_crystal::parse_poscar(target.to_str().unwrap())?;
    let prefix = cfg
        .qe_phonon
        .prefix
        .clone()
        .or_else(|| {
            cfg.qe_phonon
                .scf
                .get("prefix")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| structure.formula.clone());
    cfg.qe_phonon.work_dir = Some(ws.to_string_lossy().into_owned());
    cfg.qe_phonon.scf.insert("prefix".into(), json!(prefix));
    let ctx = PhononCtx {
        cfg,
        workspace: ws.to_string_lossy().into_owned(),
        prefix,
        structure,
        structure_source: source.to_string_lossy().into_owned(),
    };
    outdir(&ctx)?;
    Ok(ctx)
}
fn bind_inputs(ctx: &PhononCtx, config: &str, mode: &str) -> Result<()> {
    let path = Path::new(&ctx.workspace).join("chain_inputs.json");
    let expected = json!({"config": fs::read_to_string(config)?, "structure": fs::read_to_string(Path::new(&ctx.workspace).join("POSCAR"))?, "mode": mode});
    if path.exists() {
        ensure!(
            serde_json::from_slice::<Value>(&fs::read(&path)?)? == expected,
            "配置/结构/模式已经改变，请使用新的运行目录"
        );
    } else {
        atomic_json(&path, &expected)?;
    }
    Ok(())
}
/// 返回工作目录内的固定配置与原名结构快照；已存在时只核对，不覆盖。
pub(super) fn snapshot_inputs(ctx: &PhononCtx, config: &str, input: Option<&str>) -> Result<(String, String)> {
    let ws = Path::new(&ctx.workspace);
    let cfg_path = ws.join("input-init.toml");
    if Path::new(config) == cfg_path {
        return Ok((cfg_path.to_string_lossy().into_owned(), verify_snapshot(ws)?.to_string_lossy().into_owned()));
    }
    let source = input.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(&ctx.structure_source));
    let name = source.file_name().context("结构文件名无效")?.to_string_lossy();
    let mut structure_path = ws.join(format!("{name}-init"));
    if input.is_none() && ws.join("input-init.json").exists() {
        structure_path = verify_snapshot(ws)?;
    }
    let text = qf_config::phonon_snapshot_text(config, &ctx.workspace)?;
    let structure = fs::read_to_string(&source)?;
    let manifest = ws.join("input-init.json");
    let expected = json!({"config":text,"structure":structure,"structure_file":structure_path.file_name().unwrap().to_string_lossy()});
    if manifest.exists() {
        verify_snapshot(ws)?;
        ensure!(serde_json::from_slice::<Value>(&fs::read(&manifest)?)? == expected, "已有输入快照与本次输入不同，请使用原快照续算或新的工作目录");
    } else {
        ensure!(!cfg_path.exists() && !structure_path.exists(), "已有未登记输入快照，请先核对，禁止覆盖");
        fs::write(&cfg_path, &text)?;
        fs::write(&structure_path, structure.as_bytes())?;
        atomic_json(&manifest, &expected)?;
    }
    Ok((cfg_path.to_string_lossy().into_owned(), structure_path.to_string_lossy().into_owned()))
}
fn verify_snapshot(ws: &Path) -> Result<PathBuf> {
    let manifest: Value = serde_json::from_slice(&fs::read(ws.join("input-init.json"))?)?;
    let name = manifest["structure_file"].as_str().context("结构快照清单无效")?;
    ensure!(Path::new(name).components().count() == 1, "结构快照路径无效");
    let structure = ws.join(name);
    ensure!(manifest["config"].as_str() == Some(fs::read_to_string(ws.join("input-init.toml"))?.as_str()) && manifest["structure"].as_str() == Some(fs::read_to_string(&structure)?.as_str()), "输入快照已改变，禁止继续计算");
    Ok(structure)
}
fn verify_mode(ws: &str, mode: &str) -> Result<()> {
    let binding: Value = serde_json::from_slice(&fs::read(Path::new(ws).join("chain_inputs.json"))?)?;
    ensure!(binding["mode"].as_str() == Some(mode), "快照绑定的拆分模式与本次模式不同");
    Ok(())
}
pub(super) fn snapshot_structure(ws: &str) -> Result<String> {
    Ok(verify_snapshot(Path::new(ws))?.to_string_lossy().into_owned())
}

fn stage_command(config: &str, ws: &str, mode: &str, stage: &str) -> Result<String> {
    Ok(format!(
        "{} qe-phonon-chain-stage -c {} -i {} --mode {} --stage {}",
        quote(
            std::env::current_exe()?
                .to_str()
                .context("程序路径不是 UTF-8")?
        ),
        quote(config),
        quote(&snapshot_structure(ws)?),
        mode,
        stage
    ))
}
fn stage_script(ctx: &PhononCtx, file: &str, command: &str) -> Result<()> {
    let header = ctx.cfg.slurm.header.as_deref().unwrap_or("");
    fs::write(
        Path::new(&ctx.workspace).join(file),
        format!(
            "#!/bin/bash\n{header}\nset -e\ncd {}\n{command}\n",
            quote(&ctx.workspace)
        ),
    )?;
    Ok(())
}
fn prepare(ctx: &PhononCtx, config: &str, mode: &str) -> Result<()> {
    prepare_root_scf(ctx)?;
    phonon_init::prepare_input(ctx)?;
    stage_script(
        ctx,
        "submit_pre_and_split_stage.sh",
        &stage_command(config, &ctx.workspace, mode, "pre")?,
    )?;
    stage_script(
        ctx,
        "submit_merge_and_process_phonon.sh",
        &stage_command(config, &ctx.workspace, mode, "post")?,
    )?;
    Ok(())
}
fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        ensure!(
            !e.file_type()?.is_symlink(),
            "快照不允许共享符号链接: {}",
            e.path().display()
        );
        if e.file_type()?.is_dir() {
            copy_tree(&e.path(), &dst.join(e.file_name()))?;
        } else {
            fs::copy(e.path(), dst.join(e.file_name()))?;
        }
    }
    Ok(())
}
fn pre(ctx: &PhononCtx) -> Result<()> {
    let ws = Path::new(&ctx.workspace);
    let snapshot = ws.join("scf_snapshot");
    if !snapshot.exists() {
        if checked_log(&ws.join("scf.out"), true).is_err() {
            let cmd = format!(
                "{} < scf.in > scf.out 2>&1",
                qf_runner::resolve_qe_command(ctx.cfg.qe.executable_path(), "pw.x")
            );
            qf_runner::run_shell(&cmd, ws)?;
        }
        checked_log(&ws.join("scf.out"), true)?;
        let src = ws.join(outdir(ctx)?);
        phonon_init::check_save(ctx)?;
        ensure!(
            !src.join("_ph0").exists(),
            "首次快照前已有 PH scratch，请使用新目录"
        );
        let tmp = ws.join("scf_snapshot.partial");
        if tmp.exists() {
            fs::remove_dir_all(&tmp)?;
        }
        copy_tree(&src, &tmp)?;
        fs::rename(&tmp, &snapshot)?;
    }
    if phonon_init::check_output(ws, &ctx.prefix).is_err() {
        phonon_init::run_init(ctx)?;
    }
    ensure!(!load_points(ctx)?.is_empty(), "初始化未产生 q 点");
    Ok(())
}
fn crystal_from_cart(bg: [[f64; 3]; 3], cart: [f64; 3]) -> Result<[f64; 3]> {
    // bg 的行是倒格矢；解 bg^T * crystal = cart。
    let mut a = [[0.0; 4]; 3];
    for i in 0..3 {
        for j in 0..3 {
            a[i][j] = bg[j][i];
        }
        a[i][3] = cart[i];
    }
    for k in 0..3 {
        let pivot = (k..3)
            .max_by(|&i, &j| a[i][k].abs().partial_cmp(&a[j][k].abs()).unwrap())
            .unwrap();
        a.swap(k, pivot);
        ensure!(a[k][k].abs() > 1e-12, "退化倒格矩阵");
        let d = a[k][k];
        for j in k..4 {
            a[k][j] /= d;
        }
        for i in 0..3 {
            if i != k {
                let f = a[i][k];
                for j in k..4 {
                    a[i][j] -= f * a[k][j];
                }
            }
        }
    }
    Ok([a[0][3], a[1][3], a[2][3]])
}
fn load_points(ctx: &PhononCtx) -> Result<Vec<QIndexEntry>> {
    let raw = fs::read_to_string(Path::new(&ctx.workspace).join(format!("{}.dyn0", ctx.prefix)))?;
    let points = qf_io::parse_dyn0_qpoints(&raw)?;
    let info = qf_phonon::xml::parse_qe_save_xml(
        &Path::new(&ctx.workspace)
            .join("scf_snapshot")
            .join(format!("{}.save/data-file-schema.xml", ctx.prefix)),
    )?;
    points
        .into_iter()
        .map(|p| {
            // 旧 parser 字段名是 q_crystal，dyn0 中实际为 QE Cartesian。
            let cart = p.q_crystal;
            ensure!(cart.iter().all(|v| v.is_finite()), "非有限 q 坐标");
            let crystal = crystal_from_cart(info.bg, cart)?;
            Ok(QIndexEntry {
                directory: format!("{:02}", p.index),
                index: p.index,
                coords: Some(crystal),
                q_cart: Some(cart),
                q_crystal: Some(crystal),
            })
        })
        .collect()
}
fn materialize(ctx: &PhononCtx, mode: &str) -> Result<QIndexMap> {
    let map = QIndexMap {
        mode: format!("split_assign_{mode}"),
        qpoints: load_points(ctx)?,
    };
    let path = Path::new(&ctx.workspace).join("q_index_map.json");
    if path.exists() {
        let old = qf_phonon::index_map::load_q_index_map(&ctx.workspace);
        ensure!(
            serde_json::to_value(&old)? == serde_json::to_value(&map)?,
            "已有 q 映射与初始化不一致"
        );
    } else {
        atomic_json(&path, &serde_json::to_value(&map)?)?;
    }
    for e in &map.qpoints {
        let q = Path::new(&ctx.workspace).join(&e.directory);
        fs::create_dir_all(&q)?;
        if !q.join("scf_reused.json").exists() {
            let target = q.join(outdir(ctx)?);
            ensure!(
                !target.exists(),
                "q 点目录已有数据且无复用标记，请归档后重试"
            );
            let partial = q.join("scf_copy.partial");
            if partial.exists() {
                fs::remove_dir_all(&partial)?;
            }
            copy_tree(&Path::new(&ctx.workspace).join("scf_snapshot"), &partial)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(&partial, &target)?;
            atomic_json(
                &q.join("scf_reused.json"),
                &json!({"source":"../scf_snapshot","prefix":ctx.prefix}),
            )?;
        }
        let mut cfg = ctx.cfg.clone();
        cfg.qe_phonon.ph.insert("only_init".into(), json!(false));
        cfg.qe_phonon.ph.insert("only_wfc".into(), json!(false));
        cfg.qe_phonon.ph.insert("recover".into(), json!(false));
        if mode == "index" {
            ph_input::write_split_ph_input_assign_index(
                q.to_str().unwrap(),
                &cfg,
                &ctx.prefix,
                e.index as i64,
            )?;
        } else {
            ph_input::write_split_ph_input_assign_coords(
                q.to_str().unwrap(),
                &cfg,
                &ctx.prefix,
                e.q_cart.unwrap(),
            )?;
        }
        let command=format!("cd {}\n{} < split_ph.in > split_ph.out 2>&1\n{} qe-phonon-chain-stage -c {} -i {} --mode {} --stage check-q --q-dir {}",quote(q.to_str().unwrap()),qf_runner::resolve_qe_command(cfg.qe.executable_path(),"ph.x"),quote(std::env::current_exe()?.to_str().unwrap()),quote(&absolute_config_from_binding(&ctx.workspace)?),quote(&snapshot_structure(&ctx.workspace)?),mode,quote(&e.directory));
        stage_script(ctx, &format!("{}/run_split.sh", e.directory), &command)?;
    }
    Ok(map)
}
fn absolute_config_from_binding(ws: &str) -> Result<String> {
    Ok(fs::read_to_string(Path::new(ws).join("chain_config_path"))?
        .trim()
        .to_string())
}
fn q_done(ctx: &PhononCtx, e: &QIndexEntry) -> bool {
    let q = Path::new(&ctx.workspace).join(&e.directory);
    checked_log(&q.join("split_ph.out"), false).is_ok()
        && [
            q.join(format!("{}.dyn{}", ctx.prefix, e.index)),
            q.join(format!("{}.dyn", ctx.prefix)),
        ]
        .iter()
        .any(|p| p.metadata().map(|m| m.len() > 0).unwrap_or(false))
}
fn fanout(ctx: &PhononCtx, config: &str, mode: &str, local: bool) -> Result<()> {
    let _lock = lock(&ctx.workspace)?;
    let map = materialize(ctx, mode)?;
    let mut records = jobs(&ctx.workspace)?;
    let parent = if local {
        None
    } else {
        std::env::var("SLURM_JOB_ID").ok()
    };
    let mut deps: Vec<String> = parent.iter().cloned().collect();
    for e in &map.qpoints {
        if q_done(ctx, e) {
            continue;
        }
        let key = format!("q:{}", e.directory);
        if local {
            qf_runner::run_shell(
                "bash run_split.sh",
                &Path::new(&ctx.workspace).join(&e.directory),
            )?;
            ensure!(q_done(ctx, e), "q 点 {} 未完成", e.directory);
        } else {
            if let Some(id) = records.get(&key) {
                if active(id)? {
                    deps.push(id.clone());
                    continue;
                }
            }
            let id = qf_runner::submit_slurm(
                &Path::new(&ctx.workspace).join(&e.directory),
                "run_split.sh",
                &parent.iter().cloned().collect::<Vec<_>>(),
            )?;
            records.insert(key, id.clone());
            save_jobs(&ctx.workspace, &records)?;
            deps.push(id);
        }
    }
    if local {
        drop(_lock);
        return post(ctx, config);
    }
    if let Some(id) = records.get("post") {
        if active(id)? {
            println!("post 已提交: {id}");
            return Ok(());
        }
    }
    let id = qf_runner::submit_slurm(
        Path::new(&ctx.workspace),
        "submit_merge_and_process_phonon.sh",
        &deps,
    )?;
    records.insert("post".into(), id.clone());
    save_jobs(&ctx.workspace, &records)?;
    println!("已安排 q 点及合并绘图作业 {id}，依赖 {}", deps.join(":"));
    Ok(())
}
fn post(ctx: &PhononCtx, config: &str) -> Result<()> {
    let map = qf_phonon::index_map::load_q_index_map(&ctx.workspace);
    ensure!(
        !map.qpoints.is_empty() && map.qpoints.iter().all(|e| q_done(ctx, e)),
        "仍有 q 点未完成，禁止后处理"
    );
    let merged = qf_phonon::merge::merge_split_results_with_outdir(
        &ctx.workspace,
        &ctx.prefix,
        &map,
        &outdir(ctx)?.to_string_lossy(),
    )?;
    ensure!(merged.success, "合并失败: {:?}", merged.errors);
    q2r_input::write_q2r_input(&ctx.workspace, &ctx.cfg, &ctx.prefix)?;
    matdyn_input::write_matdyn_phband_input(
        &ctx.workspace,
        &ctx.cfg,
        &ctx.prefix,
        ctx.structure.lattice,
    )?;
    matdyn_input::write_matdyn_phdos_input(&ctx.workspace, &ctx.cfg, &ctx.prefix)?;
    for (bin, name) in [
        ("q2r.x", "q2r"),
        ("matdyn.x", "matdyn_phband"),
        ("matdyn.x", "matdyn_phdos"),
    ] {
        let command = format!(
            "{} < {name}.in > {name}.out 2>&1",
            qf_runner::resolve_qe_command(ctx.cfg.qe.executable_path(), bin)
        );
        qf_runner::run_shell(&command, Path::new(&ctx.workspace))?;
        checked_log(
            &Path::new(&ctx.workspace).join(format!("{name}.out")),
            false,
        )?;
    }
    std::env::set_current_dir(&ctx.workspace)?;
    run_qe_process_phonon(
        Some(config.into()),
        Some(format!("{}/POSCAR", ctx.workspace)),
        false,
        false,
        true,
        false,
    )?;
    run_qe_process_phonon(
        Some(config.into()),
        Some(format!("{}/POSCAR", ctx.workspace)),
        false,
        false,
        false,
        true,
    )?;
    for f in qf_phonon::archive::phonon_plot_files() {
        ensure!(
            Path::new(&ctx.workspace)
                .join(&f)
                .metadata()
                .map(|m| m.len() > 0)
                .unwrap_or(false),
            "缺少产物 {f}"
        );
    }
    ensure!(
        Path::new(&ctx.workspace)
            .join("phonon_combined.jpg")
            .metadata()
            .map(|m| m.len() > 0)
            .unwrap_or(false),
        "缺少联合 JPG"
    );
    atomic_json(
        &Path::new(&ctx.workspace).join("chain_complete.json"),
        &json!({"finished":true,"prefix":ctx.prefix}),
    )?;
    refresh_workflow_status(Path::new(&ctx.workspace), &ctx.prefix)?;
    println!("声子流程已完成：{}", ctx.workspace);
    Ok(())
}
pub fn run(
    config: Option<String>,
    input: Option<String>,
    run: bool,
    submit: bool,
    mode: &str,
) -> Result<()> {
    ensure!(!(run && submit), "运行和提交互斥");
    let config = fs::canonicalize(config.as_deref().unwrap_or("input.toml"))?
        .to_string_lossy()
        .into_owned();
    let input = if Path::new(&config).file_name().and_then(|p| p.to_str()) == Some("input-init.toml") {
        Some(snapshot_structure(Path::new(&config).parent().unwrap().to_str().unwrap())?)
    } else { input };
    let ctx = context(&config, input.as_deref(), mode)?;
    ensure!(!ctx.cfg.qe_phonon.retain_epw_data,
        "retain_epw_data 的 EPW 交付目前仅支持 qe-phonon-nosplit；分 q 不可冒充已适配");
    if Path::new(&config) != Path::new(&ctx.workspace).join("input-init.toml") {
        bind_inputs(&ctx, &config, mode)?;
    } else { verify_snapshot(Path::new(&ctx.workspace))?; verify_mode(&ctx.workspace, mode)?; }
    let (config, _) = snapshot_inputs(&ctx, &config, input.as_deref())?;
    fs::write(Path::new(&ctx.workspace).join("chain_config_path"), &config)?;
    prepare(&ctx, &config, mode)?;
    if !run && !submit {
        println!("已准备声子流程输入与脚本：{}", ctx.workspace);
        return Ok(());
    }
    if Path::new(&ctx.workspace)
        .join("chain_complete.json")
        .exists()
    {
        let complete = qf_phonon::archive::phonon_plot_files()
            .into_iter()
            .chain(std::iter::once("phonon_combined.jpg".into()))
            .all(|f| {
                Path::new(&ctx.workspace)
                    .join(f)
                    .metadata()
                    .map(|m| m.len() > 0)
                    .unwrap_or(false)
            });
        if complete {
            println!("声子流程已完成：{}", ctx.workspace);
            return Ok(());
        }
        fs::remove_file(Path::new(&ctx.workspace).join("chain_complete.json"))?;
    }
    if run {
        let pre_lock = lock(&ctx.workspace)?;
        pre(&ctx)?;
        drop(pre_lock);
        fanout(&ctx, &config, mode, true)?;
    } else {
        let _lock = lock(&ctx.workspace)?;
        let mut records = jobs(&ctx.workspace)?;
        // 单个 q 点失败后的续算：只复用已经完成的 pre。
        if let Some(id) = records.get("pre") {
            if active(id)? {
                println!("pre 已提交: {id}");
                return Ok(());
            }
        }
        for (stage, id) in &records {
            if stage == "post" || stage.starts_with("q:") {
                if active(id)? {
                    println!("已有活动作业 {stage}={id}，请等待或取消后续算");
                    return Ok(());
                }
            }
        }
        let id = qf_runner::submit_slurm(
            Path::new(&ctx.workspace),
            "submit_pre_and_split_stage.sh",
            &[],
        )?;
        records.insert("pre".into(), id.clone());
        save_jobs(&ctx.workspace, &records)?;
        println!("已提交前置作业 {id}；后续计算及绘图将自动安排");
    }
    Ok(())
}
pub fn stage(
    config: String,
    input: Option<String>,
    mode: String,
    stage: String,
    q_dir: Option<String>,
) -> Result<()> {
    // 新脚本直接传快照；兼容已生成脚本时，结构所在目录若有登记快照也转向它。
    let hinted_ws = input.as_deref().and_then(|p| Path::new(p).parent());
    let snapshot_ws = if Path::new(&config).file_name().and_then(|s| s.to_str()) == Some("input-init.toml") {
        Path::new(&config).parent()
    } else { hinted_ws.filter(|p| p.join("input-init.json").exists()) };
    let (config, input, frozen) = if let Some(ws) = snapshot_ws {
        let structure = verify_snapshot(ws)?;
        (ws.join("input-init.toml").to_string_lossy().into_owned(), Some(structure.to_string_lossy().into_owned()), true)
    } else { (config, input, false) };
    let config = fs::canonicalize(config)?.to_string_lossy().into_owned();
    let ctx = context(&config, input.as_deref(), &mode)?;
    if !frozen { bind_inputs(&ctx, &config, &mode)?; } else { verify_mode(&ctx.workspace, &mode)?; }
    match stage.as_str() {
        "pre" => {
            pre(&ctx)?;
            fanout(&ctx, &config, &mode, false)
        }
        "fanout" => fanout(&ctx, &config, &mode, false),
        "post" => post(&ctx, &config),
        "check-q" => {
            let map = qf_phonon::index_map::load_q_index_map(&ctx.workspace);
            let e = map
                .qpoints
                .iter()
                .find(|e| Some(&e.directory) == q_dir.as_ref())
                .context("未知 q 目录")?;
            ensure!(q_done(&ctx, e), "q 点 {} 未完成", e.directory);
            Ok(())
        }
        _ => bail!("未知阶段 {stage}"),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn nonorthogonal_cart_conversion() {
        let c = super::crystal_from_cart(
            [[1., 0.577350269, 0.], [0., 1.154700538, 0.], [0., 0., 0.87]],
            [0.5, 0.2886751345, 0.435],
        )
        .unwrap();
        for (a, b) in c.into_iter().zip([0.5, 0., 0.5]) {
            assert!((a - b).abs() < 1e-9);
        }
    }
}
