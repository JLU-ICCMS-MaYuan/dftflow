//! 共用 Wannier 选窗：显式纯能量模式和历史 AiiDA 组合模式。
//! 来源及许可说明见 docs/specs/0016-wannier-window-selection/upstream-adoption.md。

use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, path::Path};

pub const UPSTREAM_REVISION: &str = "1c0115cb7c77541a0bd549bc05005df435d651b6";
pub const DEFAULT_FROZEN_OFFSET_EV: f64 = 2.0;
pub const DIS_PROJ_MIN: f64 = 0.01;
pub const DIS_PROJ_MAX: f64 = 0.95;
/// 标准 W90 接口显式启用投影冻结；避免只写阈值却仍执行纯能量算法。
pub const PROJECTABILITY_SWITCH: &str = "dis_froz_proj = true";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionMode {
    #[default]
    FixedPlusProjectability,
    EnergyOnly,
}
impl SelectionMode {
    pub fn is_legacy(&self) -> bool {
        *self == Self::FixedPlusProjectability
    }
}

fn kept_bands(
    bands: &[Vec<f64>],
    num_wann: usize,
    exclude: &[usize],
) -> Result<(Vec<Vec<f64>>, Vec<usize>)> {
    let nbnd = validate_bands(bands)?;
    let mut excluded = exclude.to_vec();
    excluded.sort_unstable();
    excluded.dedup();
    ensure!(
        num_wann > 0 && excluded.iter().all(|&b| b > 0 && b <= nbnd),
        "data_invalid: num_wann 或 exclude_bands 无效"
    );
    ensure!(
        nbnd - excluded.len() >= num_wann,
        "data_invalid: 排除后能带少于 num_wann"
    );
    let kept = bands
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .filter(|(i, _)| excluded.binary_search(&(i + 1)).is_err())
                .map(|(_, &e)| e)
                .collect()
        })
        .collect();
    Ok((kept, excluded))
}

/// 金属的显式纯能量起点；所有边界均记录，不裁剪请求或推断稀疏网格带隙。
pub fn select_energy_only(
    bands: &[Vec<f64>],
    fermi_ev: f64,
    num_wann: usize,
    exclude_bands: &[usize],
    overrides: &Bounds,
    reference: Reference,
) -> Result<Selection> {
    ensure!(fermi_ev.is_finite(), "data_invalid: Fermi 能非有限");
    let (kept, excluded) = kept_bands(bands, num_wann, exclude_bands)?;
    let offset = match reference {
        Reference::Absolute => 0.0,
        Reference::Fermi => fermi_ev,
        Reference::UpstreamRelative => {
            anyhow::bail!("configuration: 纯能量模式仅接受绝对或 Fermi 能量参考")
        }
    };
    let homo = kept
        .iter()
        .flatten()
        .copied()
        .filter(|&e| e <= fermi_ev)
        .reduce(f64::max)
        .context("data_invalid: 没有占据态")?;
    let lumo = kept
        .iter()
        .flatten()
        .copied()
        .filter(|&e| e > fermi_ev)
        .reduce(f64::min)
        .context("data_invalid: 没有未占据态")?;
    let bound = |value: Option<f64>, default: f64| -> Result<f64> {
        let result = value.map(|v| v + offset).unwrap_or(default);
        ensure!(result.is_finite(), "data_invalid: 能窗非有限或换算溢出");
        Ok(result)
    };
    let lower = bound(
        overrides.dis_win_min,
        kept.iter().flatten().copied().fold(f64::INFINITY, f64::min) - 0.02,
    )?;
    let upper = bound(
        overrides.dis_win_max,
        kept.iter()
            .flatten()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
            + 0.02,
    )?;
    let frozen_lower = bound(overrides.dis_froz_min, lower)?;
    let frozen_upper = bound(overrides.dis_froz_max, fermi_ev + DEFAULT_FROZEN_OFFSET_EV)?;
    ensure!(
        lower < upper
            && lower <= frozen_lower
            && frozen_lower < frozen_upper
            && frozen_upper <= upper,
        "data_invalid: 纯能量外窗 [{lower},{upper}] 不包含冻结窗 [{frozen_lower},{frozen_upper}]"
    );
    let mut outer_counts = Vec::new();
    let mut energy_frozen_counts = Vec::new();
    for (k, row) in kept.iter().enumerate() {
        let outer = row.iter().filter(|&&e| lower <= e && e <= upper).count();
        let frozen = row
            .iter()
            .filter(|&&e| frozen_lower <= e && e <= frozen_upper)
            .count();
        ensure!(outer >= num_wann && frozen <= num_wann,
            "data_invalid: k={} 外窗 {} 态、冻结 {} 态，要求 frozen<=N={}<=outer；请求冻结窗 [{frozen_lower},{frozen_upper}] 不自动裁剪", k + 1, outer, frozen, num_wann);
        outer_counts.push(outer);
        energy_frozen_counts.push(frozen);
    }
    Ok(Selection {
        upstream_revision: "dftflow-energy-only-v1".into(),
        mode: "energy_only".into(),
        bands_sha256: format!("{:x}", Sha256::digest(serde_json::to_vec(bands)?)),
        fermi_ev,
        homo_ev: homo,
        lumo_ev: lumo,
        shift_ev: fermi_ev,
        requested_frozen_max_ev: frozen_upper,
        // 历史记录兼容字段：纯能量模式不施加 cap，保留请求值。
        frozen_cap_ev: frozen_upper,
        num_wann,
        excluded_bands: excluded,
        parameters: [
            ("dis_win_min".into(), lower),
            ("dis_win_max".into(), upper),
            ("dis_froz_min".into(), frozen_lower),
            ("dis_froz_max".into(), frozen_upper),
        ]
        .into(),
        outer_counts,
        energy_frozen_counts,
    })
}

