//! QE 正常初始化：主工作流与单步 --dyn0 共用，不生成自写 q 网格。
use super::*;
use anyhow::ensure;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

pub(super) fn prepare_input(ctx: &PhononCtx) -> Result<()> {
    ensure!(!ctx.cfg.qe_phonon.retain_epw_data,
        "retain_epw_data It's only perfect for now qe-phonon-nosplit ,Could not be used for individual initialization or split q");
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

pub(super) use qf_io::qe_completion::checked_log;

pub(super) fn check_save(ctx: &PhononCtx) -> Result<()> {
    let od = ctx
        .cfg
        .qe_phonon
        .scf
        .get("outdir")
        .and_then(Value::as_str)
        .unwrap_or("./tmp");
    let root = Path::new(&ctx.workspace).join(od);
    let save = root.join(format!("{}.save", ctx.prefix));
    for name in ["data-file-schema.xml"] {
        let path = save.join(name);
        ensure!(
            path.is_file() && path.metadata()?.len() > 0,
            "SCF Save file missing or empty: {};Please finish first SCF or amendment outdir",
            path.display()
        );
    }
    Ok(())
}

pub(super) fn check_output(ws: &Path, prefix: &str) -> Result<usize> {
    checked_log(&ws.join("ph_init.out"), false)?;
    let raw = fs::read_to_string(ws.join(format!("{prefix}.dyn0")))?;
    let mut lines = raw.lines().filter(|l| !l.trim().is_empty());
    let mesh: Vec<i64> = lines
        .next()
        .context("dyn0 Missing Grid")?
        .split_whitespace()
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()?;
    ensure!(
        mesh.len() == 3 && mesh.iter().all(|n| *n > 0),
        "dyn0 Grid invalid"
    );
    let count: usize = lines.next().context("dyn0 Missing q Number of points")?.trim().parse()?;
    ensure!(
        count > 0 && lines.count() == count,
        "dyn0 q The number of points is not consistent with the data line"
    );
    let points = qf_io::parse_dyn0_qpoints(&raw)?;
    ensure!(
        points.len() == count
            && points
                .iter()
                .all(|p| p.q_crystal.iter().all(|x| x.is_finite())),
        "dyn0 q Point missing or non-finite coordinates"
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
    println!("QE Initialization complete:{count} It's not a date q dots; not performing full sound count");
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
        fs::copy(&source, &target).with_context(|| format!("Copy Structure {}", source.display()))?;
    }
    let structure = qf_crystal::parse_poscar(target.to_str().context("The structural path is not UTF-8")?)?;
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
        "prefix Must be a non-empty file prefix"
    );
    cfg.qe_phonon.work_dir = Some(ws.to_string_lossy().into_owned());
    Ok(PhononCtx {
        cfg,
        workspace: ws.to_string_lossy().into_owned(),
        prefix,
        structure,
        structure_source: source.to_string_lossy().into_owned(),
    })
}

pub(super) fn execute(config: String, input: Option<String>) -> Result<()> {
    let config = fs::canonicalize(config)?;
    run_init(&context(
        config.to_str().context("Configure Paths Not UTF-8")?,
        input.as_deref(),
    )?)
}

pub(super) fn entry(
    config: Option<String>,
    input: Option<String>,
    run: bool,
    submit: bool,
) -> Result<()> {
    ensure!(!(run && submit), "-r with -s It can't be used simultaneously");
    let config = fs::canonicalize(config.as_deref().unwrap_or("input.toml"))?;
    let config = config.to_str().context("Configure Paths Not UTF-8")?;
    let ctx = context(config, input.as_deref())?;
    prepare_input(&ctx)?;
    let quote = phonon_chain::quote;
    let script = format!(
        "#!/bin/bash\n{}\ncd {} || exit $?\n{} qe-ph-init-run -c {} -i {}\n",
        ctx.cfg.slurm.header.as_deref().unwrap_or(""),
        quote(&ctx.workspace),
        quote(
            std::env::current_exe()?
                .to_str()
                .context("Program path is not UTF-8")?
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
            "Initialization has been submitted: {};Successful submission does not represent completion of initialization",
            job.context("Unreturned Job Number")?
        );
    } else {
        println!(
            "Ready {}/ph_init.in and run_ph_init.sh ;Use --dyn0 -s Submit",
            ctx.workspace
        );
    }
    Ok(())
}
