//! LOBSTER 输入生成（`vasp_lobsterin`）：写 `lobsterin` + `run_lobster.sh` 并链接所需文件。
//!
//! 复刻 `vaspflow/cohp/vasp_lobsterin.py`：工作目录命名为
//! `{sp1}_{sp2}_{dmin}_{dmax}`，链接 WAVECAR/CONTCAR/KPOINTS/OUTCAR/POTCAR/vasprun.xml/POSCAR。

use anyhow::{Context, Result};
use std::path::Path;

fn fmt_dir(f: f64) -> String {
    if f.fract() == 0.0 {
        format!("{:.1}", f)
    } else {
        format!("{}", f)
    }
}

fn parse_zvalances(zvalances: &[String]) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for entry in zvalances {
        let (element, orbitals) = entry
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("价电子参数格式不对 '{entry}'。应为 Element:orb1,orb2"))?;
        let joined = orbitals.split(',').filter(|s| !s.trim().is_empty()).collect::<Vec<_>>().join(" ");
        out.push((element.trim().to_string(), joined));
    }
    Ok(out)
}

fn parse_custom_basis(custom_basis: &[String]) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for entry in custom_basis {
        let (element, sto) = entry
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("自定义基组参数格式不对 '{entry}'。应为 Element:file.sto"))?;
        out.push((element.trim().to_string(), sto.trim().to_string()));
    }
    Ok(out)
}

#[cfg(unix)]
fn link_or_copy(work_dir: &str, filename: &str, kspace_cohp: bool) -> Result<()> {
    use std::os::unix::fs::symlink;
    let target = format!("{work_dir}/{filename}");
    if std::fs::symlink_metadata(&target).is_ok() {
        std::fs::remove_file(&target).with_context(|| format!("remove {target}"))?;
    }
    if Path::new(filename).exists() {
        if kspace_cohp && filename == "KPOINTS" {
            std::fs::copy(filename, &target).with_context(|| format!("copy {filename}"))?;
        } else {
            let abs = std::fs::canonicalize(filename).with_context(|| format!("canonicalize {filename}"))?;
            symlink(&abs, &target).with_context(|| format!("symlink {target}"))?;
        }
    } else if filename == "CONTCAR" && Path::new("POSCAR").exists() {
        let abs = std::fs::canonicalize("POSCAR")?;
        symlink(&abs, &target).with_context(|| format!("symlink {target}"))?;
    } else {
        println!("警告: {filename} 不存在，LOBSTER 可能运行失败。");
    }
    Ok(())
}

#[cfg(not(unix))]
fn link_or_copy(work_dir: &str, filename: &str, kspace_cohp: bool) -> Result<()> {
    let target = format!("{work_dir}/{filename}");
    let _ = kspace_cohp;
    if Path::new(filename).exists() {
        std::fs::copy(filename, &target).with_context(|| format!("copy {filename}"))?;
    } else if filename == "CONTCAR" && Path::new("POSCAR").exists() {
        std::fs::copy("POSCAR", &target).with_context(|| format!("copy POSCAR"))?;
    } else {
        println!("警告: {filename} 不存在，LOBSTER 可能运行失败。");
    }
    Ok(())
}

pub struct LobsterinArgs {
    pub cohp_start: f64,
    pub cohp_end: f64,
    pub species_custom: [String; 2],
    pub zvalances: Vec<String>,
    pub d_limit: [f64; 2],
    pub orbital_wise: bool,
    pub k_space_cohp: bool,
    pub mode: i64,
    pub custom_basis: Vec<String>,
    pub lobster_execmd: String,
    pub slurm_header: String,
}

/// 生成 LOBSTER 输入文件与运行脚本，返回工作目录名。
pub fn run_lobsterin(args: &LobsterinArgs) -> Result<String> {
    let work_dir = format!(
        "{}_{}_{}_{}",
        args.species_custom[0],
        args.species_custom[1],
        fmt_dir(args.d_limit[0]),
        fmt_dir(args.d_limit[1])
    );
    std::fs::create_dir_all(&work_dir).with_context(|| format!("create {work_dir}"))?;
    println!("创建目录: {work_dir}");

    let files = ["WAVECAR", "CONTCAR", "KPOINTS", "OUTCAR", "POTCAR", "vasprun.xml", "POSCAR"];
    for f in files {
        link_or_copy(&work_dir, f, args.k_space_cohp)?;
    }
    if args.k_space_cohp {
        if Path::new("KPATH.in").exists() {
            std::fs::copy("KPATH.in", format!("{work_dir}/KPATH.in")).context("copy KPATH.in")?;
        } else {
            println!("警告: KPATH.in 不存在，已请求 kSpaceCOHP，LOBSTER 可能运行失败。");
        }
    }

    let zval = parse_zvalances(&args.zvalances)?;
    let custom = parse_custom_basis(&args.custom_basis)?;

    let mut text = String::new();
    text.push_str(&format!("COHPstartEnergy  {}\n", args.cohp_start));
    text.push_str(&format!("COHPendEnergy    {}\n", args.cohp_end));
    if !custom.is_empty() {
        text.push_str("useBasisSet custom\n");
        for (element, sto) in &custom {
            text.push_str(&format!("customSTOforAtom {element}  {sto}\n"));
        }
    } else {
        text.push_str("usebasisset pbeVaspFit2015\n");
    }
    if args.mode == 0 {
        text.push_str("gaussianSmearingWidth 0.05\n");
    }
    for (element, orbitals) in &zval {
        text.push_str(&format!("basisfunctions {element} {orbitals}\n"));
    }
    let mut cohp_gen = format!(
        "cohpGenerator from {} to {} type {} type {}",
        args.d_limit[0], args.d_limit[1], args.species_custom[0], args.species_custom[1]
    );
    if args.orbital_wise {
        cohp_gen.push_str(" orbitalWise");
    }
    if args.k_space_cohp {
        cohp_gen.push_str(" kSpaceCOHP");
    }
    text.push_str(&cohp_gen);
    text.push('\n');
    std::fs::write(format!("{work_dir}/lobsterin"), text).context("write lobsterin")?;

    // run_lobster.sh
    let path = format!("{work_dir}/run_lobster.sh");
    let mut script = String::new();
    if args.slurm_header.trim().is_empty() {
        script.push_str("#!/bin/bash\n");
    } else {
        script.push_str(args.slurm_header.trim());
        script.push_str("\n\n");
    }
    script.push_str("echo 'Job started at' `date`\n");
    if args.lobster_execmd.trim().is_empty() {
        script.push_str("echo 'lobster_executable_path 未配置'\n");
    } else {
        script.push_str(&format!("if {}; then :; else status=$?; exit \"$status\"; fi\n", qf_runner::plain_scientific_command(args.lobster_execmd.trim())));
    }
    script.push_str("echo 'Job finished at' `date`\n");
    std::fs::write(&path, script).context("write run_lobster.sh")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms)?;
    }

    println!("\nLOBSTER 输入文件和运行脚本已在 {work_dir} 准备就绪。");
    println!("提示: 如果运行 LOBSTER 后得不到 COHP，请检查 NBANDS 是否足够大。");
    println!("提示: 确保在 VASP 计算中设置了 LWAVE = .TRUE. 以生成 WAVECAR。");
    Ok(work_dir)
}
