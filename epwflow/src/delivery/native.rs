//! 原生结果包：F4 负责求解与分类，本模块只归集结构化数据并验证相对文件身份。
use super::comparison::Artifact;
use crate::{
    foundation::store::{self, Identity},
    interpolation::manifest::{InterpolationManifest, Spectrum},
    superconductivity::{
        self, engine::ModeResult, gap::Statistics, manifest::SuperconductivityManifest,
    },
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModeSummary {
    pub method: String,
    pub thresholds: crate::foundation::config::Tc,
    pub search: superconductivity::search::Search,
    pub complete: bool,
    pub reason: Option<String>,
    pub attempt_count: usize,
    pub failed_temperatures_centikelvin: Vec<i64>,
}
impl From<&ModeResult> for ModeSummary {
    fn from(r: &ModeResult) -> Self {
        Self {
            method: r.method.clone(),
            thresholds: r.thresholds.clone(),
            search: r.search.clone(),
            complete: r.complete,
            reason: r.reason.clone(),
            attempt_count: r.attempts.len(),
            failed_temperatures_centikelvin: r
                .attempts
                .iter()
                .enumerate()
                .filter(|(i, a)| {
                    a.status != "valid"
                        && !r.attempts[i + 1..].iter().any(|later| {
                            later.request.temperature_centikelvin
                                == a.request.temperature_centikelvin
                                && later.request.kind == a.request.kind
                                && later.request.seed_mev == a.request.seed_mev
                        })
                })
                .map(|(_, a)| a.request.temperature_centikelvin)
                .collect(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GapSummary {
    pub mode: String,
    pub temperature_centikelvin: i64,
    pub seed_mev: f64,
    pub band: usize,
    pub statistics: Statistics,
    pub classification: Option<superconductivity::search::Side>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GapState {
    pub mode: String,
    pub temperature_centikelvin: i64,
    pub seed_mev: f64,
    pub kpoint: usize,
    pub band: usize,
    pub k_crystal: Option<[f64; 3]>,
    pub weight: f64,
    pub energy_ev: f64,
    pub delta_mev: f64,
    pub z: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GapPoint {
    pub mode: String,
    pub temperature_centikelvin: i64,
    pub initial_gap_mev: f64,
    pub execution_status: String,
    pub attempt_directory: Option<PathBuf>,
    pub status: superconductivity::attempt::GapStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GapReport {
    pub task: String,
    pub execution_complete: bool,
    pub data_complete: bool,
    pub points: Vec<GapPoint>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_run: Option<GapReport>,
    pub schema_version: u32,
    pub run_id: String,
    pub complete: bool,
    pub modes: BTreeMap<String, ModeSummary>,
    pub spectra: Vec<Spectrum>,
    pub gaps: Vec<GapSummary>,
    pub states: Vec<GapState>,
    pub solve_parameters: BTreeMap<String, superconductivity::attempt::Request>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub kind: String,
    pub run_id: String,
    pub complete: bool,
    pub files: Vec<Artifact>,
    pub sources: Vec<Identity>,
}
pub fn load(root: &Path) -> Result<(Manifest, Data)> {
    super::publication::ensure_finished(root, "manifest.json")?;
    let m: Manifest = store::read(&root.join("manifest.json"))?;
    ensure!(
        m.schema_version == 1 && m.kind == "epwflow-native-results",
        "unsupported_version: 原生结果包格式"
    );
    let d: Data = store::read(&root.join("summary.json"))?;
    let expected = [
        "summary.json",
        "a2f.csv",
        "coupling.csv",
        "gap_summary.csv",
        "gap_states.csv",
        if d.gap_run.is_some() {
            "points.csv"
        } else {
            "tc.csv"
        },
        "report.md",
    ];
    ensure!(
        m.files.len() == expected.len()
            && expected.iter().all(|n| m
                .files
                .iter()
                .filter(|a| a.relative_path == Path::new(n))
                .count()
                == 1),
        "data_invalid: 原生结果文件集合不完整或重复"
    );
    for a in &m.files {
        a.verify(root)?;
    }
    ensure!(
        d.schema_version == 1
            && d.run_id == m.run_id
            && d.complete == m.complete
            && (!d.modes.is_empty() || d.gap_run.is_some()),
        "data_invalid: 结果身份冲突"
    );
    ensure!(
        d.complete
            == d.gap_run
                .as_ref()
                .map(|r| r.execution_complete)
                .unwrap_or_else(|| d.modes.values().all(|r| r.complete)),
        "data_invalid: 结果整体状态冲突"
    );
    if let Some(r) = &d.gap_run {
        ensure!(
            r.task == "gap"
                && d.modes.is_empty()
                && !r.points.is_empty()
                && r.execution_complete == r.points.iter().all(|p| p.status.execution_complete)
                && r.data_complete == r.points.iter().all(|p| p.status.data_status == "valid"),
            "data_invalid: 单点结果状态冲突"
        );
        for (i, p) in r.points.iter().enumerate() {
            ensure!(
                matches!(p.mode.as_str(), "iso" | "aniso")
                    && p.temperature_centikelvin > 0
                    && !r.points[..i].iter().any(|q| q.mode == p.mode
                        && q.temperature_centikelvin == p.temperature_centikelvin),
                "data_invalid: 单点结果计划无效"
            );
        }
    }
    let contains_point = |mode: &str, t: i64| {
        d.gap_run
            .as_ref()
            .map(|r| {
                r.points.iter().any(|p| {
                    p.mode == mode
                        && p.temperature_centikelvin == t
                        && p.status.data_status == "valid"
                })
            })
            .unwrap_or_else(|| d.modes.contains_key(mode))
    };
    for s in &d.spectra {
        validate_spectrum(s)?;
    }
    for g in &d.gaps {
        ensure!(
            contains_point(&g.mode, g.temperature_centikelvin)
                && g.temperature_centikelvin > 0
                && g.seed_mev > 0.0
                && [
                    g.seed_mev,
                    g.statistics.signed_mean_mev,
                    g.statistics.absolute_mean_mev,
                    g.statistics.rms_mev,
                    g.statistics.min_absolute_mev,
                    g.statistics.max_absolute_mev,
                    g.statistics.weight_sum
                ]
                .iter()
                .all(|x| x.is_finite()),
            "data_invalid: 能隙统计无效"
        );
    }
    for s in &d.states {
        ensure!(
            contains_point(&s.mode, s.temperature_centikelvin)
                && s.temperature_centikelvin > 0
                && s.band > 0
                && s.kpoint > 0
                && [s.seed_mev, s.weight, s.energy_ev, s.delta_mev, s.z]
                    .iter()
                    .all(|x| x.is_finite())
                && s.weight >= 0.0
                && s.z > 0.0
                && s.k_crystal.is_none_or(|k| k.iter().all(|x| x.is_finite())),
            "data_invalid: 能隙状态无效"
        );
    }
    Ok((m, d))
}
fn validate_spectrum(s: &Spectrum) -> Result<()> {
    ensure!(
        s.energy_mev.len() >= 2
            && s.energy_mev.len() == s.a2f.len()
            && s.energy_mev
                .iter()
                .chain(&s.a2f)
                .all(|x| x.is_finite() && *x >= 0.0)
            && s.energy_mev.windows(2).all(|w| w[1] > w[0])
            && s.lambda_integral.is_finite()
            && s.lambda_sum.is_finite(),
        "data_invalid: 原生谱数据无效"
    );
    Ok(())
}
pub(crate) fn cumulative(s: &Spectrum) -> Vec<f64> {
    let mut total = 0.0;
    let mut out = vec![0.0];
    for i in 1..s.a2f.len() {
        let f = |j: usize| {
            if s.energy_mev[j] > 0.0 {
                2.0 * s.a2f[j] / s.energy_mev[j]
            } else {
                0.0
            }
        };
        total += (s.energy_mev[i] - s.energy_mev[i - 1]) * (f(i) + f(i - 1)) / 2.0;
        out.push(total);
    }
    out
}
fn csv(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub fn collect(work: &Path, output: Option<&Path>) -> Result<PathBuf> {
    let f3_path = crate::foundation::layout::path(&work, "interpolation/manifest.json");
    let f4_path = crate::foundation::layout::path(&work, "superconductivity/manifest.json");
    let f3 = InterpolationManifest::load(&f3_path)?;
    if store::read::<serde_json::Value>(&f4_path)?
        .get("task")
        .and_then(|v| v.as_str())
        == Some("gap")
    {
        return collect_gap(work, output, &f3_path, &f4_path, f3);
    }
    let f4 = SuperconductivityManifest::load(&f4_path)?;
    ensure!(
        f4.schema_version == 2
            && f4.run_id == f3.run_id
            && !f3.spectra.is_empty()
            && f4.interpolation_digest == store::hash_file(&f3_path)?,
        "input_conflict: 原生交付的 F3 来源不匹配"
    );
    for s in &f3.spectra {
        validate_spectrum(s)?;
    }
    let mut data = Data {
        gap_run: None,
        schema_version: 1,
        run_id: f4.run_id.clone(),
        complete: f4.converged && f4.validation.all_passed(),
        modes: f4
            .native_modes
            .iter()
            .map(|(n, r)| (n.clone(), r.into()))
            .collect(),
        spectra: f3.spectra,
        gaps: vec![],
        states: vec![],
        solve_parameters: BTreeMap::new(),
    };
    let mut sources = vec![Identity::capture(&f3_path)?, Identity::capture(&f4_path)?];
    for (name, mode) in &f4.native_modes {
        for a in &mode.attempts {
            let Some(id) = &a.record else { continue };
            sources.push(id.clone());
            if a.status != "valid" {
                continue;
            }
            let r = superconductivity::verify::record(&id.path)?;
            sources.push(r.executable.clone());
            sources.extend(r.artifacts.iter().cloned());
            sources.push(r.source.a2f.clone());
            sources.push(r.source.crystal.clone());
            sources.extend(r.source.selecq.iter().cloned());
            sources.extend(r.source.matrix_files.iter().cloned());
            data.solve_parameters
                .entry(name.clone())
                .or_insert_with(|| r.request.clone());
            if let Some(g) = r.gap {
                for (band, statistics) in g.band_statistics()? {
                    data.gaps.push(GapSummary {
                        mode: name.clone(),
                        temperature_centikelvin: g.temperature_centikelvin,
                        seed_mev: r.request.seed_mev.context("data_invalid: 缺少初值")?,
                        band,
                        statistics,
                        classification: mode.search.points.get(&g.temperature_centikelvin).copied(),
                    });
                }
                for (i, s) in g.states.iter().enumerate() {
                    data.states.push(GapState {
                        mode: name.clone(),
                        temperature_centikelvin: g.temperature_centikelvin,
                        seed_mev: r.request.seed_mev.unwrap(),
                        kpoint: s.kpoint,
                        band: s.band,
                        k_crystal: s.k_crystal,
                        weight: s.weight,
                        energy_ev: s.energy_ev,
                        delta_mev: g.delta_mev[i],
                        z: g.z[i],
                    });
                }
            }
        }
    }
    publish(work, output, &f3_path, &f4_path, data, sources)
}

fn publish(
    work: &Path,
    output: Option<&Path>,
    f3_path: &Path,
    f4_path: &Path,
    data: Data,
    mut sources: Vec<Identity>,
) -> Result<PathBuf> {
    sources.sort_by(|a, b| a.path.cmp(&b.path).then(a.sha256.cmp(&b.sha256)));
    sources.dedup_by(|a, b| a.path == b.path && a.sha256 == b.sha256);
    ensure!(
        sources.windows(2).all(|w| w[0].path != w[1].path),
        "input_conflict: 同一来源路径有不同身份"
    );
    // 上游主清单始终排在前两项，便于核对同一运行的部分结果更新。
    sources.retain(|s| s.path != f3_path && s.path != f4_path);
    sources.insert(0, Identity::capture(f4_path)?);
    sources.insert(0, Identity::capture(f3_path)?);
    let output = output
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::foundation::layout::path(work, "results"));
    fs::create_dir_all(&output)?;
    let _lock = store::Lock::acquire(&output.join("native-results"))?;
    super::publication::recover(&output, "manifest.json")?;
    let existing = output.join("manifest.json");
    if existing.exists() {
        let (old, _) = load(&output)?;
        if serde_json::to_value(&old.sources)? == serde_json::to_value(&sources)? {
            return Ok(output);
        }
        // 同一运行的 partial 交付可在原来源更新后重建；不能偷换绑定。
        ensure!(
            old.run_id == data.run_id
                && old
                    .sources
                    .first()
                    .is_some_and(|x| x.sha256 == sources[0].sha256),
            "input_conflict: 结果绑定其他运行，请使用新的 --output"
        );
    }
    let names = [
        "summary.json",
        "a2f.csv",
        "coupling.csv",
        "gap_summary.csv",
        "gap_states.csv",
        if data.gap_run.is_some() {
            "points.csv"
        } else {
            "tc.csv"
        },
        "report.md",
    ];
    let owned = if existing.exists() {
        load(&output)?
            .0
            .files
            .iter()
            .map(|a| a.relative_path.clone())
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    for name in names {
        ensure!(
            !output.join(name).exists() || owned.contains(&PathBuf::from(name)),
            "input_conflict: 未知结果文件冲突 {name}"
        );
    }
    let stage = output.join(format!(".native-{}", store::token()));
    fs::create_dir(&stage)?;
    store::write(&stage.join("summary.json"), &data)?;
    let mut a2f = "smearing_id,energy_mev,a2f,lambda_cumulative\n".to_string();
    let mut coupling = "smearing_id,lambda_integral,lambda_sum,cutoff_ev\n".to_string();
    for s in &data.spectra {
        for ((&e, &a), l) in s.energy_mev.iter().zip(&s.a2f).zip(cumulative(s)) {
            a2f.push_str(&format!("{},{e},{a},{l}\n", csv(&s.smearing_id)));
        }
        coupling.push_str(&format!(
            "{},{},{},{}\n",
            csv(&s.smearing_id),
            s.lambda_integral,
            s.lambda_sum,
            s.cutoff_ev
        ));
    }
    let mut gap="mode,temperature_k,seed_mev,band,definition,signed_mean_mev,absolute_mean_mev,rms_mev,min_absolute_mev,max_absolute_mev,weight_sum,classification\n".to_string();
    for g in &data.gaps {
        let s = &g.statistics;
        gap.push_str(&format!(
            "{},{},{},{},lowest_positive_matsubara,{},{},{},{},{},{},{}\n",
            g.mode,
            g.temperature_centikelvin as f64 / 100.0,
            g.seed_mev,
            g.band,
            s.signed_mean_mev,
            s.absolute_mean_mev,
            s.rms_mev,
            s.min_absolute_mev,
            s.max_absolute_mev,
            s.weight_sum,
            csv(&serde_json::to_string(&g.classification)?)
        ));
    }
    let mut states="mode,temperature_k,seed_mev,band,kpoint,kx_crystal,ky_crystal,kz_crystal,weight,energy_ev,delta_mev,z,definition\n".to_string();
    for s in &data.states {
        let k = s
            .k_crystal
            .map(|v| v.map(|x| x.to_string()))
            .unwrap_or_default();
        states.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},lowest_positive_matsubara\n",
            s.mode,
            s.temperature_centikelvin as f64 / 100.0,
            s.seed_mev,
            s.band,
            s.kpoint,
            k[0],
            k[1],
            k[2],
            s.weight,
            s.energy_ev,
            s.delta_mev,
            s.z
        ));
    }
    let mut tc =
        "mode,method,status,lower_k,upper_k,estimate_k,half_width_k,attempt_count,reason\n"
            .to_string();
    let optional = |x: Option<f64>| x.map(|v| v.to_string()).unwrap_or_default();
    for (name, r) in &data.modes {
        let interval = r.search.interval_k();
        tc.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            name,
            r.method,
            r.search.status,
            optional(interval.map(|x| x[0])),
            optional(interval.map(|x| x[1])),
            optional(r.search.estimate_k()),
            optional(r.search.half_width_k()),
            r.attempt_count,
            csv(r
                .reason
                .as_deref()
                .or(r.search.reason.as_deref())
                .unwrap_or(""))
        ));
    }
    let point_csv = if let Some(r) = &data.gap_run {
        let mut text="mode,temperature_k,initial_gap_mev,execution_status,execution_complete,converged,data_status,log_path,reason\n".to_string();
        for p in &r.points {
            text.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}\n",
                p.mode,
                p.temperature_centikelvin as f64 / 100.0,
                p.initial_gap_mev,
                p.execution_status,
                p.status.execution_complete,
                p.status
                    .converged
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                p.status.data_status,
                csv(&p
                    .attempt_directory
                    .as_ref()
                    .map(|d| d.join("epw.out").to_string_lossy().into_owned())
                    .unwrap_or_default()),
                csv(p.status.reason.as_deref().unwrap_or(""))
            ));
        }
        Some(text)
    } else {
        None
    };
    for (name, text) in [
        ("a2f.csv", a2f),
        ("coupling.csv", coupling),
        ("gap_summary.csv", gap),
        ("gap_states.csv", states),
        if let Some(text) = point_csv {
            ("points.csv", text)
        } else {
            ("tc.csv", tc)
        },
    ] {
        fs::write(stage.join(name), text)?;
    }
    if let Some(gap) = &data.gap_run {
        fs::write(stage.join("report.md"),format!("# 指定温度能隙结果\n\n运行：{}。原生执行完成：{}；能隙数据全部有效：{}。逐点状态见 summary.json。\n\n能隙与 Z 为最低正 Matsubara 频率数据。无效或缺失点不绘制、不补零，不对温度点插值。未进行 Tc 搜索。\n",data.run_id,gap.execution_complete,gap.data_complete))?;
    } else {
        fs::write(stage.join("report.md"),format!("# 原生超导结果\n\n运行：{}。所选模式数值完成：{}。独立科学参考及六组合验收需另行核验。\n\n能隙与 Z 均为最低正 Matsubara 频率数据；完整逐频率数组保留于 F4 原始交付，源身份见 manifest.json。不存在的数值保留为空，不填零。Tc 中点为有效区间估计。小网格结果不表示材料参数收敛。\n\n数据：[汇总](summary.json)、[谱](a2f.csv)、[耦合](coupling.csv)、[能隙统计](gap_summary.csv)、[状态](gap_states.csv)、[Tc](tc.csv)。图像位于 figures。\n",data.run_id,data.complete))?;
    }
    let m = Manifest {
        schema_version: 1,
        kind: "epwflow-native-results".into(),
        run_id: data.run_id.clone(),
        complete: data.complete,
        files: names
            .iter()
            .map(|n| Artifact::capture(&stage, n))
            .collect::<Result<_>>()?,
        sources,
    };
    store::write(&stage.join("manifest.json"), &m)?;
    load(&stage)?;
    let old = if existing.exists() {
        load(&output)?.0.files
    } else {
        vec![]
    };
    super::publication::publish(&output, &stage, "manifest.json", old, m.files.clone(), &m)?;
    Ok(output)
}

pub fn check(root: &Path) -> Result<serde_json::Value> {
    let (m, d) = load(root)?;
    let mut source_verification = "verified";
    for s in &m.sources {
        if s.path.exists() {
            s.verify()?;
        } else {
            source_verification = "unavailable";
        }
    }
    Ok(
        serde_json::json!({"schema_version":1,"operation_status":"ok","native_complete":m.complete,
        "workflow_complete":false,"source_verification":source_verification,"run_id":d.run_id,"modes":d.modes,"gap_run":d.gap_run}),
    )
}

fn collect_gap(
    work: &Path,
    output: Option<&Path>,
    f3_path: &Path,
    f4_path: &Path,
    f3: InterpolationManifest,
) -> Result<PathBuf> {
    let run = superconductivity::points::Run::load(f4_path)?;
    ensure!(
        run.run_id == f3.run_id
            && run.interpolation_digest == store::hash_file(f3_path)?
            && !f3.spectra.is_empty(),
        "input_conflict: 单点结果 F3 来源不匹配"
    );
    let mut report = GapReport {
        task: "gap".into(),
        execution_complete: run.execution_complete,
        data_complete: run.data_complete,
        points: vec![],
    };
    let mut data = Data {
        gap_run: None,
        schema_version: 1,
        run_id: run.run_id.clone(),
        complete: run.execution_complete,
        modes: BTreeMap::new(),
        spectra: f3.spectra,
        gaps: vec![],
        states: vec![],
        solve_parameters: BTreeMap::new(),
    };
    let mut sources = vec![];
    // 保留所有尝试的来源身份；每个点只导出最后一次有效结果。
    for a in &run.attempts {
        if let Some(id) = &a.record {
            sources.push(id.clone());
            let r = superconductivity::verify::record(&id.path)?;
            sources.push(r.executable);
            sources.extend(r.artifacts);
            sources.push(r.source.a2f);
            sources.push(r.source.crystal);
            sources.extend(r.source.selecq);
            sources.extend(r.source.matrix_files);
        }
    }
    for request in &run.plan {
        let name = if request.mode == crate::foundation::config::Mode::Iso {
            "iso"
        } else {
            "aniso"
        }
        .to_string();
        let entry = run.latest(request);
        let record = entry
            .and_then(|a| a.record.as_ref())
            .map(|id| superconductivity::verify::record(&id.path))
            .transpose()?;
        report.points.push(GapPoint {
            mode: name.clone(),
            temperature_centikelvin: request.temperature_centikelvin,
            initial_gap_mev: request.seed_mev.unwrap(),
            execution_status: entry
                .map(|a| {
                    if matches!(a.status.as_str(), "valid" | "completed") {
                        "completed".into()
                    } else {
                        a.status.clone()
                    }
                })
                .unwrap_or("pending".into()),
            attempt_directory: entry.map(|a| a.directory.clone()),
            status: record
                .as_ref()
                .and_then(|r| r.gap_status.clone())
                .unwrap_or(superconductivity::attempt::GapStatus {
                    execution_complete: false,
                    converged: None,
                    data_status: "unavailable".into(),
                    reason: entry.and_then(|a| a.reason.clone()),
                }),
        });
        data.solve_parameters
            .entry(name.clone())
            .or_insert_with(|| request.clone());
        if let Some(g) = record.filter(|r| r.status == "valid").and_then(|r| r.gap) {
            for (band, statistics) in g.band_statistics()? {
                data.gaps.push(GapSummary {
                    mode: name.clone(),
                    temperature_centikelvin: g.temperature_centikelvin,
                    seed_mev: request.seed_mev.unwrap(),
                    band,
                    statistics,
                    classification: None,
                });
            }
            for (i, state) in g.states.iter().enumerate() {
                data.states.push(GapState {
                    mode: name.clone(),
                    temperature_centikelvin: g.temperature_centikelvin,
                    seed_mev: request.seed_mev.unwrap(),
                    kpoint: state.kpoint,
                    band: state.band,
                    k_crystal: state.k_crystal,
                    weight: state.weight,
                    energy_ev: state.energy_ev,
                    delta_mev: g.delta_mev[i],
                    z: g.z[i],
                });
            }
        }
    }
    data.gap_run = Some(report);
    publish(work, output, f3_path, f4_path, data, sources)
}
