//! 电子结构输入卡写出：eband（pw.x bands + bands.x + projwfc.x）与 eledos（dos.x + projwfc.x）。
//! 复刻 `qeflow/input_deck/bands.py`、`dos.py` 与 `kpath.py`。

use std::fs;
use std::path::Path;

use anyhow::{bail, Result};
use qf_config::{merge_recursive, Config, SectionMap};
use qf_crystal::kmesh::reciprocal_lattice;
use qf_crystal::Structure;
use serde_json::json;
use serde_json::Value;

use crate::pw::{build_nested_pw_params, write_pw_input_with_precision};
use crate::to_qe_value;

/// eband 配置段模板（对齐 `qeflow/templates.py: get_qe_eband_parameters_template`）。
pub fn eband_template() -> Value {
    json!({
        "CONTROL": {
            "calculation": "bands",
            "restart_mode": "from_scratch",
            "prefix": "qe",
            "outdir": "./tmp",
            "pseudo_dir": "./pseudo",
            "verbosity": "high",
            "tprnfor": true,
            "tstress": true,
            "forc_conv_thr": "1.0e-6",
            "etot_conv_thr": "1.0e-7"
        },
        "SYSTEM": { "ecutwfc": 50.0, "ecutrho": 400.0, "occupations": "tetrahedra" },
        "ELECTRONS": { "conv_thr": 1.0e-8, "mixing_beta": 0.7, "electron_maxstep": 400 },
        "k_points": { "kpath_points": 100, "kpath_file": "KPATH.in" },
        "eband": { "filband": null, "lsym": true },
        "bands": { "executable_path": null, "args": "" },
        "projwfc": { "executable_path": null, "args": "-pd .true." }
    })
}

/// eledos 配置段模板（对齐 `qeflow/templates.py: get_qe_eledos_parameters_template`）。
pub fn eledos_template() -> Value {
    json!({
        "CONTROL": { "outdir": "./tmp" },
        "SYSTEM": {},
        "ELECTRONS": {},
        "eledos": {
            "DeltaE": 0.01, "emin": -10, "emax": 30, "ngauss": 0, "degauss": 0.0,
            "fildos": null, "filpdos": null, "filproj": null, "outdir": null
        },
        "dos": { "executable_path": null, "args": "-pd .true." },
        "projwfc": { "executable_path": null, "args": "-pd .true." }
    })
}

/// 将模板与用户段递归合并（base=模板，override=用户），返回合并后的 `qe_*` 配置段。
pub fn merge_section(template: &Value, user: &SectionMap) -> SectionMap {
    let user_v = serde_json::to_value(user).unwrap_or(Value::Null);
    let mut merged = template.clone();
    merge_recursive(&mut merged, &user_v);
    serde_json::from_value(merged).unwrap_or_default()
}

/// 应用模板后的 eband 配置段。
pub fn eband_merged(config: &Config) -> SectionMap {
    merge_section(&eband_template(), &config.qe_eband)
}

/// 应用模板后的 eledos 配置段。
pub fn eledos_merged(config: &Config) -> SectionMap {
    merge_section(&eledos_template(), &config.qe_eledos)
}