/// 未设置的边界保留为 None，由原生程序决定实际默认范围。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub dis_win_min: Option<f64>,
    pub dis_win_max: Option<f64>,
    pub dis_froz_min: Option<f64>,
    pub dis_froz_max: Option<f64>,
}

/// 上游覆盖值相对运行时带边；另两种用于适配已有显式能量参考。
#[derive(Debug, Clone, Copy)]
pub enum Reference {
    UpstreamRelative,
    Absolute,
    Fermi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub upstream_revision: String,
    pub mode: String,
    pub bands_sha256: String,
    pub fermi_ev: f64,
    pub homo_ev: f64,
    pub lumo_ev: f64,
    pub shift_ev: f64,
    pub requested_frozen_max_ev: f64,
    pub frozen_cap_ev: f64,
    pub num_wann: usize,
    pub excluded_bands: Vec<usize>,
    pub parameters: BTreeMap<String, f64>,
    pub outer_counts: Vec<usize>,
    /// 这里只统计能量冻结；投影冻结由原生程序使用真实 AMN 计算。
    pub energy_frozen_counts: Vec<usize>,
}

/// 默认 FIXED_PLUS_PROJECTABILITY；未运行科学程序，也不认证插值质量。
pub fn select_default(
    bands: &[Vec<f64>],
    fermi_ev: f64,
    num_wann: usize,
    exclude_bands: &[usize],
    overrides: &Bounds,
    reference: Reference,
) -> Result<Selection> {
    ensure!(fermi_ev.is_finite(), "data_invalid: Fermi 能非有限");
    let (kept, excluded) = kept_bands(bands, num_wann, exclude_bands)?;
    let homo = bands
        .iter()
        .flatten()
        .copied()
        .filter(|&e| e <= fermi_ev)
        .reduce(f64::max)
        .context("data_invalid: 没有 HOMO，全部能带未占据")?;
    let lumo = bands
        .iter()
        .flatten()
        .copied()
        .filter(|&e| e > fermi_ev)
        .reduce(f64::min)
        .context("data_invalid: 没有 LUMO，全部能带已占据")?;
    let shift = if lumo - homo > 1e-3 { lumo } else { fermi_ev };
    let offset = match reference {
        Reference::UpstreamRelative => shift,
        Reference::Absolute => 0.0,
        Reference::Fermi => fermi_ev,
    };
    let mut parameters = BTreeMap::new();
    for (name, value) in [
        ("dis_win_min", overrides.dis_win_min),
        ("dis_win_max", overrides.dis_win_max),
        ("dis_froz_min", overrides.dis_froz_min),
        ("dis_froz_max", overrides.dis_froz_max),
    ] {
        if let Some(value) = value {
            ensure!(
                value.is_finite() && (value + offset).is_finite(),
                "data_invalid: {name} 非有限或换算溢出"
            );
            parameters.insert(name.to_string(), value + offset);
        }
    }
    // 默认上界总是按上游带边平移；显式覆盖值才按配置的参考转换。
    let requested = parameters
        .get("dis_froz_max")
        .copied()
        .unwrap_or(shift + DEFAULT_FROZEN_OFFSET_EV);
    ensure!(requested.is_finite(), "data_invalid: 默认冻结上界溢出");
    let mut cap = kept
        .iter()
        .map(|row| row[num_wann - 1])
        .fold(f64::NEG_INFINITY, f64::max);
    if kept[0].len() > num_wann {
        let next = kept
            .iter()
            .map(|row| row[num_wann])
            .fold(f64::INFINITY, f64::min)
            - 1e-4;
        cap = cap.min(next);
    }
    parameters.insert("dis_froz_max".into(), requested.min(cap));
    parameters.insert("dis_proj_min".into(), DIS_PROJ_MIN);
    parameters.insert("dis_proj_max".into(), DIS_PROJ_MAX);
    let lower = parameters
        .get("dis_win_min")
        .copied()
        .unwrap_or(f64::NEG_INFINITY);
    let upper = parameters
        .get("dis_win_max")
        .copied()
        .unwrap_or(f64::INFINITY);
    let frozen_lower = parameters.get("dis_froz_min").copied().unwrap_or(lower);
    let frozen_upper = parameters["dis_froz_max"];
    ensure!(
        lower <= frozen_lower
            && frozen_lower <= frozen_upper
            && frozen_upper <= upper
            && lower < upper,
        "data_invalid: 上游限制后的能窗不包含或冻结窗倒置，不能静默放宽上界"
    );
    let mut outer_counts = Vec::with_capacity(kept.len());
    let mut energy_frozen_counts = Vec::with_capacity(kept.len());
    for (k, row) in kept.iter().enumerate() {
        let outer = row.iter().filter(|&&e| lower <= e && e <= upper).count();
        let frozen = row
            .iter()
            .filter(|&&e| frozen_lower <= e && e <= frozen_upper)
            .count();
        ensure!(
            outer >= num_wann,
            "data_invalid: k={} 外窗仅 {} 态，少于 {}",
            k + 1,
            outer,
            num_wann
        );
        ensure!(
            frozen <= num_wann,
            "data_invalid: k={} 冻结 {} 态，超过 {}",
            k + 1,
            frozen,
            num_wann
        );
        outer_counts.push(outer);
        energy_frozen_counts.push(frozen);
    }
    Ok(Selection {
        upstream_revision: UPSTREAM_REVISION.into(),
        mode: "fixed_plus_projectability".into(),
        bands_sha256: format!("{:x}", Sha256::digest(serde_json::to_vec(bands)?)),
        fermi_ev,
        homo_ev: homo,
        lumo_ev: lumo,
        shift_ev: shift,
        requested_frozen_max_ev: requested,
        frozen_cap_ev: cap,
        num_wann,
        excluded_bands: excluded,
        parameters,
        outer_counts,
        energy_frozen_counts,
    })
}

