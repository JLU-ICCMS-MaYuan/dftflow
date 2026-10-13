//! VASP 结构优化（`vasp_opt` rv4/rv1/rvf）输入生成。
//!
//! 复刻 `vaspflow/opt/vasp_opt.py`：rv4 写出 `INCAR_1..4` + `fouropt.sh`，rv1/rvf 写出
//! 单个 `INCAR` + `oneopt.sh`/`fopt.sh`；统一生成 POTCAR 并拷贝 POSCAR。

use anyhow::{bail, Context, Result};
use qf_config::SectionMap;
use serde_json::Value;
use std::path::Path;

use crate::potcar;

/// 数值格式化：整数按整数输出；浮点对齐 Python `float.__repr__`（指数至少两位）。
fn fmt_num(v: &Value) -> String {
    if let Some(i) = v.as_i64() {
        return i.to_string();
    }
    if let Some(f) = v.as_f64() {
        let s = format!("{}", f);
        if let Some(pos) = s.find('e') {
            let mant = &s[..pos];
            let exp: i32 = s[pos + 1..].parse().unwrap_or(0);
            return format!("{}e{:02}", mant, exp);
        }
        return s;
    }
    v.to_string().trim_matches('"').to_string()
}

fn kv(key: &str, v: &Value, width: usize) -> String {
    format!("{:<width$} = {}", key, fmt_num(v), width = width)
}

fn get<'a>(params: &'a SectionMap, key: &str) -> Value {
    params.get(key).cloned().unwrap_or(Value::Null)
}

fn resolve_kspacing(params: &SectionMap) -> f64 {
    if let Some(Value::Number(_)) = params.get("KSPACING") {
        return params.get("KSPACING").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }
    let kmesh = params.get("kmesh").and_then(|v| v.as_f64()).unwrap_or(0.025);
    kmesh * 2.0 * 3.1415926
}

fn write_incar(path: &str, lines: &[String]) -> Result<()> {
    let mut text = String::new();
    for l in lines {
        text.push_str(l);
        text.push('\n');
    }
    std::fs::write(path, text).with_context(|| format!("write {path}"))?;
    Ok(())
}

/// stage1：POTIM 插在 ISIF 与 LWAVE 之间（对齐 Python `insert(-3)`）。
fn build_incar1(params: &SectionMap, symprec: &Value, pstress: f64) -> Vec<String> {
    let mut lines = vec![
        kv("ISTART", &get(params, "ISTART"), 9),
        kv("ICHARG", &get(params, "ICHARG"), 9),
        kv("ISYM", &get(params, "ISYM"), 9),
        kv("ENCUT", &get(params, "ENCUT"), 9),
        kv("PREC", &get(params, "PREC"), 9),
        kv("SYMPREC", symprec, 9),
        kv("NCORE", &get(params, "NCORE"), 9),
        kv("KSPACING", &get(params, "KSPACING"), 9),
        kv("ISMEAR", &get(params, "ISMEAR"), 9),
        kv("SIGMA", &get(params, "SIGMA"), 9),
        kv("NELM", &get(params, "NELM"), 9),
        kv("NELMIN", &get(params, "NELMIN"), 9),
        kv("EDIFF", &get(params, "EDIFF"), 9),
        kv("EDIFFG", &get(params, "EDIFFG"), 9),
        kv("NSW", &get(params, "NSW"), 9),
        kv("IBRION", &get(params, "IBRION"), 9),
        kv("ISIF", &get(params, "ISIF"), 9),
        kv("LWAVE", &get(params, "LWAVE"), 9),
        kv("LCHARG", &get(params, "LCHARG"), 9),
    ];
    if get(params, "POTIM") != Value::Null {
        let idx = lines.len() - 2; // 插在 PSTRESS 之前
        lines.insert(idx, kv("POTIM", &get(params, "POTIM"), 9));
    }
    lines.push(format!("{:<9} = {}", "PSTRESS", pstress));
    lines
}

/// stage2/stage3：POTIM 在 ISIF 之后、LWAVE 之前。
fn build_incar23(params: &SectionMap, symprec: &Value, pstress: f64) -> Vec<String> {
    vec![
        kv("ISTART", &get(params, "ISTART"), 9),
        kv("ICHARG", &get(params, "ICHARG"), 9),
        kv("ISYM", &get(params, "ISYM"), 9),
        kv("ENCUT", &get(params, "ENCUT"), 9),
        kv("PREC", &get(params, "PREC"), 9),
        kv("SYMPREC", symprec, 9),
        kv("NCORE", &get(params, "NCORE"), 9),
        kv("KSPACING", &get(params, "KSPACING"), 9),
        kv("ISMEAR", &get(params, "ISMEAR"), 9),
        kv("SIGMA", &get(params, "SIGMA"), 9),
        kv("NELM", &get(params, "NELM"), 9),
        kv("NELMIN", &get(params, "NELMIN"), 9),
        kv("EDIFF", &get(params, "EDIFF"), 9),
        kv("EDIFFG", &get(params, "EDIFFG"), 9),
        kv("NSW", &get(params, "NSW"), 9),
        kv("IBRION", &get(params, "IBRION"), 9),
        kv("ISIF", &get(params, "ISIF"), 9),
        kv("POTIM", &get(params, "POTIM"), 9),
        kv("LWAVE", &get(params, "LWAVE"), 9),
        kv("LCHARG", &get(params, "LCHARG"), 9),
        format!("{:<9} = {}", "PSTRESS", pstress),
    ]
}

