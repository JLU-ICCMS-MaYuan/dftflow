//! wannier_init：准备 Wannier90 .win 输入（复刻 `wannier90flow/wannier_init.py`）。

use crate::config::load_init_config;
use crate::poscar::parse_poscar;
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

/// Python `DEFAULT_SLURM_HEADER`。
pub const DEFAULT_SLURM_HEADER: &str = "#!/bin/bash\n#SBATCH --job-name=wannier90\n#SBATCH --output=slurm.out\n#SBATCH --error=slurm.err\n";

/// 默认日志留在脚本目录；用户提供的 header 原样保留。
pub(crate) fn slurm_header(directory: &Path, cfg: &Value) -> String {
    match cfg["slurm"]["header"].as_str().filter(|s| !s.is_empty()) {
        Some(header) => header.to_string(),
        None => format!("{DEFAULT_SLURM_HEADER}#SBATCH --chdir={}\n", qf_runner::shlex_quote(&directory.to_string_lossy())),
    }
}

/// `generate_kpoints_from_grid`：Gamma-centered 均匀网格。
pub fn generate_kpoints_from_grid(grid: &[i64]) -> Result<Vec<[f64; 3]>> {
    if grid.len() != 3 {
        anyhow::bail!("mp_grid 必须包含 3 个整数。");
    }
    let (n1, n2, n3) = (grid[0], grid[1], grid[2]);
    if n1.min(n2).min(n3) <= 0 {
        anyhow::bail!("mp_grid 中的每个值必须 > 0。");
    }
    let mut kpts = Vec::with_capacity((n1 * n2 * n3) as usize);
    for i in 0..n1 {
        for j in 0..n2 {
            for k in 0..n3 {
                kpts.push([i as f64 / n1 as f64, j as f64 / n2 as f64, k as f64 / n3 as f64]);
            }
        }
    }
    Ok(kpts)
}

#[derive(Debug, Clone)]
pub struct KPathSegment {
    pub from: String,
    pub from_k: [f64; 3],
    pub to: String,
    pub to_k: [f64; 3],
}

/// `parse_kpath`：解析 vaspkit 303 的 KPATH.in（两行一段，4 列：kx ky kz label）。
pub fn parse_kpath(path: &str) -> Result<Vec<KPathSegment>> {
    let text = std::fs::read_to_string(path)?;
    let mut points: Vec<([f64; 3], String)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 4 {
            let Ok(kx) = parts[0].parse::<f64>() else { continue };
            let Ok(ky) = parts[1].parse::<f64>() else { continue };
            let Ok(kz) = parts[2].parse::<f64>() else { continue };
            points.push(([kx, ky, kz], parts[3].to_string()));
        }
    }
    let mut segments = Vec::new();
    let mut i = 0;
    while i + 1 < points.len() {
        segments.push(KPathSegment {
            from: points[i].1.clone(),
            from_k: points[i].0,
            to: points[i + 1].1.clone(),
            to_k: points[i + 1].0,
        });
        i += 2;
    }
    Ok(segments)
}

fn format_vector(v: &[f64; 3]) -> String {
    format!("{:16.10} {:16.10} {:16.10}", v[0], v[1], v[2])
}

fn format_kpoint(pt: &[f64; 3]) -> String {
    format!("{:12.8} {:12.8} {:12.8}", pt[0], pt[1], pt[2])
}

/// `format_pw2_value`：bool → .true./.false.；数值 → 原样；None → 'None'；其余加单引号。
pub fn format_pw2_value(v: &Value) -> String {
    match v {
        Value::Bool(b) => {
            if *b {
                ".true.".into()
            } else {
                ".false.".into()
            }
        }
        Value::Number(_) => v.to_string(),
        Value::Null => "'None'".into(),
        Value::String(s) => format!("'{s}'"),
        other => format!("'{other}'"),
    }
}