pub fn validate_bands(bands: &[Vec<f64>]) -> Result<usize> {
    let nbnd = bands.first().context("data_invalid: 本征值为空")?.len();
    ensure!(
        nbnd > 0
            && bands
                .iter()
                .all(|r| r.len() == nbnd && r.iter().all(|v| v.is_finite())),
        "data_invalid: 本征值不完整、形状不一致或含非有限值"
    );
    ensure!(
        bands.iter().all(|r| r.windows(2).all(|v| v[0] <= v[1])),
        "data_invalid: 本征值必须按带能量升序排列"
    );
    Ok(nbnd)
}

/// 读取实际 QE NSCF 的单通道能谱，严格限制到 output/band_structure。
pub fn read_qe_bands(path: &Path) -> Result<(Vec<Vec<f64>>, f64)> {
    let text = std::fs::read_to_string(path).with_context(|| format!("读取 {}", path.display()))?;
    let doc = roxmltree::Document::parse(&text)?;
    let output = doc
        .descendants()
        .find(|n| n.has_tag_name("output"))
        .context("data_invalid: 缺少 QE output")?;
    let band = output
        .children()
        .find(|n| n.has_tag_name("band_structure"))
        .context("data_invalid: 缺少 QE band_structure")?;
    let field = |name: &str| -> Result<&str> {
        band.children()
            .find(|n| n.has_tag_name(name))
            .and_then(|n| n.text())
            .context("data_invalid: QE 能谱字段缺失")
    };
    let number = |s: &str| -> Result<f64> { Ok(s.trim().replace(['d', 'D'], "e").parse()?) };
    for name in ["lsda", "noncolin", "spinorbit"] {
        if let Ok(value) = field(name) {
            ensure!(
                !matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "true" | ".true." | "t"
                ),
                "unsupported_version: 当前选窗 XML 适配仅支持非磁性单通道"
            );
        }
    }
    let nbnd: usize = field("nbnd")?.trim().parse()?;
    let nks: usize = field("nks")?.trim().parse()?;
    const HA_EV: f64 = 27.211386245988;
    let fermi = number(field("fermi_energy")?)? * HA_EV;
    let mut bands = Vec::new();
    for ks in band.children().filter(|n| n.has_tag_name("ks_energies")) {
        let en = ks
            .children()
            .find(|n| n.has_tag_name("eigenvalues"))
            .context("data_invalid: 缺本征值")?;
        let values: Vec<f64> = en
            .text()
            .context("data_invalid: 本征值为空")?
            .split_whitespace()
            .map(|s| number(s).map(|v| v * HA_EV))
            .collect::<Result<_>>()?;
        ensure!(values.len() == nbnd, "data_invalid: QE 带数不一致");
        if let Some(size) = en.attribute("size") {
            ensure!(
                size.parse::<usize>()? == nbnd,
                "data_invalid: QE size 不一致"
            );
        }
        bands.push(values);
    }
    ensure!(
        bands.len() == nks && fermi.is_finite(),
        "data_invalid: QE 点数或 Fermi 能无效"
    );
    validate_bands(&bands)?;
    Ok((bands, fermi))
}

