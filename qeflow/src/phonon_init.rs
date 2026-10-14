//! QE 正常初始化：主工作流与单步 --dyn0 共用，不生成自写 q 网格。
use super::*;
use anyhow::ensure;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

pub(super) fn prepare_input(ctx: &PhononCtx) -> Result<()> {
    let mut cfg = ctx.cfg.clone();
    for key in ["ldisp", "only_init", "lqdir"] {
        cfg.qe_phonon.ph.insert(key.into(), json!(true));
    }
    for key in ["recover", "only_wfc"] {
        cfg.qe_phonon.ph.insert(key.into(), json!(false));
    }
    for key in [
        "start_q",
        "last_q",
        "start_irr",
        "last_irr",
        "prefix",
        "outdir",
    ] {
        cfg.qe_phonon.ph.remove(key);
    }
    ph_input::write_ph_input(&ctx.workspace, &cfg, &ctx.prefix, "ph_init.in")?;
    Ok(())
}

pub(super) fn checked_log(path: &Path, scf: bool) -> Result<()> {
    let text = fs::read_to_string(path).with_context(|| format!("缺少输出 {}", path.display()))?;
    let lower = text.to_lowercase();
    ensure!(
        text.contains("JOB DONE")
            && !["error in routine", "convergence not achieved", "%%%%%%%%"]
                .iter()
                .any(|s| lower.contains(s)),
        "计算未正常完成: {}",
        path.display()
    );
    if scf {
        ensure!(
            lower.contains("convergence has been achieved"),
            "SCF 未收敛: {}",
            path.display()
        );
    }
    Ok(())
}

fn has_wfc(dir: &Path) -> Result<bool> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        // 不跟随目录链接，避免环；普通波函数文件链接可由 QE 正常读取。
        if entry.file_type()?.is_dir() {
            if has_wfc(&entry.path())? {
                return Ok(true);
            }
        } else if entry.file_name().to_string_lossy().contains("wfc")
            && fs::metadata(entry.path())?.len() > 0
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn check_save(ctx: &PhononCtx) -> Result<()> {
    let od = ctx
        .cfg
        .qe_phonon
        .scf
        .get("outdir")
        .and_then(Value::as_str)
        .unwrap_or("./out");
    let root = Path::new(&ctx.workspace).join(od);
    let save = root.join(format!("{}.save", ctx.prefix));
    for name in ["data-file-schema.xml", "charge-density.dat"] {
        let path = save.join(name);
        ensure!(
            path.is_file() && path.metadata()?.len() > 0,
            "SCF 保存文件缺失或为空: {}；请先完成 SCF 或修正 outdir",
            path.display()
        );
    }
    ensure!(has_wfc(&root)?, "SCF 波函数缺失: {}", root.display());
    Ok(())
}

pub(super) fn check_output(ws: &Path, prefix: &str) -> Result<usize> {
    checked_log(&ws.join("ph_init.out"), false)?;
    let raw = fs::read_to_string(ws.join(format!("{prefix}.dyn0")))?;
    let mut lines = raw.lines().filter(|l| !l.trim().is_empty());
    let mesh: Vec<i64> = lines
        .next()
        .context("dyn0 缺少网格")?
        .split_whitespace()
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()?;
    ensure!(
        mesh.len() == 3 && mesh.iter().all(|n| *n > 0),
        "dyn0 网格无效"
    );
    let count: usize = lines.next().context("dyn0 缺少 q 点数量")?.trim().parse()?;
    ensure!(
        count > 0 && lines.count() == count,
        "dyn0 q 点数量与数据行不一致"
    );
    let points = qf_io::parse_dyn0_qpoints(&raw)?;
    ensure!(
        points.len() == count
            && points
                .iter()
                .all(|p| p.q_crystal.iter().all(|x| x.is_finite())),
        "dyn0 q 点缺失或存在非有限坐标"
    );
    Ok(count)
}