/// 终态 INCAR（INCAR_4 / INCAR）：KSPACING 由 kmesh 推导，NPAR/KPAR 按存在与否写出。
fn build_incar_final(params: &SectionMap, symprec: &Value, pstress: f64) -> Vec<String> {
    let mut lines = vec![
        kv("ISTART", &get(params, "ISTART"), 9),
        kv("ICHARG", &get(params, "ICHARG"), 9),
        kv("ISYM", &get(params, "ISYM"), 9),
        kv("ENCUT", &get(params, "ENCUT"), 9),
        kv("PREC", &get(params, "PREC"), 9),
        kv("SYMPREC", symprec, 9),
    ];
    if get(params, "NPAR") != Value::Null {
        lines.push(kv("NPAR", &get(params, "NPAR"), 7));
    }
    if get(params, "NCORE") != Value::Null {
        lines.push(kv("NCORE", &get(params, "NCORE"), 9));
    }
    if get(params, "KPAR") != Value::Null {
        lines.push(kv("KPAR", &get(params, "KPAR"), 7));
    }
    lines.push(kv("KSPACING", &Value::from(resolve_kspacing(params)), 9));
    lines.extend(vec![
        kv("ISMEAR", &get(params, "ISMEAR"), 9),
        kv("SIGMA", &get(params, "SIGMA"), 9),
        kv("NELM", &get(params, "NELM"), 9),
        kv("NELMIN", &get(params, "NELMIN"), 9),
        kv("EDIFF", &get(params, "EDIFF"), 9),
        kv("EDIFFG", &get(params, "EDIFFG"), 9),
        kv("NSW", &get(params, "NSW"), 9),
        kv("IBRION", &get(params, "IBRION"), 9),
        kv("ISIF", &get(params, "ISIF"), 9),
        kv("POTIM", &get(params, "POTIM"), 9),
        kv("LWAVE", &get(params, "LWAVE"), 9),
        kv("LCHARG", &get(params, "LCHARG"), 9),
        format!("{:<9} = {}", "PSTRESS", pstress),
    ]);
    lines
}

pub fn build_mode_commands(mode: &str, vasp_execmd: &str) -> Vec<String> {
    match mode {
        "rv4" => vec![
            "for i in {1..4}; do".to_string(),
            "cp -f INCAR_$i INCAR".to_string(),
            format!("{vasp_execmd} > vasp.log_$i 2>&1"),
            "cp -f CONTCAR CONTCAR_$i".to_string(),
            "cp -f OUTCAR OUTCAR_$i".to_string(),
            "cp -f CONTCAR POSCAR".to_string(),
            "done".to_string(),
        ],
        "rv1" => vec![vasp_execmd.to_string()],
        "rvf" => vec![
            "num=0".to_string(),
            "while true;do".to_string(),
            "        let num+=1".to_string(),
            "        echo \"run fine vasp opt-$num\"".to_string(),
            format!("        timeout 14400s {vasp_execmd}"),
            "        cp -f CONTCAR CONTCAR-fine &&  cp -f CONTCAR POSCAR".to_string(),
            "        rows=`sed -n '/F\\=/p' OSZICAR | wc -l`".to_string(),
            "        echo \"rows-$rows\"".to_string(),
            "        echo $num >> count_opt_times".to_string(),
            "        if [ \"$rows\" -eq \"1\" ];then".to_string(),
            "                break".to_string(),
            "        fi".to_string(),
            "        if [ \"$num\" -gt \"50\" ]; then".to_string(),
            "                break".to_string(),
            "        fi".to_string(),
            "done".to_string(),
        ],
        _ => vec![],
    }
}