/// 保守的执行前能力门：检查实际二进制关键字，不依据版本字符串猜测。
/// 关键字存在仅是必要条件；原生执行失败仍须传播，不能退回旧选窗。
pub fn require_projectability_keywords(program: &Path) -> Result<()> {
    let resolved = if program.components().count() == 1 {
        std::env::var_os("PATH")
            .into_iter()
            .flat_map(|v| std::env::split_paths(&v).collect::<Vec<_>>())
            .map(|p| p.join(program))
            .find(|p| p.is_file())
            .context("unsupported_version: 找不到实际 Wannier 程序")?
    } else {
        program.to_path_buf()
    };
    let mut file = std::fs::File::open(&resolved)
        .with_context(|| format!("无法检查 {}", resolved.display()))?;
    let keys = [
        b"dis_proj_min".as_slice(),
        b"dis_proj_max".as_slice(),
        b"dis_froz_proj".as_slice(),
    ];
    let mut found = [false; 3];
    let mut tail = Vec::new();
    let mut buf = [0u8; 65536];
    loop {
        let size = file.read(&mut buf)?;
        if size == 0 {
            break;
        }
        tail.extend_from_slice(&buf[..size]);
        tail.make_ascii_lowercase();
        for (i, key) in keys.iter().enumerate() {
            found[i] |= tail.windows(key.len()).any(|w| w == *key);
        }
        if found.iter().all(|v| *v) {
            return Ok(());
        }
        let start = tail.len().saturating_sub(16);
        tail.drain(..start);
    }
    anyhow::bail!("unsupported_version: {} 无法核验支持 dis_proj_min/dis_proj_max/dis_froz_proj；默认组合选窗禁止退回旧方案，请配置支持投影解纠缠的实际程序", resolved.display())
}

pub fn write_selection(path: &Path, selection: &Selection) -> Result<()> {
    std::fs::write(path, serde_json::to_vec_pretty(selection)?)?;
    Ok(())
}