fn as_str_opt(v: Option<&Value>) -> Option<String> {
    v.and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn control_outdir(section: &SectionMap) -> String {
    as_str_opt(section.get("CONTROL").and_then(|v| v.get("outdir")))
        .map(|s| s.trim_matches('\'').trim_matches('"').to_string())
        .unwrap_or_else(|| "./tmp".to_string())
}

fn eledos_outdir(section: &SectionMap) -> String {
    let from_eledos = section
        .get("eledos")
        .and_then(|v| v.get("outdir"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim_matches('\'').trim_matches('"').to_string());
    from_eledos.unwrap_or_else(|| control_outdir(section))
}

type KPoint = ([f64; 3], String);

/// 解析 KPATH.in：每行 4 字段 `kx ky kz label`；两两成段，相邻段端点重合则合并。
/// 对齐 `qeflow/input_deck/kpath.py: parse_kpath_segments`。
pub fn parse_kpath_segments(path: &str) -> Result<Vec<Vec<KPoint>>> {
    let text = fs::read_to_string(path).map_err(|_| anyhow::anyhow!("找不到 KPATH.in: {path}"))?;
    let mut points: Vec<KPoint> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 4 {
            continue;
        }
        // VASPKIT 的标题行也可能正好四列，按 Python 参考跳过非坐标行。
        let (Ok(x), Ok(y), Ok(z)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>(), parts[2].parse::<f64>()) else { continue; };
        let coords = [x, y, z];
        points.push((coords, parts[3].to_string()));
    }
    if points.len() % 2 != 0 {
        bail!("KPATH.in 格式异常，点数为奇数: {}", points.len());
    }
    if points.is_empty() {
        bail!("KPATH.in 未解析到任何高对称点");
    }
    let mut segments: Vec<Vec<KPoint>> = Vec::new();
    let tol = 1.0e-6;
    for pair in points.chunks(2) {
        let (k1, l1) = &pair[0];
        let (k2, l2) = &pair[1];
        if segments.is_empty() {
            segments.push(vec![(k1.clone(), l1.clone()), (k2.clone(), l2.clone())]);
            continue;
        }
        let prev_end = &segments.last().unwrap().last().unwrap().0;
        let coincident = (prev_end[0] - k1[0]).abs() < tol
            && (prev_end[1] - k1[1]).abs() < tol
            && (prev_end[2] - k1[2]).abs() < tol;
        if coincident {
            segments.last_mut().unwrap().push((k2.clone(), l2.clone()));
        } else {
            segments.push(vec![(k1.clone(), l1.clone()), (k2.clone(), l2.clone())]);
        }
    }
    Ok(segments)
}

/// 生成 `K_POINTS crystal_b` 文本（含各点权重与标签）。对齐 `write_eband_kpoints`。
fn eband_kpoints_text(segments: &[Vec<KPoint>], kpath_points: i64) -> String {
    let nseg = segments.len();
    let mut lines = Vec::new();
    for (si, seg) in segments.iter().enumerate() {
        let npt = seg.len();
        for (pi, (coords, label)) in seg.iter().enumerate() {
            let is_last = pi == npt - 1;
            let is_jump = is_last && si < nseg - 1;
            let is_final = is_last && si == nseg - 1;
            let weight = if is_jump || is_final { 1 } else { kpath_points };
            lines.push(format!(
                "  {x:12.8} {y:12.8} {z:12.8} {w:6} ! {label}\n",
                x = coords[0],
                y = coords[1],
                z = coords[2],
                w = weight
            ));
        }
    }
    let total = lines.len();
    let mut s = String::from("K_POINTS crystal_b\n");
    s.push_str(&format!("{total}\n"));
    for l in &lines {
        s.push_str(l);
    }
    s
}

fn dot_k(coords: &[f64; 3], recip: &[[f64; 3]; 3]) -> [f64; 3] {
    [
        coords[0] * recip[0][0] + coords[1] * recip[1][0] + coords[2] * recip[2][0],
        coords[0] * recip[0][1] + coords[1] * recip[1][1] + coords[2] * recip[2][1],
        coords[0] * recip[0][2] + coords[1] * recip[1][2] + coords[2] * recip[2][2],
    ]
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// 3×3 矩阵求逆（克拉默法则）。
fn mat3_inv(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let inv_det = 1.0 / det;
    let mut inv = [[0.0f64; 3]; 3];
    inv[0][0] = (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det;
    inv[0][1] = (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det;
    inv[0][2] = (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det;
    inv[1][0] = (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det;
    inv[1][1] = (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det;
    inv[1][2] = (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det;
    inv[2][0] = (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det;
    inv[2][1] = (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det;
    inv[2][2] = (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det;
    inv
}

/// gun 风格倒格子（对齐 `write_gun_kpath_labels`）：bohr 缩放后 `inv(lattice).T * alat`。
fn gun_reciprocal_lattice(lattice: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let bohr = 0.52917721092;
    let lb = [
        [lattice[0][0] / bohr, lattice[0][1] / bohr, lattice[0][2] / bohr],
        [lattice[1][0] / bohr, lattice[1][1] / bohr, lattice[1][2] / bohr],
        [lattice[2][0] / bohr, lattice[2][1] / bohr, lattice[2][2] / bohr],
    ];
    let alat = norm(lb[0]);
    let inv = mat3_inv(&lb);
    let mut r = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = inv[j][i] * alat;
        }
    }
    r
}

/// 写出 `{prefix}_gun_band.labelinfo.dat`（对齐 `write_gun_kpath_labels`）。
fn write_gun_labelinfo(
    workspace: &str,
    prefix: &str,
    segments: &[Vec<KPoint>],
    lattice: &[[f64; 3]; 3],
) -> Result<String> {
    let recip = gun_reciprocal_lattice(lattice);
    let mut label_points: Vec<KPoint> = Vec::new();
    for (si, seg) in segments.iter().enumerate() {
        if si == 0 {
            label_points.extend(seg.iter().cloned());
            continue;
        }
        let prev_end = segments[si - 1].last().unwrap().clone();
        label_points.push(prev_end);
        label_points.extend(seg[1..].iter().cloned());
    }
    let mut total_dist = 0.0f64;
    let mut last: Option<[f64; 3]> = None;
    let path = format!("{workspace}/{prefix}_gun_band.labelinfo.dat");
    let mut s = String::from("# label dist kx ky kz\n");
    for (coords, label) in &label_points {
        let cart = dot_k(coords, &recip);
        if let Some(prev) = last {
            total_dist += norm([cart[0] - prev[0], cart[1] - prev[1], cart[2] - prev[2]]);
        }
        s.push_str(&format!(
            "{label:8} {dist:12.6} {x:12.8} {y:12.8} {z:12.8}\n",
            label = label,
            dist = total_dist,
            x = coords[0],
            y = coords[1],
            z = coords[2]
        ));
        last = Some(cart);
    }
    fs::write(&path, &s)?;
    Ok(path)
}

/// 写出 `{prefix}_band.labelinfo.dat`（对齐 `write_indexed_kpath_labels`）。
fn write_indexed_labelinfo(
    workspace: &str,
    prefix: &str,
    segments: &[Vec<KPoint>],
    lattice: &[[f64; 3]; 3],
    weights: &[i64],
) -> Result<String> {
    let recip = reciprocal_lattice(*lattice);
    let mut label_points: Vec<KPoint> = Vec::new();
    let mut jump_starts: Vec<bool> = Vec::new();
    for (si, seg) in segments.iter().enumerate() {
        for (pi, kp) in seg.iter().enumerate() {
            label_points.push(kp.clone());
            jump_starts.push(si > 0 && pi == 0);
        }
    }
    if label_points.len() != weights.len() {
        bail!("高对称点数量与 K_POINTS 权重数量不一致");
    }
    let mut indices = vec![1i64];
    for i in 1..weights.len() {
        indices.push(indices[i - 1] + weights[i - 1]);
    }
    let mut total_dist = 0.0f64;
    let mut last: Option<[f64; 3]> = None;
    let path = format!("{workspace}/{prefix}_band.labelinfo.dat");
    let mut s = String::new();
    for i in 0..label_points.len() {
        let (coords, label) = &label_points[i];
        let cart = dot_k(coords, &recip);
        if let Some(prev) = last {
            if !jump_starts[i] {
                total_dist += norm([cart[0] - prev[0], cart[1] - prev[1], cart[2] - prev[2]]);
            }
        }
        s.push_str(&format!(
            "{label:8} {idx:12} {dist:16.10} {x:16.10} {y:16.10} {z:16.10}\n",
            label = label,
            idx = indices[i],
            dist = total_dist,
            x = coords[0],
            y = coords[1],
            z = coords[2]
        ));
        last = Some(cart);
    }
    fs::write(&path, &s)?;
    Ok(path)
}

/// 写出 eband 全部输入：eband.in（pw.x, bands）+ elebanddata.in（&BANDS）+ elebandprojdata.in（&projwfc）
/// + 两个 labelinfo.dat。对齐 `write_eband_inputs` / `write_eband_kpoints` / `write_banddata_inputs`。
pub fn write_eband_inputs(
    workspace: &str,
    config: &Config,
    structure: &Structure,
    prefix: &str,
) -> Result<Vec<String>> {
    let merged = eband_merged(config);
    let mut params = build_nested_pw_params(&merged);
    params.control.insert("calculation".to_string(), json!("bands"));

    let kpath_points = merged
        .get("k_points")
        .and_then(|v| v.get("kpath_points"))
        .and_then(|v| v.as_i64())
        .unwrap_or(100);
    let kpath_file = merged
        .get("k_points")
        .and_then(|v| v.get("kpath_file"))
        .and_then(|v| v.as_str())
        .unwrap_or("KPATH.in")
        .to_string();
    let kpath_path = if Path::new(&kpath_file).is_absolute() {
        kpath_file
    } else {
        format!("{workspace}/{kpath_file}")
    };
    let segments = parse_kpath_segments(&kpath_path)?;

    let mut weights: Vec<i64> = Vec::new();
    let nseg = segments.len();
    for (si, seg) in segments.iter().enumerate() {
        let npt = seg.len();
        for (pi, _kp) in seg.iter().enumerate() {
            let is_last = pi == npt - 1;
            let is_jump = is_last && si < nseg - 1;
            let is_final = is_last && si == nseg - 1;
            weights.push(if is_jump || is_final { 1 } else { kpath_points });
        }
    }

    let kpoints = eband_kpoints_text(&segments, kpath_points);
    let pw_content = write_pw_input_with_precision(config, structure, &params, prefix, &kpoints, 8)?;

    fs::create_dir_all(workspace)?;
    let eband_path = format!("{workspace}/eband.in");
    fs::write(&eband_path, &pw_content)?;

    let gun_path = write_gun_labelinfo(workspace, prefix, &segments, &structure.lattice)?;
    let band_label_path =
        write_indexed_labelinfo(workspace, prefix, &segments, &structure.lattice, &weights)?;

    let outdir = control_outdir(&merged);
    let lsym = merged
        .get("eband")
        .and_then(|v| v.get("lsym"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let lsym_str = if lsym { "true" } else { "false" };
    let band_prefix = format!("{prefix}_band");

    let banddata = format!(
        "&BANDS\n prefix = '{prefix}',\n outdir = '{outdir}',\n filband = '{band_prefix}',\n lp = .true.\n/\n"
    );
    let banddata_path = format!("{workspace}/elebanddata.in");
    fs::write(&banddata_path, &banddata)?;

    let projdata = format!(
        "&projwfc\n prefix = '{prefix}',\n outdir = '{outdir}',\n lsym = .{lsym_str}.,\n filproj = '{band_prefix}'\n/\n"
    );
    let projdata_path = format!("{workspace}/elebandprojdata.in");
    fs::write(&projdata_path, &projdata)?;

    Ok(vec![
        eband_path,
        banddata_path,
        projdata_path,
        gun_path,
        band_label_path,
    ])
}

/// 写出 eledos 全部输入：eletdos.in（&dos）+ elepdos.in（&projwfc）。对齐 `write_eledos_inputs`。
pub fn write_eledos_inputs(
    workspace: &str,
    config: &Config,
    prefix: &str,
) -> Result<Vec<String>> {
    let merged = eledos_merged(config);
    let outdir = eledos_outdir(&merged);
    let eledos = merged.get("eledos").cloned().unwrap_or(Value::Object(Default::default()));
    let qe_val = |key: &str, default: Value| -> Value {
        eledos.get(key).cloned().filter(|v| !v.is_null()).unwrap_or(default)
    };
    let fildos = as_str_opt(Some(&qe_val("fildos", json!(format!("{prefix}.tdos")))))
        .unwrap_or_else(|| format!("{prefix}.tdos"));
    let filpdos = as_str_opt(Some(&qe_val("filpdos", json!(format!("{prefix}.pdos")))))
        .unwrap_or_else(|| format!("{prefix}.pdos"));
    let filproj = as_str_opt(Some(&qe_val("filproj", json!(format!("{prefix}.proj")))))
        .unwrap_or_else(|| format!("{prefix}.proj"));

    let eletdos = format!(
        "&dos\n prefix = {prefix_q},\n outdir = {outdir_q},\n fildos = {fildos_q},\n DeltaE = {deltae},\n emin = {emin},\n emax = {emax}\n/\n",
        prefix_q = to_qe_value(&json!(prefix)),
        outdir_q = to_qe_value(&json!(&outdir)),
        fildos_q = to_qe_value(&json!(&fildos)),
        deltae = to_qe_value(&qe_val("DeltaE", json!(0.01))),
        emin = to_qe_value(&qe_val("emin", json!(-10))),
        emax = to_qe_value(&qe_val("emax", json!(30))),
    );
    let elepdos = format!(
        "&projwfc\n prefix = {prefix_q},\n outdir = {outdir_q},\n filpdos = {filpdos_q},\n filproj = {filproj_q},\n ngauss = {ngauss},\n DeltaE = {deltae},\n degauss = {degauss},\n emin = {emin},\n emax = {emax}\n/\n",
        prefix_q = to_qe_value(&json!(prefix)),
        outdir_q = to_qe_value(&json!(&outdir)),
        filpdos_q = to_qe_value(&json!(&filpdos)),
        filproj_q = to_qe_value(&json!(&filproj)),
        ngauss = to_qe_value(&qe_val("ngauss", json!(0))),
        deltae = to_qe_value(&qe_val("DeltaE", json!(0.01))),
        degauss = to_qe_value(&qe_val("degauss", json!(0.0))),
        emin = to_qe_value(&qe_val("emin", json!(-10))),
        emax = to_qe_value(&qe_val("emax", json!(30))),
    );

    fs::create_dir_all(workspace)?;
    let eletdos_path = format!("{workspace}/eletdos.in");
    fs::write(&eletdos_path, &eletdos)?;
    let elepdos_path = format!("{workspace}/elepdos.in");
    fs::write(&elepdos_path, &elepdos)?;
    Ok(vec![eletdos_path, elepdos_path])
}