fn write_script(work_dir: &str, name: &str, slurm_header: &str, commands: &[String]) -> Result<String> {
    let path = format!("{work_dir}/{name}");
    let mut text = String::new();
    if slurm_header.trim().is_empty() {
        text.push_str("#!/bin/bash\n");
    } else {
        text.push_str(slurm_header.trim());
        text.push_str("\n\n");
    }
    for c in commands {
        text.push_str(c);
        text.push('\n');
    }
    std::fs::write(&path, text).with_context(|| format!("write {path}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(name.to_string())
}

fn prepare_poscar(work_dir: &str, struct_file: Option<&str>) -> Result<()> {
    let target = format!("{work_dir}/POSCAR");
    if let Some(sf) = struct_file {
        if !Path::new(sf).exists() {
            bail!("找不到结构文件: {sf}");
        }
        std::fs::copy(sf, &target).with_context(|| format!("copy {sf} -> {target}"))?;
    } else if Path::new("POSCAR").exists() {
        let abs_src = std::fs::canonicalize("POSCAR")?;
        let abs_tgt = std::fs::canonicalize(&target).unwrap_or_default();
        if abs_src != abs_tgt {
            std::fs::copy("POSCAR", &target).with_context(|| format!("copy POSCAR -> {target}"))?;
        }
    }
    Ok(())
}

pub struct VaspOptArgs {
    pub work_dir: String,
    pub struct_file: Option<String>,
    pub mode: String,
    pub pressure_gpa: Option<f64>,
    pub vasp_execmd: String,
    pub potcar_dir: String,
    pub slurm_header: String,
}

/// 生成 vasp_opt 输入（INCAR/脚本/POTCAR/POSCAR）。返回脚本文件名。
pub fn run_vasp_opt(cfg_vasp_opt: &SectionMap, cfg_stages: &SectionMap, args: &VaspOptArgs) -> Result<String> {
    let (vopt, s1, s2, s3) = crate::templates::vasp_opt_merged(cfg_vasp_opt, cfg_stages);
    let pstress = match args.pressure_gpa {
        Some(gpa) => gpa * 10.0,
        None => vopt
            .get("PSTRESS")
            .and_then(|v| v.as_f64())
            .context("缺少 PSTRESS，且未通过 -p 指定")?,
    };
    std::fs::create_dir_all(&args.work_dir)
        .with_context(|| format!("create {}", args.work_dir))?;
    prepare_poscar(&args.work_dir, args.struct_file.as_deref())?;
    let structure = qf_crystal::parse_poscar(&format!("{}/POSCAR", args.work_dir))
        .map_err(|e| anyhow::anyhow!("解析结构失败: {e}"))?;
    let elements = potcar::distinct_elements(&structure);
    if !args.potcar_dir.is_empty() {
        potcar::concat_potcar(&args.potcar_dir, &elements, &format!("{}/POTCAR", args.work_dir))?;
    }
    let symprec = vopt.get("SYMPREC").cloned().unwrap_or(Value::Null);

    let script = match args.mode.as_str() {
        "rv4" => {
            write_incar(&format!("{}/INCAR_1", args.work_dir), &build_incar1(&s1, &symprec, pstress))?;
            write_incar(&format!("{}/INCAR_2", args.work_dir), &build_incar23(&s2, &symprec, pstress))?;
            write_incar(&format!("{}/INCAR_3", args.work_dir), &build_incar23(&s3, &symprec, pstress))?;
            write_incar(&format!("{}/INCAR_4", args.work_dir), &build_incar_final(&vopt, &symprec, pstress))?;
            write_script(&args.work_dir, "fouropt.sh", &args.slurm_header, &build_mode_commands("rv4", &args.vasp_execmd))?
        }
        "rv1" => {
            write_incar(&format!("{}/INCAR", args.work_dir), &build_incar_final(&vopt, &symprec, pstress))?;
            write_script(&args.work_dir, "oneopt.sh", &args.slurm_header, &build_mode_commands("rv1", &args.vasp_execmd))?
        }
        "rvf" => {
            write_incar(&format!("{}/INCAR", args.work_dir), &build_incar_final(&vopt, &symprec, pstress))?;
            write_script(&args.work_dir, "fopt.sh", &args.slurm_header, &build_mode_commands("rvf", &args.vasp_execmd))?
        }
        other => bail!("mode 必须为 rv4/rv1/rvf，收到 {other}"),
    };
    Ok(script)
}

/// 多压强批量提交脚本（复刻 `_create_batch_submit_script`）。
pub fn write_batch_script(name: &str, slurm_header: &str, cases: &[(String, Vec<String>)]) -> Result<String> {
    let path = format!("{name}");
    let mut text = String::new();
    if slurm_header.trim().is_empty() {
        text.push_str("#!/bin/bash\n");
    } else {
        text.push_str(slurm_header.trim());
        text.push_str("\n\n");
    }
    for (case_dir, commands) in cases {
        text.push_str(&format!("cd \"{case_dir}\"\n"));
        for c in commands {
            text.push_str(c);
            text.push('\n');
        }
        text.push('\n');
    }
    std::fs::write(&path, text).with_context(|| format!("write {path}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(name.to_string())
}