/// `resolve_pw2_input_path`：返回 (完整路径, 文件名)。
pub fn resolve_pw2_input_path(work_dir: &str, prefix: &str, cfg: &Value) -> (String, String) {
    let pw2_cfg = cfg.get("pw2wannier90");
    let input_file = pw2_cfg
        .and_then(|c| c.get("input_file"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut candidates: Vec<String> = Vec::new();
    if let Some(f) = &input_file {
        candidates.push(f.clone());
    } else {
        candidates.push(format!("{prefix}.pw2wan"));
        candidates.push(format!("{prefix}.pw2wan.in"));
    }
    for cand in &candidates {
        let path = if Path::new(cand).is_absolute() {
            cand.clone()
        } else {
            format!("{work_dir}/{cand}")
        };
        if Path::new(&path).exists() {
            let name = Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            return (path, name);
        }
    }
    let default_name = input_file.unwrap_or_else(|| format!("{prefix}.pw2wan"));
    let path = if Path::new(&default_name).is_absolute() {
        default_name
    } else {
        format!("{work_dir}/{default_name}")
    };
    let name = Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    (path, name)
}

/// `ensure_pw2wan_input`：不存在时生成默认 pw2wannier90 输入。
pub fn ensure_pw2wan_input(path: &str, prefix: &str, cfg: &Value) -> Result<()> {
    if Path::new(path).exists() {
        println!("pw2wannier90 输入文件已存在: {path}");
        return Ok(());
    }
    let pw2_cfg = cfg.get("pw2wannier90");
    // 模板中的空值表示未指定，不能输出 Fortran 无效的 'None'。
    let get = |k: &str, default: Value| -> Value {
        pw2_cfg
            .and_then(|c| c.get(k))
            .filter(|v| !v.is_null())
            .cloned()
            .unwrap_or(default)
    };
    let outdir = get("outdir", Value::String("./".into()));
    let prefix_val = get("prefix", Value::String(prefix.to_string()));
    let seedname = get("seedname", Value::String(prefix.to_string()));

    let mut lines = vec![
        "&inputpp".to_string(),
        format!("  outdir = {}", format_pw2_value(&outdir)),
        format!("  prefix = {}", format_pw2_value(&prefix_val)),
        format!("  seedname = {}", format_pw2_value(&seedname)),
    ];
    match pw2_cfg.and_then(|c| c.get("write_amn")).filter(|v| !v.is_null()) {
        Some(v) => lines.push(format!("  write_amn = {}", format_pw2_value(v))),
        None => lines.push("  write_amn = .true.".into()),
    }
    match pw2_cfg.and_then(|c| c.get("write_mmn")).filter(|v| !v.is_null()) {
        Some(v) => lines.push(format!("  write_mmn = {}", format_pw2_value(v))),
        None => lines.push("  write_mmn = .true.".into()),
    }
    // 可选空值不写入 namelist。
    for name in ["write_unk", "spin_component", "wan_mode"] {
        if let Some(value) = pw2_cfg.and_then(|c| c.get(name)).filter(|v| !v.is_null()) {
            lines.push(format!("  {name} = {}", format_pw2_value(value)));
        }
    }
    lines.push("/\n".into());

    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(path, lines.join("\n")).with_context(|| format!("write {path}"))?;
    println!("已生成 pw2wannier90 输入文件: {path}");
    Ok(())
}

/// `create_run_script`：生成 run_wannier90.sh（-pp → pw2wannier90 → wannier90）。
pub fn create_run_script(work_dir: &str, prefix: &str, cfg: &Value) -> Result<()> {
    let exec_path = qf_runner::plain_scientific_command(cfg["wannier90"]["executable_path"]
        .as_str()
        .unwrap_or("wannier90.x"));
    let pw2_exec_path = qf_runner::plain_scientific_command(cfg["pw2wannier90"]["executable_path"]
        .as_str()
        .unwrap_or("pw2wannier90.x"));
    let (pw2_input_path, pw2_input_name) = resolve_pw2_input_path(work_dir, prefix, cfg);
    ensure_pw2wan_input(&pw2_input_path, prefix, cfg)?;
    let directory = std::fs::canonicalize(work_dir)?;
    let slurm_header = slurm_header(&directory, cfg);

    let script_path = format!("{work_dir}/run_wannier90.sh");
    let content = format!(
        "{}\n\ncd -- {} || exit $?\n\necho \"Prep NNKP at $(date)\"\n{exec_path} -pp \"{prefix}\" || exit $?\n\necho \"Run pw2wannier90 at $(date)\"\nif [ ! -f \"{pw2_input_name}\" ]; then\n  echo \"缺少 pw2wannier90 输入文件: {pw2_input_name}\"\n  exit 1\nfi\n{pw2_exec_path} < \"{pw2_input_name}\" > PW2WAN.log 2>&1 || exit $?\n\necho \"Run Wannier90 (requires *.amn/*.mmn ready)\"\n{exec_path} \"{prefix}\" > WANNIER90.log 2>&1\n",
        slurm_header.trim(), qf_runner::shlex_quote(&directory.to_string_lossy())
    );
    std::fs::write(&script_path, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
    }
    println!("已生成提交脚本: {script_path}");
    Ok(())
}

/// `run_wannier90_pipeline`：-pp → pw2wannier90 → wannier90。
pub fn run_wannier90_pipeline(work_dir: &str, prefix: &str, cfg: &Value) -> Result<()> {
    let exec_path = cfg["wannier90"]["executable_path"]
        .as_str()
        .unwrap_or("wannier90.x");
    qf_wannier::require_projectability_keywords(Path::new(exec_path))?;
    let pw2_exec_path = cfg["pw2wannier90"]["executable_path"]
        .as_str()
        .unwrap_or("pw2wannier90.x");
    let (pw2_input_path, pw2_input_name) = resolve_pw2_input_path(work_dir, prefix, cfg);
    ensure_pw2wan_input(&pw2_input_path, prefix, cfg)?;

    println!("开始执行 Wannier90 流程（-pp -> pw2wannier90 -> wannier90）...");
    let cmd_pp = format!("{exec_path} -pp \"{prefix}\"");
    println!("执行命令: {cmd_pp}");
    qf_runner::run_shell(&cmd_pp, Path::new(work_dir))?;
    let cmd_pw2 = format!("{pw2_exec_path} < \"{pw2_input_name}\" > PW2WAN.log 2>&1");
    println!("执行命令: {cmd_pw2}");
    qf_runner::run_shell(&cmd_pw2, Path::new(work_dir))?;
    let cmd_w90 = format!("{exec_path} \"{prefix}.win\" > WANNIER90.log 2>&1");
    println!("执行命令: {cmd_w90}");
    qf_runner::run_shell(&cmd_w90, Path::new(work_dir))?;
    Ok(())
}

/// `write_win`：写出 .win 文件（格式与字段顺序 1:1 复刻）。
pub fn write_win(
    output: &str,
    struct_: &crate::poscar::Poscar,
    cfg: &Value,
    kpoints: &[[f64; 3]],
    band_path: &[KPathSegment],
) -> Result<()> {
    let init_cfg = &cfg["wannier_init"];
    let system_name = init_cfg["system_name"]
        .as_str()
        .filter(|s| !s.is_empty())
        .unwrap_or(if struct_.comment.is_empty() {
            "wannier_system"
        } else {
            &struct_.comment
        });
    let projections: Vec<&str> = init_cfg["projections"]["list"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    let kpt_cfg = &init_cfg["k_points"];

    if projections.is_empty() {
        anyhow::bail!("配置缺少投影 (projections)。");
    }
    let Some(num_wann) = init_cfg.get("num_wann").and_then(|v| v.as_i64()) else {
        anyhow::bail!("配置缺少 num_wann。");
    };
    anyhow::ensure!(num_wann > 0, "num_wann 必须为正");
    let bands_file = init_cfg["bands_file"].as_str().context(
        "上游默认组合选窗需要 wannier_init.bands_file 指向实际 QE NSCF XML；旧手填窗生成路径已弃用")?;
    let (bands, fermi) = qf_wannier::read_qe_bands(Path::new(bands_file))?;
    anyhow::ensure!(bands.len() == kpoints.len(), "bands_file k 点数与生成的 Wannier 网格不符");
    if let Some(value) = init_cfg["fermi_energy"].as_f64() {
        anyhow::ensure!((value - fermi).abs() <= 1e-8, "配置 Fermi 能与 bands_file 不一致");
    }
    let exclude_bands: Vec<usize> = if let Some(value) = init_cfg.get("exclude_bands").filter(|v| !v.is_null()) {
        value.as_array().context("exclude_bands 必须是 1-based 整数数组")?.iter()
            .map(|v| v.as_u64().and_then(|n| usize::try_from(n).ok()).context("非法排除带号"))
            .collect::<Result<_>>()?
    } else { Vec::new() };
    let bound = |key: &str| -> Result<Option<f64>> {
        init_cfg.get(key).filter(|v| !v.is_null())
            .map(|v| v.as_f64().with_context(|| format!("{key} 必须是有限能量数值"))).transpose()
    };
    let bounds = qf_wannier::Bounds {
        dis_win_min: bound("dis_win_min")?,
        dis_win_max: bound("dis_win_max")?,
        dis_froz_min: bound("dis_froz_min")?,
        dis_froz_max: bound("dis_froz_max")?,
    };
    let selection = qf_wannier::select_default(&bands, fermi, num_wann as usize,
        &exclude_bands, &bounds, qf_wannier::Reference::UpstreamRelative)?;
    let num_bands = bands[0].len() - selection.excluded_bands.len();
    if let Some(nb) = init_cfg["num_bands"].as_u64() {
        anyhow::ensure!(nb as usize == num_bands, "num_bands 与排除后的实际能带数不符");
    }

    let get_i = |k: &str| init_cfg.get(k).and_then(|v| v.as_i64());
    let get_b = |k: &str| init_cfg.get(k).and_then(|v| v.as_bool());

    let mut out = String::new();
    out.push_str(&format!("num_wann = {num_wann}\n"));
    out.push_str(&format!("num_bands = {num_bands}\n"));
    if !selection.excluded_bands.is_empty() {
        out.push_str(&format!("exclude_bands = {}\n", selection.excluded_bands.iter().map(usize::to_string).collect::<Vec<_>>().join(",")));
    }
    if let Some(v) = get_i("iprint") {
        out.push_str(&format!("iprint = {v}\n"));
    }
    if let Some(v) = get_i("num_iter") {
        out.push_str(&format!("num_iter = {v}\n"));
    }
    if let Some(v) = get_i("dis_num_iter") {
        out.push_str(&format!("dis_num_iter = {v}\n"));
    }
    for (k, v) in &selection.parameters {
        out.push_str(&format!("{k} = {v:.16e}\n"));
    }
    out.push_str(qf_wannier::PROJECTABILITY_SWITCH);
    out.push('\n');
    for k in ["write_hr", "write_bvec"] {
        if let Some(v) = get_b(k) {
            out.push_str(&format!("{k} = {}\n", bool_lower(v)));
        }
    }
    let mut bands_plot = get_b("bands_plot");
    if let Some(bp) = bands_plot {
        if bp && band_path.is_empty() {
            println!("提示：已请求 bands_plot，但未找到 kpoint_path，将自动关闭 bands_plot。");
            bands_plot = Some(false);
            out.push_str("bands_plot = false\n");
        } else {
            out.push_str(&format!("bands_plot = {}\n", bool_lower(bp)));
        }
    }
    if let Some(fmt) = init_cfg.get("bands_plot_format").and_then(|v| v.as_str()) {
        if bands_plot == Some(true) {
            out.push_str(&format!("bands_plot_format = {fmt}\n"));
        }
    }
    if let Some(v) = get_b("wvfn_formatted") {
        out.push_str(&format!("wvfn_formatted = {}\n", bool_lower(v)));
    }
    let wannier_plot = get_b("wannier_plot");
    if let Some(v) = wannier_plot {
        out.push_str(&format!("wannier_plot = {}\n", bool_lower(v)));
    }
    if let Some(fmt) = init_cfg.get("wannier_plot_format").and_then(|v| v.as_str()) {
        if wannier_plot == Some(true) {
            out.push_str(&format!("wannier_plot_format = {fmt}\n"));
        }
    }
    out.push_str(&format!("fermi_energy = {fermi:.16e}\n"));
    if let Some(v) = get_b("fermi_surface_plot") {
        out.push_str(&format!("fermi_surface_plot = {}\n", bool_lower(v)));
    }
    if let Some(v) = get_b("dos") {
        out.push_str(&format!("dos = {}\n", bool_lower(v)));
    }
    if let Some(v) = init_cfg.get("dos_kmesh") {
        if !v.is_null() {
            let value = if let Some(arr) = v.as_array() {
                arr.iter()
                    .filter_map(|x| x.as_i64())
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                value_to_plain_string(v)
            };
            out.push_str(&format!("dos_kmesh = {value}\n"));
        }
    }
    if let Some(v) = init_cfg.get("dos_project") {
        if !v.is_null() {
            let value = if let Some(arr) = v.as_array() {
                arr.iter()
                    .map(value_to_plain_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                value_to_plain_string(v)
            };
            out.push_str(&format!("dos_project = {value}\n"));
        }
    }

    let mp_grid: Vec<String> = kpt_cfg["mp_grid"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_i64())
                .map(|x| x.to_string())
                .collect()
        })
        .unwrap_or_default();
    out.push_str(&format!("mp_grid = {}\n", mp_grid.join(" ")));
    out.push('\n');

    out.push_str("begin projections\n");
    for proj in &projections {
        out.push_str(&format!(" {proj}\n"));
    }
    out.push_str("end projections\n\n");

    out.push_str("begin unit_cell_cart\nAng\n");
    for row in &struct_.lattice {
        out.push_str(&format!("{}\n", format_vector(row)));
    }
    out.push_str("end unit_cell_cart\n\n");

    if struct_.coord_type == "direct" {
        out.push_str("begin atoms_frac\n");
    } else {
        out.push_str("begin atoms_cart\nAng\n");
    }
    let mut atom_idx = 0usize;
    for (el, count) in struct_.elements.iter().zip(struct_.counts.iter()) {
        for _ in 0..*count {
            let pos = &struct_.positions[atom_idx];
            out.push_str(&format!("{el:<3} {}\n", format_kpoint(pos)));
            atom_idx += 1;
        }
    }
    if struct_.coord_type == "direct" {
        out.push_str("end atoms_frac\n\n");
    } else {
        out.push_str("end atoms_cart\n\n");
    }

    out.push_str("begin kpoints\n");
    for pt in kpoints {
        out.push_str(&format!("{}\n", format_kpoint(pt)));
    }
    out.push_str("end kpoints\n\n");

    if !band_path.is_empty() {
        out.push_str("begin kpoint_path\n");
        for seg in band_path {
            out.push_str(&format!(
                "{:<6} {:8.4} {:8.4} {:8.4}   {:<6} {:8.4} {:8.4} {:8.4}\n",
                seg.from, seg.from_k[0], seg.from_k[1], seg.from_k[2],
                seg.to, seg.to_k[0], seg.to_k[1], seg.to_k[2]
            ));
        }
        out.push_str("end kpoint_path\n\n");
    }

    out.push_str(&format!("! system_name: {system_name}\n"));

    std::fs::write(output, out).with_context(|| format!("write {output}"))?;
    qf_wannier::write_selection(&Path::new(output).with_extension("window-selection.json"), &selection)?;
    Ok(())
}

/// Python `str(bool).lower()`。
fn bool_lower(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

fn value_to_plain_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => bool_lower(*b).to_string(),
        other => other.to_string(),
    }
}

/// wannier_init 主流程（对齐 `main()`）。
pub fn run(input: &str, config: &str, run: bool, submit: bool) -> Result<()> {
    let work_dir = "wannier90";
    std::fs::create_dir_all(work_dir)?;

    let struct_ = parse_poscar(input)?;
    std::fs::copy(input, format!("{work_dir}/POSCAR"))
        .with_context(|| format!("copy POSCAR to {work_dir}"))?;
    let cfg = load_init_config(config)?;
    let kpt_cfg = &cfg["wannier_init"]["k_points"];

    let kpoints_opt: Option<Vec<[f64; 3]>> = if let Some(kps) = kpt_cfg.get("kpoints").and_then(|v| v.as_array()) {
        if kps.is_empty() {
            None
        } else {
            Some(
                kps.iter()
                    .map(|kp| {
                        let arr = kp.as_array().cloned().unwrap_or_default();
                        [
                            arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0),
                            arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0),
                            arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0),
                        ]
                    })
                    .collect(),
            )
        }
    } else {
        None
    };
    let kpoints = match kpoints_opt {
        Some(k) => k,
        None => {
            let grid = kpt_cfg["mp_grid"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_i64()).collect::<Vec<_>>())
                .unwrap_or_default();
            generate_kpoints_from_grid(&grid)?
        }
    };

    let bands_plot_cfg = cfg["wannier_init"]["bands_plot"].as_bool().unwrap_or(false);
    let kpath_file_cfg = kpt_cfg["kpath_file"].as_str().unwrap_or("KPATH.in");
    let kpath_file = if Path::new(kpath_file_cfg).is_absolute() {
        kpath_file_cfg.to_string()
    } else {
        format!("{work_dir}/{kpath_file_cfg}")
    };
    let mut band_segments: Vec<KPathSegment> = Vec::new();
    if bands_plot_cfg && !Path::new(&kpath_file).exists() {
        println!("检测到 bands_plot=true，开始运行 vaspkit 303 生成 KPATH.in ...");
        vaspkit_kpath(work_dir)?;
    }
    if Path::new(&kpath_file).exists() {
        band_segments = parse_kpath(&kpath_file)?;
        println!("从 {kpath_file} 读取到 {} 段能带路径。", band_segments.len());
    } else {
        println!("未找到 {kpath_file}，将不写入 kpoint_path。");
    }

    let basename = cfg["wannier_init"]["system_name"]
        .as_str()
        .filter(|s| !s.is_empty())
        .unwrap_or(if struct_.comment.is_empty() {
            "wannier90"
        } else {
            &struct_.comment
        })
        .to_string();
    let output = format!("{work_dir}/{basename}.win");

    write_win(&output, &struct_, &cfg, &kpoints, &band_segments)?;
    create_run_script(work_dir, &basename, &cfg)?;
    println!("已生成 {output}，包含 {} 个 k 点。", kpoints.len());
    if run {
        qf_wannier::require_projectability_keywords(Path::new(cfg["wannier90"]["executable_path"].as_str().unwrap_or("wannier90.x")))?;
        run_wannier90_pipeline(work_dir, &basename, &cfg)?;
    } else if submit {
        qf_wannier::require_projectability_keywords(Path::new(cfg["wannier90"]["executable_path"].as_str().unwrap_or("wannier90.x")))?;
        qf_runner::submit_via_sbatch(Path::new(work_dir), "run_wannier90.sh")?;
    }
    Ok(())
}

/// vaspkit task 303：在 `work_dir` 生成 KPATH.in（stdin `3\n303\n`）。
pub fn vaspkit_kpath(work_dir: &str) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new("vaspkit")
        .current_dir(work_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| anyhow::anyhow!("vaspkit 运行失败，无法生成 KPATH.in: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"3\n303\n");
    }
    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("vaspkit 运行失败，无法生成 KPATH.in");
    }
    Ok(())
}