pub(super) fn run_init(ctx: &PhononCtx) -> Result<()> {
    let ws = Path::new(&ctx.workspace);
    let marker = ws.join("ph_init_complete.json");
    if marker.exists() {
        fs::remove_file(&marker)?;
    }
    check_save(ctx)?;
    prepare_input(ctx)?;
    // 归档上次列表，防止本次 QE 未写文件而误验收旧结果。
    let dyn0 = ws.join(format!("{}.dyn0", ctx.prefix));
    if dyn0.exists() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        fs::rename(
            &dyn0,
            ws.join(format!("{}.dyn0.before-init-{stamp}", ctx.prefix)),
        )?;
    }
    qf_runner::run_shell(
        &format!(
            "{} < ph_init.in > ph_init.out 2>&1",
            qf_runner::resolve_qe_command(ctx.cfg.qe.executable_path(), "ph.x")
        ),
        ws,
    )?;
    let count = check_output(ws, &ctx.prefix)?;
    let tmp = ws.join("ph_init_complete.json.partial");
    fs::write(
        &tmp,
        serde_json::to_vec_pretty(
            &json!({"prefix":ctx.prefix,"q_count":count,"stage":"initialization"}),
        )?,
    )?;
    fs::rename(tmp, marker)?;
    println!("QE 初始化完成：{count} 个不可约 q 点；未执行完整声子计算");
    Ok(())
}

fn context(config: &str, input: Option<&str>) -> Result<PhononCtx> {
    let mut cfg = qf_config::load(Some(config), Some(config))?;
    let base = Path::new(config).parent().unwrap();
    let ws = base.join(cfg.qe_phonon.work_dir.as_deref().unwrap_or("qe_phonon"));
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
        fs::copy(&source, &target).with_context(|| format!("复制结构 {}", source.display()))?;
    }
    let structure = qf_crystal::parse_poscar(target.to_str().context("结构路径不是 UTF-8")?)?;
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
    ensure!(
        !prefix.is_empty() && !prefix.contains(['/', '\\']) && prefix != "." && prefix != "..",
        "prefix 必须是非空文件名前缀"
    );
    cfg.qe_phonon.work_dir = Some(ws.to_string_lossy().into_owned());
    Ok(PhononCtx {
        cfg,
        workspace: ws.to_string_lossy().into_owned(),
        prefix,
        structure,
    })
}

pub(super) fn execute(config: String, input: Option<String>) -> Result<()> {
    let config = fs::canonicalize(config)?;
    run_init(&context(
        config.to_str().context("配置路径不是 UTF-8")?,
        input.as_deref(),
    )?)
}

pub(super) fn entry(
    config: Option<String>,
    input: Option<String>,
    run: bool,
    submit: bool,
) -> Result<()> {
    ensure!(!(run && submit), "-r 与 -s 不能同时使用");
    let config = fs::canonicalize(config.as_deref().unwrap_or("input.toml"))?;
    let config = config.to_str().context("配置路径不是 UTF-8")?;
    let ctx = context(config, input.as_deref())?;
    prepare_input(&ctx)?;
    let quote = phonon_chain::quote;
    let script = format!(
        "#!/bin/bash\n{}\nset -e\ncd {}\n{} qe-ph-init-run -c {} -i {}\n",
        ctx.cfg.slurm.header.as_deref().unwrap_or(""),
        quote(&ctx.workspace),
        quote(
            std::env::current_exe()?
                .to_str()
                .context("程序路径不是 UTF-8")?
        ),
        quote(config),
        quote(&format!("{}/POSCAR", ctx.workspace))
    );
    fs::write(Path::new(&ctx.workspace).join("run_ph_init.sh"), script)?;
    if run {
        run_init(&ctx)?;
    } else if submit {
        check_save(&ctx)?;
        let job = qf_runner::submit_via_sbatch(Path::new(&ctx.workspace), "run_ph_init.sh")?;
        println!(
            "初始化作业已提交: {}；提交成功不代表初始化完成",
            job.context("未返回作业号")?
        );
    } else {
        println!(
            "已准备 {}/ph_init.in 和 run_ph_init.sh；使用 --dyn0 -s 提交",
            ctx.workspace
        );
    }
    Ok(())
}
