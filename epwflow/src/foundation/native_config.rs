//! 版本 2 的外部配置；运行时类型只适配既有科学算法，不是另一种配置格式。
use super::config::{self, RuntimeConfig};
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

#[path = "native_config_flat.rs"]
mod flat;

pub const SCF: &str = "1.qe-scf";
pub const NSCF: &str = "2.qe-nscf";
pub const ELECTRONIC: &str = "3.qe-electronic";
pub const WANNIER: &str = "5.epw-wannier";
pub const COARSE: &str = "6.epw-coarse";
pub const FINE: &str = "7.epw-fine";
pub const SUPER: &str = "8.epw-superconductivity";

pub(crate) fn configuration_error(error: anyhow::Error) -> anyhow::Error {
    anyhow::anyhow!(flat::message(&format!("{error:#}")))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    #[serde(default)]
    pub plot: config::PlotConfig,
    // 仅记录外部展示方式，不参与科学参数、序列化或阶段身份。
    #[serde(skip)]
    pub(crate) eight_stages: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub baseparams: BTreeMap<String, Value>,
    // 外部表达的来源与数组写法仅用于诊断和往返，不进入科学阶段身份。
    #[serde(skip)]
    pub(crate) input_locations: BTreeMap<String, String>,
    #[serde(skip)]
    pub(crate) input_arrays: BTreeMap<String, Value>,
    pub schema_version: u32,
    pub work_dir: PathBuf,
    pub outdir: PathBuf,
    pub structure: PathBuf,
    pub pseudos: BTreeMap<String, PathBuf>,
    pub programs: config::Programs,
    #[serde(default)]
    pub slurm: qf_config::SlurmSection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kpath: Option<PathBuf>,
    pub stages: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub workflow: BTreeMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) runtime: RuntimeConfig,
    document: Document,
}
impl Deref for Config {
    type Target = RuntimeConfig;
    fn deref(&self) -> &RuntimeConfig {
        &self.runtime
    }
}
impl DerefMut for Config {
    fn deref_mut(&mut self) -> &mut RuntimeConfig {
        &mut self.runtime
    }
}
impl<'de> Deserialize<'de> for Config {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let document = Document::deserialize(d)?;
        ensure_version(document.schema_version).map_err(serde::de::Error::custom)?;
        let document = flat::expand(document).map_err(serde::de::Error::custom)?;
        document.plot.validate().map_err(serde::de::Error::custom)?;
        let runtime = decode(&document).map_err(|e| serde::de::Error::custom(flat::document_message(&format!("{e:#}"), &document)))?;
        Ok(Self { runtime, document })
    }
}
impl Serialize for Config {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        self.document().and_then(flat::flatten)
            .map_err(serde::ser::Error::custom)?
            .serialize(s)
    }
}
fn ensure_version(version: u32) -> Result<()> {
    ensure!(
        version == 2,
        "unsupported_version: only schema_version = 2 is supported; legacy configuration is incompatible"
    );
    Ok(())
}
fn at<'a>(v: &'a Value, path: &[&str]) -> Result<&'a Value> {
    let mut node = v;
    for name in path {
        node = node
            .get(*name)
            .with_context(|| format!("configuration: stages.{}: required parameter missing", path.join(".")))?;
    }
    Ok(node)
}
fn get<T: serde::de::DeserializeOwned>(v: &Value, path: &[&str]) -> Result<T> {
    serde_json::from_value(at(v, path)?.clone())
        .with_context(|| format!("configuration: stages.{}: parameter type error", path.join(".")))
}
fn optional<T: serde::de::DeserializeOwned>(v: &Value, path: &[&str]) -> Result<Option<T>> {
    let mut node = v;
    for name in path {
        let Some(next) = node.get(*name) else {
            return Ok(None);
        };
        node = next;
    }
    Ok(Some(serde_json::from_value(node.clone()).with_context(
        || format!("configuration: stages.{}: parameter type error", path.join(".")),
    )?))
}
fn put(v: &mut Value, path: &[&str], value: Value) {
    let (last, parents) = path.split_last().unwrap();
    let mut node = v;
    for name in parents {
        if !node.get(*name).is_some_and(Value::is_object) {
            node[*name] = json!({});
        }
        node = &mut node[*name];
    }
    if value.is_null() {
        if let Some(o) = node.as_object_mut() {
            o.remove(*last);
        }
    } else {
        node[*last] = value;
    }
}
fn stages(d: &Document) -> Value {
    serde_json::to_value(&d.stages).expect("Stage configuration validated by serde")
}
fn ph_name(d: &Document) -> Result<String> {
    let names: Vec<_> = d
        .stages
        .keys()
        .filter(|s| s.starts_with("4.qe-phonon-"))
        .cloned()
        .collect();
    ensure!(
        names.len() == 1,
        "configuration: stages: exactly one 4.qe-phonon-nosplit/index/coords stage is required"
    );
    ensure!(
        [
            "4.qe-phonon-nosplit",
            "4.qe-phonon-index",
            "4.qe-phonon-coords"
        ]
        .contains(&names[0].as_str()),
        "configuration: stages.{}: unknown phonon mode",
        names[0]
    );
    Ok(names[0].clone())
}
fn pw_grid(v: &Value, stage: &str) -> Result<([u32; 3], [u32; 3])> {
    if stage == NSCF {
        let grid: [u32; 3] = get(v, &[stage, "pw", "K_POINTS"])
            .context("configuration: stages.2.qe-nscf.pw.K_POINTS: requires three positive integers [A, B, C]; complete uniform EPW NSCF grid requires zero shift; explicit point lists are no longer accepted")?;
        qf_input::pw::uniform_kpoint_count(grid.map(i64::from))
            .context("configuration: stages.2.qe-nscf.pw.K_POINTS: invalid grid")?;
        return Ok((grid, [0; 3]));
    }

    // 尝试解析为数组格式：[nk1, nk2, nk3] 或 [nk1, nk2, nk3, s1, s2, s3]
    if let Ok(arr) = get::<Vec<u32>>(v, &[stage, "pw", "K_POINTS"]) {
        if arr.len() == 3 {
            return Ok(([arr[0], arr[1], arr[2]], [0, 0, 0]));
        } else if arr.len() == 6 {
            return Ok((
                [arr[0], arr[1], arr[2]],
                [arr[3], arr[4], arr[5]]
            ));
        } else {
            bail!("configuration: stages.{stage}.pw.K_POINTS: array format requires 3 elements [nk1, nk2, nk3] or 6 elements [nk1, nk2, nk3, s1, s2, s3]");
        }
    }

    // 回退到旧的字符串格式
    let text: String = get(v, &[stage, "pw", "K_POINTS"])?;
    let mut rows = text.lines().filter(|row| !row.trim().is_empty());
    let kind = rows
        .next()
        .context("configuration: K_POINTS card is empty")?
        .trim();
    if kind == "automatic" {
        ensure!(stage!=NSCF,"configuration: stages.{NSCF}.pw.K_POINTS: explicit complete uniform crystal grid required; symmetry-reduced automatic grids are unsupported");
        let nums = rows
            .flat_map(str::split_whitespace)
            .map(str::parse::<u32>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(
            nums.len() == 6,
            "configuration: stages.{stage}.pw.K_POINTS: automatic requires six integers"
        );
        return Ok(([nums[0], nums[1], nums[2]], [nums[3], nums[4], nums[5]]));
    }
    ensure!(
        kind == "crystal",
        "configuration: stages.{stage}.pw.K_POINTS: supports automatic (SCF) or a complete crystal grid"
    );
    let count: usize = rows
        .next()
        .context("configuration: K_POINTS point count missing")?
        .trim()
        .parse()?;
    let points = rows
        .map(|row| {
            row.split_whitespace()
                .map(str::parse::<f64>)
                .collect::<std::result::Result<Vec<_>, _>>()
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        count > 0
            && points.len() == count
            && points
                .iter()
                .all(|p| p.len() == 4 && p.iter().all(|x| x.is_finite()) && p[3] > 0.),
        "configuration: stages.{stage}.pw.K_POINTS: invalid count/coordinates/weights"
    );
    let mut axes: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
    for p in &points {
        for i in 0..3 {
            let x = p[i].rem_euclid(1.);
            if !axes[i].iter().any(|v| (v - x).abs() < 1e-9) {
                axes[i].push(x);
            }
        }
    }
    let dims = axes.map(|mut a| {
        a.sort_by(f64::total_cmp);
        a
    });
    let grid = std::array::from_fn(|i| dims[i].len() as u32);
    ensure!(
        grid.iter().map(|n| *n as usize).product::<usize>() == count,
        "configuration: stages.{stage}.pw.K_POINTS: not a complete Cartesian-product grid"
    );
    for axis in &dims {
        for (i, x) in axis.iter().enumerate() {
            ensure!(
                (*x - i as f64 / axis.len() as f64).abs() <= 1e-8,
                "configuration: stages.{stage}.pw.K_POINTS: uniform zero-shift grid required"
            );
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for p in points {
        let index: [i64; 3] =
            std::array::from_fn(|i| (p[i].rem_euclid(1.) * grid[i] as f64).round() as i64);
        ensure!(
            seen.insert(index),
            "configuration: stages.{stage}.pw.K_POINTS: duplicate point"
        );
    }
    Ok((grid, [0; 3]))
}

fn grid(v: &Value, path: &[&str], prefix: &str) -> Result<[u32; 3]> {
    let mut r = [0; 3];
    for (i, x) in r.iter_mut().enumerate() {
        let name = format!("{prefix}{}", i + 1);
        let mut p = path.to_vec();
        p.push(&name);
        *x = get(v, &p)?;
    }
    Ok(r)
}
fn solver(
    v: &Value,
    mode: &str,
    task: config::SuperconductivityTask,
) -> Result<config::Superconductivity> {
    let p = at(v, &[SUPER, mode, "epw", "INPUTEPW"])?;
    let workflow = at(v, &[SUPER, "workflow"])?;
    Ok(config::Superconductivity {
        task,
        mode: get(workflow, &["mode"])?,
        mu_star: get(p, &["muc"])?,
        frequency_cutoff_ev: get(p, &["wscut"])?,
        max_iterations: get(p, &["nsiter"])?,
        convergence_relative: get(p, &["conv_thr_iaxis"])?,
        mixing_beta: get(p, &["broyden_beta"])?,
        mixing_history: get(p, &["broyden_ndim"])?,
        initial_shape: get(p, &["a_gap0"])?,
        initial_gap_mev: if task == config::SuperconductivityTask::Gap {
            Some(get::<f64>(p, &["gap_edge"])? * 1000.)
        } else {
            None
        },
        initial_gaps_mev: optional::<[f64; 2]>(workflow, &["initial_gaps"])?
            .map(|v| v.map(|x| x * 1000.))
            .unwrap_or([0.; 2]),
        gap_temperatures_k: get(p, &["temps"])?,
        spectrum_index: get(workflow, &["spectrum_index"])?,
        iso: optional(workflow, &["iso"])?,
        aniso: optional(workflow, &["aniso"])?,
    })
}

/// 已由原生格式生成器产生的数据与文件路径，只能通过已登记的推导规则进入输入。
pub fn apply(v: &Value, task: &[&str], input: &str) -> Result<String> {
    apply_impl(v, task, input).map_err(configuration_error)
}
fn apply_impl(v: &Value, task: &[&str], input: &str) -> Result<String> {
    let clean = input
        .lines()
        .filter(|line| !line.starts_with("! Parameter source ") && !line.starts_with(super::legacy_language::INPUT_SOURCE_PREFIX))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let input = clean.as_str();
    let call = at(v, task)?
        .as_object()
        .context("configuration: native program configuration must be a table")?;
    let stage = task[0];
    let derived = at(v, &[stage, "workflow", "derived"])?
        .as_object()
        .context("configuration: workflow.derived must be a table")?;
    let task_name = task[1..].join(".");
    let assignment = regex::Regex::new(r"(?i)([a-z_][a-z_0-9]*(?:\s*\(\s*\d+\s*\))?)\s*=")?;
    let sections = regex::Regex::new(r"(?im)^\s*&([a-z_][a-z_0-9]*)\b")?;
    let mut output = String::new();
    let mut end = 0;
    for section in sections.captures_iter(input) {
        let start = section.get(0).unwrap();
        if start.start() < end {
            continue;
        }
        let name = section[1].to_uppercase();
        let mut quote = None;
        let mut stop = None;
        let mut mask = vec![true; input.len() - start.end()];
        for (i, c) in input[start.end()..].char_indices() {
            if let Some(q) = quote {
                mask[i] = false;
                if c == q {
                    quote = None;
                }
            } else if c == '\'' || c == '"' {
                quote = Some(c);
                mask[i] = false;
            } else if c == '/' {
                stop = Some(start.end() + i);
                break;
            }
        }
        let stop = stop.with_context(|| format!("data_invalid: {name} native input terminator missing"))?;
        let body = &input[start.end()..stop];
        let found: Vec<_> = assignment
            .captures_iter(body)
            .filter(|c| mask[c.get(0).unwrap().start()])
            .collect();
        let mut original = BTreeMap::new();
        for (i, c) in found.iter().enumerate() {
            let key = c[1].replace(' ', "");
            let finish = found
                .get(i + 1)
                .map_or(body.len(), |n| n.get(0).unwrap().start());
            let value = body[c.get(0).unwrap().end()..finish]
                .trim()
                .trim_end_matches(',')
                .trim()
                .to_string();
            ensure!(
                original.insert(key.to_lowercase(), value).is_none(),
                "data_invalid: {name} duplicate native parameter {}",
                &c[1]
            );
        }
        let settings = call
            .get(&name)
            .and_then(Value::as_object)
            .with_context(|| {
                format!(
                    "configuration: stages.{}.{}: native input section missing",
                    task.join("."),
                    name
                )
            })?;
        let mut result = BTreeMap::new();
        for (key, value) in settings {
            ensure!(
                key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "configuration: stages.{}.{name}.{key}: invalid native parameter name",
                task.join(".")
            );
            let location = format!("{task_name}.{name}.{key}");
            ensure!(
                !derived.contains_key(&location),
                "configuration: stages.{stage}.{location}: both explicit and workflow.derived definitions provided"
            );
            if let Some(values) = value.as_array() {
                for (n, value) in values.iter().enumerate() {
                    result.insert(format!("{}({})", key, n + 1), qf_input::to_qe_value(value));
                }
            } else {
                result.insert(key.to_lowercase(), qf_input::to_qe_value(value));
            }
        }
        for (key, value) in original {
            let base = key.split('(').next().unwrap();
            let location = format!("{task_name}.{name}.{base}");
            if derived.contains_key(&location) {
                ensure!(
                    !settings.keys().any(|k| k.eq_ignore_ascii_case(base)),
                    "configuration: stages.{stage}.{location}: duplicate derived and native values"
                );
                result.insert(key.clone(), value);
            } else {
                ensure!(result.contains_key(&key) || (key == base && settings.contains_key(base)),"configuration: stages.{stage}.{location}: actual input contains scientific parameters neither configured nor registered as derived");
            }
        }
        output.push_str(&input[end..start.start()]);
        output.push_str(&format!("&{}\n", &section[1]));
        let mut parameters = result.into_iter().collect::<Vec<_>>();
        // 原生数组按数字下标输出，保持卡片/路径等数组正文的阅读顺序。
        let order = |name: &str| -> (String, usize) {
            let (base, index) = name.split_once('(').map_or((name, 0), |(base, index)| {
                (base, index.trim_end_matches(')').parse().unwrap_or(0))
            });
            (base.to_owned(), index)
        };
        parameters.sort_by_key(|(name, _)| order(name));
        for (key, value) in parameters {
            output.push_str(&format!(" {key} = {value}\n"));
        }
        output.push_str("/\n");
        end = stop + 1;
        if input.as_bytes().get(end) == Some(&b'\n') {
            end += 1;
        }
    }
    ensure!(end > 0, "data_invalid: native input contains no namelist");
    output.push_str(&input[end..]);
    Ok(output)
}

impl Config {
    pub(crate) fn config_error(&self, error: anyhow::Error) -> anyhow::Error {
        anyhow::anyhow!(flat::document_message(&format!("{error:#}"), &self.document))
    }

    /// 导出八主阶段表达，不修改运行参数、旧配置版本或计算现场。
    pub fn to_stage_toml(&self) -> Result<String> {
        let mut document = self.document()?;
        document.eight_stages = true;
        Ok(toml::to_string_pretty(&flat::flatten(document)?)?)
    }

    /// 仅供显式导入升级；普通配置入口继续拒绝嵌套格式。
    pub(crate) fn load_nested(path: &std::path::Path) -> Result<Self> {
        let path = super::paths::absolute(&std::env::current_dir()?, path)?;
        let document: Document = toml::from_str(&std::fs::read_to_string(&path)?)?;
        ensure_version(document.schema_version)?;
        ensure!(document.stages.get(SCF).is_some_and(|v| v.get("pw").is_some()),
            "configuration: controlled upgrade accepts only legacy nested version 2");
        document.plot.validate()?;
        let runtime = decode(&document)?;
        let mut cfg = Self { runtime, document };
        cfg.validate()?;
        cfg.resolve(&super::config_versions::base(&path)?)?;
        Ok(cfg)
    }

    pub fn apply_native(&self, task: &[&str], input: &str) -> Result<String> {
        apply(&self.stage_document()?, task, input).map_err(|e| self.config_error(e))
    }
    pub fn configure_file(&self, task: &[&str], path: &std::path::Path) -> Result<()> {
        let input = std::fs::read_to_string(path)?;
        let text = self.apply_native(task, &input)?;
        std::fs::write(path, &text)?;
        Ok(())
    }
}
fn decode(d: &Document) -> Result<RuntimeConfig> {
    let v = stages(d);
    let ph = ph_name(d)?;
    let (scf_grid, shift) = pw_grid(&v, SCF)?;
    let (nscf_grid, nscf_shift) = pw_grid(&v, NSCF)?;
    ensure!(
        nscf_shift == [0; 3],
        "configuration: stages.{NSCF}.pw.K_POINTS: complete uniform EPW NSCF grid requires zero shift"
    );
    let species: String = get(&v, &[SCF, "pw", "ATOMIC_SPECIES"])?;
    let mut masses = BTreeMap::new();
    for row in species.lines().filter(|s| !s.trim().is_empty()) {
        let p: Vec<_> = row.split_whitespace().collect();
        ensure!(
            p.len() == 3,
            "configuration: stages.{SCF}.pw.ATOMIC_SPECIES: each row requires element, mass and pseudopotential file"
        );
        ensure!(
            masses
                .insert(p[0].to_owned(), p[1].parse::<f64>()?)
                .is_none(),
            "configuration: stages.{SCF}.pw.ATOMIC_SPECIES: duplicate element"
        );
    }
    let bounds = |a: &str, b: &str| -> Result<Option<[f64; 2]>> {
        match (
            optional::<f64>(&v, &[WANNIER, "wannier90", a])?,
            optional::<f64>(&v, &[WANNIER, "wannier90", b])?,
        ) {
            (None, None) => Ok(None),
            (Some(x), Some(y)) => Ok(Some([x, y])),
            _ => bail!("configuration: stages.{WANNIER}.wannier90.{a}/{b}: specify both values together"),
        }
    };
    let fine = at(&v, &[FINE, "epw", "INPUTEPW"])?;
    let fine_wf = at(&v, &[FINE, "workflow"])?;
    let task = get(&v, &[SUPER, "workflow", "task"])?;
    let mode: config::Mode = get(&v, &[SUPER, "workflow", "mode"])?;
    let sc = solver(
        &v,
        if mode == config::Mode::Aniso {
            "aniso"
        } else {
            "iso"
        },
        task,
    )?;
    let comparison = config::Comparison {
        points_per_edge: get(&v, &[ELECTRONIC, "workflow", "points_per_edge"])?,
        electronic_sigma_ev: get(
            &v,
            &[WANNIER, "electron_dos", "postw90", "dos_smr_fixed_en_width"],
        )?,
        phonon_sigma_cm1: get(&v, &[&ph, "workflow", "phonon_sigma_cm1"])?,
    };
    Ok(RuntimeConfig {
        plot: d.plot.clone(),
        schema_version: 2,
        work_dir: d.work_dir.clone(),
        outdir: d.outdir.clone(),
        structure: d.structure.clone(),
        pseudos: d.pseudos.clone(),
        programs: d.programs.clone(),
        slurm: d.slurm.clone(),
        layout: d.layout.clone(),
        kpath: d.kpath.clone(),
        atomic_masses: masses,
        comparison: Some(comparison),
        qe: config::Qe {
            prefix: get(&v, &[SCF, "pw", "CONTROL", "prefix"])?,
            ecutwfc_ry: get(&v, &[SCF, "pw", "SYSTEM", "ecutwfc"])?,
            ecutrho_ry: get(&v, &[SCF, "pw", "SYSTEM", "ecutrho"])?,
            occupations: get(&v, &[SCF, "pw", "SYSTEM", "occupations"])?,
            smearing: optional(&v, &[SCF, "pw", "SYSTEM", "smearing"])?,
            degauss_ry: optional(&v, &[SCF, "pw", "SYSTEM", "degauss"])?,
            scf: config::Scf {
                k_grid: scf_grid,
                k_shift: shift,
                conv_thr_ry: get(&v, &[SCF, "pw", "ELECTRONS", "conv_thr"])?,
            },
            nscf: config::Nscf {
                k_grid: nscf_grid,
                nbnd: get(&v, &[NSCF, "pw", "SYSTEM", "nbnd"])?,
                conv_thr_ry: get(&v, &[NSCF, "pw", "ELECTRONS", "conv_thr"])?,
            },
        },
        phonon: config::Phonon {
            mode: serde_json::from_value(json!(ph.rsplit('-').next().unwrap()))?,
            q_grid: grid(&v, &[&ph, "ph", "INPUTPH"], "nq")?,
            tr2_ph: get(&v, &[&ph, "ph", "INPUTPH", "tr2_ph"])?,
        },
        wannier: config::Wannier {
            selection_mode: get(&v, &[WANNIER, "workflow", "selection_mode"])?,
            guiding_centres: optional(&v, &[WANNIER, "wannier90", "guiding_centres"])?,
            num_wann: get(&v, &[WANNIER, "wannier90", "num_wann"])?,
            projections: get(&v, &[WANNIER, "wannier90", "projections"])?,
            exclude_bands: get(&v, &[WANNIER, "wannier90", "exclude_bands"])?,
            energy_reference: config::EnergyReference::QeZero,
            outer_window_ev: bounds("dis_win_min", "dis_win_max")?,
            frozen_window_ev: if optional::<f64>(&v, &[WANNIER, "wannier90", "dis_froz_min"])?
                .is_some()
            {
                bounds("dis_froz_min", "dis_froz_max")?
            } else {
                None
            },
            frozen_max_ev: if optional::<f64>(&v, &[WANNIER, "wannier90", "dis_froz_min"])?
                .is_none()
            {
                optional(&v, &[WANNIER, "wannier90", "dis_froz_max"])?
            } else {
                None
            },
            localization: config::Convergence {
                max_iterations: get(&v, &[WANNIER, "wannier90", "num_iter"])?,
                tolerance_angstrom2: get(&v, &[WANNIER, "wannier90", "conv_tol"])?,
                window_iterations: get(&v, &[WANNIER, "wannier90", "conv_window"])?,
            },
            disentanglement: Some(config::Convergence {
                max_iterations: get(&v, &[WANNIER, "wannier90", "dis_num_iter"])?,
                tolerance_angstrom2: get(&v, &[WANNIER, "wannier90", "dis_conv_tol"])?,
                window_iterations: get(&v, &[WANNIER, "wannier90", "dis_conv_window"])?,
            }),
        },
        interpolation: config::Interpolation {
            k_grid: grid(fine, &[], "nkf")?,
            q_grid: grid(fine, &[], "nqf")?,
            electron_smearing: match get::<i32>(&v, &[FINE, "epw", "INPUTEPW", "ngaussw"])? {
                0 => config::Smearing::Gaussian,
                1 => config::Smearing::Mp,
                -1 => config::Smearing::Mv,
                -99 => config::Smearing::Fd,
                _ => bail!("configuration: stages.{FINE}.epw.INPUTEPW.ngaussw: unsupported smearing"),
            },
            degaussw_ev: get(&v, &[FINE, "epw", "INPUTEPW", "degaussw"])?,
            phonon_smearings_mev: {
                let mut smearings = vec![get(&v, &[FINE, "epw", "INPUTEPW", "degaussq"])?];
                if let Some(tasks) = v[FINE].get("spectra").and_then(Value::as_object) {
                    for task in tasks.values() {
                        smearings.push(get(task, &["epw", "INPUTEPW", "degaussq"])?);
                    }
                }
                smearings
            },
            spectral_points: get(&v, &[FINE, "epw", "INPUTEPW", "nqstep"])?,
            fsthick_ev: get(&v, &[FINE, "epw", "INPUTEPW", "fsthick"])?,
            eps_acoustic_cm1: get(&v, &[FINE, "epw", "INPUTEPW", "eps_acoustic"])?,
            fermi_source: get(fine_wf, &["fermi_source"])?,
        },
        superconductivity: sc,
    })
}

fn validate_node(node: &Value, path: &str) -> Result<()> {
    let object = node
        .as_object()
        .with_context(|| format!("configuration: {path}: configuration table required"))?;
    let allowed=section_parameters(path.rsplit('.').next().unwrap_or(""));
    if let Some(allowed) = allowed {
        for (key, value) in object {
            ensure!(
                allowed.split_whitespace().any(|k| k == key),
                "configuration: {path}.{key}: unknown or unsupported native parameter"
            );
            validate_parameter(key, value, &format!("{path}.{key}"))?;
        }
        return Ok(());
    }
    validate_task_node(node, path)
}

fn section_parameters(section: &str) -> Option<&'static str> {
    match section {
        "CONTROL"=>Some("calculation prefix outdir pseudo_dir verbosity tstress tprnfor disk_io restart_mode wf_collect nstep"),
        "SYSTEM"=>Some("ibrav nat ntyp nbnd ecutwfc ecutrho occupations smearing degauss nosym noinv force_symmorphic use_all_frac nr1 nr2 nr3 nr1s nr2s nr3s"),
        "ELECTRONS"=>Some("conv_thr electron_maxstep mixing_beta mixing_ndim mixing_mode diagonalization diago_thr_init diago_david_ndim startingpot startingwfc scf_must_converge"),
        "INPUTPH"=>Some("prefix outdir tr2_ph ldisp only_init lqdir recover only_wfc fildyn fildvscf nq1 nq2 nq3 start_q last_q alpha_mix nmix_ph niter_ph trans epsil amass verbosity"),
        "BANDS"=>Some("prefix outdir filband lsym no_overlap lp plot_2d"),
        "DOS"=>Some("prefix outdir fildos ngauss degauss Emin Emax DeltaE bz_sum"),
        "PROJWFC"=>Some("prefix outdir filpdos filproj ngauss degauss Emin Emax DeltaE lsym pawproj lwrite_overlaps diag_basis kresolveddos"),
        "INPUT"=>Some("fildyn flfrc zasr asr flfrq fldos q_in_cryst_coord q_in_band_form dos ndos nk1 nk2 nk3 amass loto_2d write_lr read_lr"),
        "INPUTEPW"=>Some("prefix outdir dvscf_dir ep_coupling elph etf_mem nbndsub iverbosity amass nk1 nk2 nk3 nq1 nq2 nq3 nkf1 nkf2 nkf3 nqf1 nqf2 nqf3 fsthick degaussw degaussq nqstep nqsmear delta_qsmear eps_acoustic elecselfen phonselfen a2f ngaussw efermi_read fermi_energy epwread epwwrite epbread wannierize ephwrite eliashberg band_plot num_iter proj bands_skipped wdata dis_win_min dis_win_max dis_froz_min dis_froz_max selecqread filkf filqf liso laniso mp_mesh_k nstemp temps tc_linear tc_linear_solver limag lreal lpade lacon imag_read fbw positive_matsu gridsamp nswi muc wscut nsiter conv_thr_iaxis conv_thr_raxis broyden_beta broyden_ndim a_gap0 gap_edge fila2f"),
        "wannier90"=>Some("num_wann num_iter conv_tol conv_window dis_num_iter dis_conv_tol dis_conv_window guiding_centres projections exclude_bands bands_plot bands_num_points dis_proj_min dis_proj_max dis_froz_proj dis_win_min dis_win_max dis_froz_min dis_froz_max"),
        "postw90"=>Some("write_hr write_bvec bands_plot dos num_elec_per_state dos_kmesh dos_energy_min dos_energy_max dos_energy_step dos_adpt_smr dos_smr_fixed_en_width dos_smr_type fermi_energy"),
        _=>None,
    }
}

fn validate_task_node(node: &Value, path: &str) -> Result<()> {
    let object = node.as_object().context("configuration: configuration table required")?;
    if path.ends_with(".spectra") {
        for (name, task) in object {
            ensure!(
                !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "configuration: {path}.{name}: invalid subtask name"
            );
            validate_node(task, &format!("{path}.{name}"))?;
        }
        return Ok(());
    }
    for (key, value) in object {
        if key == "workflow" {
            let workflow = value
                .as_object()
                .with_context(|| format!("configuration: {path}.workflow: must be a table"))?;
            for (name, item) in workflow {
                ensure!(
                    [
                        "derived",
                        "points_per_edge",
                        "phonon_sigma_cm1",
                        "fermi_source",
                        "selection_mode",
                        "task",
                        "mode",
                        "spectrum_index",
                        "initial_gaps",
                        "iso",
                        "aniso"
                    ]
                    .contains(&name.as_str()),
                    "configuration: {path}.workflow.{name}: unknown workflow setting"
                );
                if name == "derived" {
                    for (target, rule) in item
                        .as_object()
                        .context("configuration: workflow.derived must be a table")?
                    {
                        let field = target.rsplit('.').next().unwrap();
                        ensure!(["outdir","pseudo_dir","ibrav","nat","ntyp","fildyn","flfrc","flfrq","fldos","filband","fildos","filpdos","dvscf_dir","fermi_energy","proj","bands_skipped","wdata","dis_win_min","dis_win_max","dis_froz_min","dis_froz_max","dos_energy_min","dos_energy_max","start_q","last_q","gap_edge","tc_linear","fila2f"].contains(&field),"configuration: {path}.workflow.derived.{target}: parameter cannot be derived automatically");
                        ensure!(
                            rule.as_str().is_some_and(|s| !s.trim().is_empty()),
                            "configuration: {path}.workflow.derived.{target}: derivation source missing"
                        );
                    }
                }
            }
        } else if key == "K_POINTS" || key == "ATOMIC_SPECIES" {
            ensure!(
                path.ends_with(".pw") && (value.is_string() || (key == "K_POINTS" && (path == format!("stages.{SCF}.pw") || path == format!("stages.{NSCF}.pw")) && value.is_array())),
                "configuration: {path}.{key}: native PW card text required"
            );
        } else {
            ensure!(
                [
                    "pw",
                    "spectra",
                    "ph",
                    "q2r",
                    "matdyn",
                    "dos",
                    "projwfc",
                    "bands",
                    "epw",
                    "wannier90",
                    "postw90",
                    "qe_eband",
                    "qe_eledos",
                    "q",
                    "dispersion",
                    "comparison",
                    "electron_dos",
                    "iso",
                    "aniso",
                    "CONTROL",
                    "SYSTEM",
                    "ELECTRONS",
                    "INPUTPH",
                    "INPUT",
                    "BANDS",
                    "DOS",
                    "PROJWFC",
                    "INPUTEPW"
                ]
                .contains(&key.as_str()),
                "configuration: {path}.{key}: unknown stage task or input section"
            );
            validate_node(value, &format!("{path}.{key}"))?;
        }
    }
    Ok(())
}

fn validate_parameter(key: &str, value: &Value, path: &str) -> Result<()> {
    let booleans = "tstress tprnfor wf_collect nosym noinv force_symmorphic use_all_frac scf_must_converge ldisp only_init lqdir recover only_wfc trans epsil lsym no_overlap lp plot_2d pawproj lwrite_overlaps diag_basis kresolveddos q_in_cryst_coord q_in_band_form dos loto_2d write_lr read_lr ep_coupling elph elecselfen phonselfen a2f efermi_read epwread epwwrite epbread wannierize ephwrite eliashberg band_plot selecqread liso laniso mp_mesh_k tc_linear limag lreal lpade lacon imag_read fbw positive_matsu guiding_centres dis_froz_proj write_hr write_bvec bands_plot dos_adpt_smr";
    let strings = "calculation prefix outdir pseudo_dir verbosity disk_io restart_mode occupations smearing mixing_mode diagonalization startingpot startingwfc fildyn fildvscf filband fildos bz_sum filpdos filproj flfrc zasr asr flfrq fldos dvscf_dir bands_skipped filkf filqf fila2f dos_smr_type tc_linear_solver";
    let integers = "nr1 nr2 nr3 nr1s nr2s nr3s nstep ibrav nat ntyp nbnd electron_maxstep mixing_ndim diago_david_ndim nq1 nq2 nq3 start_q last_q nmix_ph niter_ph ngauss ndos nk1 nk2 nk3 etf_mem nbndsub iverbosity nkf1 nkf2 nkf3 nqf1 nqf2 nqf3 nqstep nqsmear ngaussw num_iter nstemp gridsamp nswi nsiter broyden_ndim num_wann conv_window dis_num_iter dis_conv_window num_elec_per_state bands_num_points";
    let is = |list: &str| list.split_whitespace().any(|s| s == key);
    let valid = if is(booleans) {
        value.is_boolean()
    } else if is(strings) {
        value
            .as_str()
            .is_some_and(|s| !s.contains(['\n', '\r', '\0']))
    } else if is(integers) {
        value.as_i64().is_some()
    } else if ["amass", "temps", "alpha_mix"].contains(&key) {
        value
            .as_array()
            .is_some_and(|a| !a.is_empty() && a.iter().all(Value::is_number))
    } else if ["projections", "proj", "wdata"].contains(&key) {
        value.as_array().is_some_and(|a| {
            a.iter()
                .all(|s| s.as_str().is_some_and(|s| !s.contains(['\n', '\r', '\0'])))
        })
    } else if key == "exclude_bands" {
        value
            .as_array()
            .is_some_and(|a| a.iter().all(|n| n.as_u64().is_some_and(|n| n > 0)))
    } else if key == "dos_kmesh" {
        value
            .as_array()
            .is_some_and(|a| a.len() == 3 && a.iter().all(|n| n.as_u64().is_some_and(|n| n > 0)))
    } else {
        value.is_number()
    };
    ensure!(valid, "configuration: {path}: native parameter type error");
    Ok(())
}

impl Config {
    pub fn validate_native(&self) -> Result<()> {
        self.validate_native_impl().map_err(|e| self.config_error(e))
    }
    fn validate_native_impl(&self) -> Result<()> {
        ensure_version(self.schema_version)?;
        let v = self.stage_document()?;
        let ph = self.phonon_stage();
        let expected = [
            SCF,
            NSCF,
            ELECTRONIC,
            ph.as_str(),
            WANNIER,
            COARSE,
            FINE,
            SUPER,
        ];
        for key in v
            .as_object()
            .context("configuration: stages must be a table")?
            .keys()
        {
            ensure!(
                expected.contains(&key.as_str()),
                "configuration: stages.{key}: unknown stage"
            );
        }
        for name in expected {
            at(&v, &[name])?;
        }
        fn finite(value: &Value, path: &str) -> Result<()> {
            match value {
                Value::Null => bail!("configuration: {path}: empty value or nonfinite number"),
                Value::Number(n) => ensure!(
                    n.as_f64().is_some_and(f64::is_finite),
                    "configuration: {path}: nonfinite number"
                ),
                Value::String(s) => ensure!(
                    !s.contains(['\0', '\r']),
                    "configuration: {path}: invalid control character"
                ),
                Value::Array(a) => {
                    for (i, x) in a.iter().enumerate() {
                        finite(x, &format!("{path}[{i}]"))?;
                    }
                }
                Value::Object(o) => {
                    for (k, x) in o {
                        finite(x, &format!("{path}.{k}"))?;
                    }
                }
                _ => (),
            };
            Ok(())
        }
        finite(&v, "stages")?;
        for (name, stage) in v.as_object().unwrap() {
            validate_node(stage, &format!("stages.{name}"))?;
        }
        for stage in [SCF, NSCF] {
            let calculation: String = get(&v, &[stage, "pw", "CONTROL", "calculation"])?;
            ensure!(
                if stage == SCF { calculation == "scf" } else { ["nscf", "bands"].contains(&calculation.as_str()) },
                "configuration: stages.{stage}.pw.CONTROL.calculation: does not match the stage definition"
            );
            let cut: f64 = get(&v, &[stage, "pw", "SYSTEM", "ecutwfc"])?;
            let rho: f64 = get(&v, &[stage, "pw", "SYSTEM", "ecutrho"])?;
            ensure!(
                cut > 0. && rho >= cut,
                "configuration: stages.{stage}.pw.SYSTEM.ecutwfc/ecutrho: invalid cutoff energies"
            );
        }
        let (scf_grid, shift) = pw_grid(&v, SCF)?;
        let (nscf_grid, _) = pw_grid(&v, NSCF)?;
        ensure!(scf_grid == nscf_grid && shift == [0; 3],
            "configuration: stages.{SCF}.pw.K_POINTS={scf_grid:?}, shift={shift:?} and stages.{NSCF}.pw.K_POINTS={nscf_grid:?}: identical zero-shift grids required");
        for task in [vec![SCF, "pw"], vec![NSCF, "pw"], vec![ELECTRONIC, "qe_eband", "pw"]] {
            let system = at(&v, &[task.as_slice(), &["SYSTEM"]].concat())?;
            for keys in [["nr1", "nr2", "nr3"], ["nr1s", "nr2s", "nr3s"]] {
                let present = keys.iter().filter(|k| system.get(**k).is_some()).count();
                ensure!(present == 0 || (present == 3 && keys.iter().all(|k| system[*k].as_u64().is_some_and(|n| n > 0 && n <= i32::MAX as u64))),
                    "configuration: stages.{}.SYSTEM.{keys:?}: FFT requires three positive integers", task.join("."));
            }
        }
        // 重复参数只做约束检查，不把上游值写入下游配置。
        for task in [
            vec![SCF, "pw"],
            vec![NSCF, "pw"],
            vec![ELECTRONIC, "qe_eband", "pw"],
        ] {
            let system = at(&v, &[task.as_slice(), &["SYSTEM"]].concat())?;
            let occ: String = get(system, &["occupations"])?;
            if occ == "smearing" {
                get::<String>(system, &["smearing"])?;
                config::positive(
                    get(system, &["degauss"])?,
                    &format!("stages.{}.SYSTEM.degauss", task.join(".")),
                )?;
            } else {
                ensure!(
                    occ == "fixed"
                        && system.get("degauss").is_none()
                        && system.get("smearing").is_none(),
                    "configuration: stages.{}.SYSTEM.occupations: occupations conflict with smearing fields",
                    task.join(".")
                );
            }
        }
        for (name, stage) in v.as_object().unwrap() {
            let rules = at(stage, &["workflow", "derived"])?
                .as_object()
                .context("configuration: derivation table missing")?;
            for target in rules.keys() {
                let mut cursor = stage;
                let mut present = true;
                for key in target.split('.') {
                    if let Some(n) = cursor.get(key) {
                        cursor = n;
                    } else {
                        present = false;
                        break;
                    }
                }
                ensure!(!present,"configuration: stages.{name}.{target}: both explicit and internally derived definitions provided");
            }
        }
        let prefix: String = get(&v, &[SCF, "pw", "CONTROL", "prefix"])?;
        let same = |path: &[&str], expected: Value, source: &str| -> Result<()> {
            let actual = at(&v, path)?;
            ensure!(
                *actual == expected,
                "input_conflict: stages.{}={} differs from {source}={}",
                path.join("."),
                actual,
                expected
            );
            Ok(())
        };
        // 各阶段独立选择路径密度；物理路径身份在读取原生坐标时核对。
        same(
            &[WANNIER, "epw", "INPUTEPW", "num_iter"],
            at(&v, &[WANNIER, "wannier90", "num_iter"])?.clone(),
            &format!("stages.{WANNIER}.wannier90.num_iter (the same embedded fit)"),
        )?;
        same(
            &[WANNIER, "wannier90", "bands_plot"],
            json!(true),
            "Current electronic comparison delivery contract",
        )?;
        same(
            &[NSCF, "pw", "CONTROL", "prefix"],
            json!(prefix),
            &format!("stages.{SCF}.pw.CONTROL.prefix"),
        )?;
        for task in [
            vec![ELECTRONIC, "qe_eband", "pw", "CONTROL", "prefix"],
            vec![ELECTRONIC, "qe_eband", "bands", "BANDS", "prefix"],
            vec![ELECTRONIC, "qe_eledos", "dos", "DOS", "prefix"],
            vec![ELECTRONIC, "qe_eledos", "projwfc", "PROJWFC", "prefix"],
            vec![&ph, "ph", "INPUTPH", "prefix"],
            vec![SUPER, "iso", "epw", "INPUTEPW", "prefix"],
            vec![SUPER, "aniso", "epw", "INPUTEPW", "prefix"],
        ] {
            same(
                &task,
                json!(prefix),
                &format!("stages.{SCF}.pw.CONTROL.prefix (the same dataset)"),
            )?;
        }
        same(
            &[&ph, "ph", "INPUTPH", "only_init"],
            json!(self.phonon.mode != config::PhMode::Nosplit),
            "Phonon splitting mode",
        )?;
        if self.phonon.mode != config::PhMode::Nosplit {
            same(
                &[&ph, "q", "ph", "INPUTPH", "prefix"],
                json!(prefix),
                &format!("stages.{SCF}.pw.CONTROL.prefix"),
            )?;
            same(
                &[&ph, "q", "ph", "INPUTPH", "only_init"],
                json!(false),
                "Actual response calculation per q point",
            )?;
            same(
                &[&ph, "q", "ph", "INPUTPH", "ldisp"],
                json!(self.phonon.mode == config::PhMode::Index),
                "Phonon point source mode",
            )?;
        }
        for (stage, flags) in [
            (
                WANNIER,
                vec![("wannierize", true), ("epwread", false), ("elph", false)],
            ),
            (
                COARSE,
                vec![("wannierize", false), ("epwread", false), ("elph", true)],
            ),
            (
                FINE,
                vec![
                    ("wannierize", false),
                    ("epwread", true),
                    ("ephwrite", true),
                    ("liso", false),
                    ("laniso", false),
                ],
            ),
        ] {
            for (key, flag) in flags {
                same(
                    &[stage, "epw", "INPUTEPW", key],
                    json!(flag),
                    "Current stage native execution contract",
                )?;
            }
        }
        for stage in [WANNIER, COARSE, FINE] {
            same(
                &[stage, "epw", "INPUTEPW", "prefix"],
                json!(prefix),
                &format!("stages.{SCF}.pw.CONTROL.prefix"),
            )?;
            for (base, grid, source) in [
                (
                    "nk",
                    self.qe.nscf.k_grid,
                    format!("stages.{NSCF}.pw.K_POINTS"),
                ),
                (
                    "nq",
                    self.phonon.q_grid,
                    format!("stages.{ph}.ph.INPUTPH.nq1/nq2/nq3"),
                ),
            ] {
                if stage == WANNIER && base == "nq" {
                    continue;
                }
                for (i, n) in grid.iter().enumerate() {
                    same(
                        &[stage, "epw", "INPUTEPW", &format!("{base}{}", i + 1)],
                        json!(n),
                        &source,
                    )?;
                }
            }
            same(
                &[stage, "epw", "INPUTEPW", "nbndsub"],
                json!(self.wannier.num_wann),
                &format!("stages.{WANNIER}.wannier90.num_wann"),
            )?;
        }
        for stage in [WANNIER, COARSE, FINE] {
            let masses: Vec<f64> = get(&v, &[stage, "epw", "INPUTEPW", "amass"])?;
            ensure!(
                masses.len() == self.atomic_masses.len()
                    && masses.iter().all(|m| m.is_finite() && *m > 0.),
                "configuration: stages.{stage}.epw.INPUTEPW.amass: requires positive masses in structure-species order"
            );
        }
        if let Some(tasks) = v[FINE].get("spectra").and_then(Value::as_object) {
            for (name, task) in tasks {
                for key in [
                    "nkf1",
                    "nkf2",
                    "nkf3",
                    "nqf1",
                    "nqf2",
                    "nqf3",
                    "fsthick",
                    "degaussw",
                    "ngaussw",
                    "efermi_read",
                    "epwread",
                    "ephwrite",
                    "wannierize",
                ] {
                    let actual = at(task, &["epw", "INPUTEPW", key])?;
                    let expected = at(&v, &[FINE, "epw", "INPUTEPW", key])?;
                    ensure!(actual==expected,"input_conflict: stages.{FINE}.spectra.{name}.epw.INPUTEPW.{key}={actual} differs from stages.{FINE}.epw.INPUTEPW.{key}={expected}: bound to the same native matrix");
                }
            }
        }
        let nk = self.native_value::<u32>(&[FINE, "epw", "INPUTEPW", "nstemp"])?;
        let temps = self.native_value::<Vec<f64>>(&[FINE, "epw", "INPUTEPW", "temps"])?;
        ensure!(
            nk as usize == temps.len() && nk > 0,
            "configuration: stages.{FINE}.epw.INPUTEPW.nstemp/temps: length mismatch"
        );
        for mode in [config::Mode::Iso, config::Mode::Aniso] {
            let name = if mode == config::Mode::Iso {
                "iso"
            } else {
                "aniso"
            };
            let s = self.solver_for(mode)?;
            for (key,expected) in [("epwread",true),("wannierize",false),("elph",false),("eliashberg",true),("limag",true),("lreal",false)] {
                same(&[SUPER,name,"epw","INPUTEPW",key],json!(expected),"Current imaginary-axis solver stage contract")?;
            }
            if s.task == config::SuperconductivityTask::Gap {
                same(&[SUPER,name,"epw","INPUTEPW","tc_linear"],json!(false),"Nonlinear gap calculation at specified temperatures")?;
            }
            let n: u32 = get(&v, &[SUPER, name, "epw", "INPUTEPW", "nstemp"])?;
            ensure!(
                n as usize == s.gap_temperatures_k.len() && n > 0 && n <= 50,
                "configuration: stages.{SUPER}.{name}.epw.INPUTEPW.nstemp/temps: length mismatch"
            );
            for (base, g) in [
                ("nkf", self.interpolation.k_grid),
                ("nqf", self.interpolation.q_grid),
            ] {
                for (i, x) in g.iter().enumerate() {
                    same(
                        &[SUPER, name, "epw", "INPUTEPW", &format!("{base}{}", i + 1)],
                        json!(x),
                        &format!("stages.{FINE}.epw.INPUTEPW.{base}{}", i + 1),
                    )?;
                }
            }
            same(
                &[SUPER, name, "epw", "INPUTEPW", "liso"],
                json!(mode == config::Mode::Iso),
                "Current method definition",
            )?;
            same(
                &[SUPER, name, "epw", "INPUTEPW", "laniso"],
                json!(mode == config::Mode::Aniso),
                "Current method definition",
            )?;
            config::positive(
                s.frequency_cutoff_ev,
                &format!("stages.{SUPER}.{name}.epw.INPUTEPW.wscut"),
            )?;
            ensure!(
                s.mu_star >= 0.
                    && s.max_iterations > 0
                    && s.convergence_relative > 0.
                    && s.mixing_beta.abs() > 0.
                    && s.mixing_beta.abs() <= 1.
                    && (1..=8).contains(&s.mixing_history),
                "configuration: stages.{SUPER}.{name}.epw.INPUTEPW: invalid iteration/mixing parameters"
            );
            let mut seen = std::collections::BTreeSet::new();
            if s.task == config::SuperconductivityTask::Gap {
                config::positive(
                    s.initial_gap(),
                    &format!("stages.{SUPER}.{name}.epw.INPUTEPW.gap_edge"),
                )?;
            }
            for t in &s.gap_temperatures_k {
                ensure!(*t <= 999.99, "configuration: stages.{SUPER}.{name}.epw.INPUTEPW.temps: native temperature-file format supports at most 999.99 K");
                ensure!(
                    seen.insert(config::temperature(*t, "temps")?),
                    "configuration: stages.{SUPER}.{name}.epw.INPUTEPW.temps: duplicate temperature"
                );
                config::temperature(*t, &format!("stages.{SUPER}.{name}.epw.INPUTEPW.temps"))?;
                if s.task == config::SuperconductivityTask::Gap
                    && (s.mode == config::Mode::Both || s.mode == mode)
                {
                    crate::superconductivity::attempt::Request::from_config(
                        self,
                        mode,
                        crate::superconductivity::attempt::SolveKind::Gap,
                        config::temperature(*t, "temps")?,
                        Some(s.initial_gap()),
                    )?;
                }
            }
        }
        Ok(())
    }
    /// 多展宽是显式子任务，各子任务必须完整填写 INPUTEPW；不继承主任务。
    pub fn fine_task(&self, smearing: f64) -> Result<Vec<String>> {
        let values = self.stage_document()?;
        let primary: f64 = get(&values, &[FINE, "epw", "INPUTEPW", "degaussq"])?;
        if primary == smearing {
            return Ok(vec![FINE.into(), "epw".into()]);
        }
        if let Some(tasks) = values[FINE].get("spectra").and_then(Value::as_object) {
            for (name, task) in tasks {
                if get::<f64>(task, &["epw", "INPUTEPW", "degaussq"])? == smearing {
                    return Ok(vec![
                        FINE.into(),
                        "spectra".into(),
                        name.clone(),
                        "epw".into(),
                    ]);
                }
            }
        }
        bail!("configuration: stages.{FINE}: degaussq={smearing} has no corresponding explicit native subtask")
    }
    pub fn stage_document(&self) -> Result<Value> {
        Ok(stages(&self.document()?))
    }
    pub fn native_value<T: serde::de::DeserializeOwned>(&self, path: &[&str]) -> Result<T> {
        get(&self.stage_document()?, path)
    }
    /// 原生编辑只有一个入口，立即重新建立算法视图；不允许两份参数各自漂移。
    pub fn set_native(&mut self, path: &[&str], value: Value) -> Result<()> {
        let mut d = self.document()?;
        let mut v = stages(&d);
        put(&mut v, path, value);
        d.input_locations.remove(&flat::location(&format!("stages.{}", path.join("."))));
        if path.last().is_some_and(|k|["fermi_energy","fila2f"].contains(k)) && path.len()>1 {
            let location=path[1..].join(".");
            if let Some(derived)=v.get_mut(path[0]).and_then(|s|s.get_mut("workflow")).and_then(|s|s.get_mut("derived")).and_then(Value::as_object_mut) {
                derived.remove(&location);
            }
        }
        d.stages = serde_json::from_value(v)?;
        let runtime = decode(&d).map_err(|e| self.config_error(e))?;
        self.document = d;
        self.runtime = runtime;
        Ok(())
    }
    pub fn phonon_stage(&self) -> String {
        format!(
            "4.qe-phonon-{}",
            match self.phonon.mode {
                config::PhMode::Nosplit => "nosplit",
                config::PhMode::Index => "index",
                config::PhMode::Coords => "coords",
            }
        )
    }
    pub fn solver_for(&self, mode: config::Mode) -> Result<config::Superconductivity> {
        let owner=if self.superconductivity.mode == config::Mode::Aniso {config::Mode::Aniso} else {config::Mode::Iso};
        if mode == owner {
            return Ok(self.superconductivity.clone());
        }
        solver(
            &self.stage_document()?,
            if mode == config::Mode::Iso {"iso"} else {"aniso"},
            self.superconductivity.task,
        )
    }
    fn document(&self) -> Result<Document> {
        let mut d = self.document.clone();
        d.plot = self.plot.clone();
        d.schema_version = self.schema_version;
        d.work_dir = self.work_dir.clone();
        d.outdir = self.outdir.clone();
        d.structure = self.structure.clone();
        d.pseudos = self.pseudos.clone();
        d.programs = self.programs.clone();
        d.slurm = self.slurm.clone();
        d.layout = self.layout.clone();
        d.kpath = self.kpath.clone();
        // 算法视图的可变字段仅写回其所有者；其他阶段始终保存自己的原生设置。
        let mut v = stages(&d);
        let ph = ph_name(&d)?;
        let active = self.phonon_stage();
        if ph != active {
            let item = v.as_object_mut().unwrap().remove(&ph).unwrap();
            v[&active] = item;
        }
        put(
            &mut v,
            &[SCF, "pw", "CONTROL", "prefix"],
            json!(self.qe.prefix),
        );
        for (key, value) in [
            ("ecutwfc", json!(self.qe.ecutwfc_ry)),
            ("ecutrho", json!(self.qe.ecutrho_ry)),
            ("occupations", json!(self.qe.occupations)),
            ("smearing", json!(self.qe.smearing)),
            ("degauss", json!(self.qe.degauss_ry)),
        ] {
            put(&mut v, &[SCF, "pw", "SYSTEM", key], value);
        }
        for (stage, g, shift, thr) in [
            (
                SCF,
                self.qe.scf.k_grid,
                self.qe.scf.k_shift,
                self.qe.scf.conv_thr_ry,
            ),
            (NSCF, self.qe.nscf.k_grid, [0; 3], self.qe.nscf.conv_thr_ry),
        ] {
            let legacy_automatic = at(&v, &[stage, "pw", "K_POINTS"])?
                .as_str().is_some_and(|text| text.trim_start().starts_with("automatic"));
            if pw_grid(&v, stage)? != (g, shift) || legacy_automatic {
                let card = if shift == [0; 3] {
                    json!(g)
                } else {
                    json!([g[0], g[1], g[2], shift[0], shift[1], shift[2]])
                };
                put(&mut v, &[stage, "pw", "K_POINTS"], card);
            }
            put(&mut v, &[stage, "pw", "ELECTRONS", "conv_thr"], json!(thr));
        }
        put(
            &mut v,
            &[NSCF, "pw", "SYSTEM", "nbnd"],
            json!(self.qe.nscf.nbnd),
        );
        put(
            &mut v,
            &[&active, "ph", "INPUTPH", "tr2_ph"],
            json!(self.phonon.tr2_ph),
        );
        for (i, n) in self.phonon.q_grid.iter().enumerate() {
            put(
                &mut v,
                &[&active, "ph", "INPUTPH", &format!("nq{}", i + 1)],
                json!(n),
            );
        }
        let w = &self.wannier;
        let wp = [WANNIER, "wannier90"];
        for (key, value) in [
            ("num_wann", json!(w.num_wann)),
            ("num_iter", json!(w.localization.max_iterations)),
            ("conv_tol", json!(w.localization.tolerance_angstrom2)),
            ("conv_window", json!(w.localization.window_iterations)),
            ("guiding_centres", json!(w.guiding_centres)),
            ("projections", json!(w.projections)),
            ("exclude_bands", json!(w.exclude_bands)),
        ] {
            put(&mut v, &[wp[0], wp[1], key], value);
        }
        put(
            &mut v,
            &[WANNIER, "workflow", "selection_mode"],
            json!(w.selection_mode),
        );
        for (keys, values) in [
            (["dis_win_min", "dis_win_max"], w.outer_window_ev),
            (["dis_froz_min", "dis_froz_max"], w.frozen_window_ev),
        ] {
            for (i, key) in keys.iter().enumerate() {
                put(
                    &mut v,
                    &[WANNIER, "wannier90", key],
                    json!(values.map(|a| a[i]).or(if *key == "dis_froz_max" {
                        w.frozen_max_ev
                    } else {
                        None
                    })),
                );
            }
        }
        if let Some(c) = &w.disentanglement {
            for (k, x) in [
                ("dis_num_iter", json!(c.max_iterations)),
                ("dis_conv_tol", json!(c.tolerance_angstrom2)),
                ("dis_conv_window", json!(c.window_iterations)),
            ] {
                put(&mut v, &[WANNIER, "wannier90", k], x);
            }
        }
        let i = &self.interpolation;
        for (key, val) in [
            ("degaussw", json!(i.degaussw_ev)),
            (
                "degaussq",
                json!(i
                    .phonon_smearings_mev
                    .first()
                    .context("configuration: stages.7.epw-fine.epw.INPUTEPW.degaussq: missing")?),
            ),
            ("fsthick", json!(i.fsthick_ev)),
            ("eps_acoustic", json!(i.eps_acoustic_cm1)),
            ("nqstep", json!(i.spectral_points)),
            (
                "ngaussw",
                json!(match i.electron_smearing {
                    config::Smearing::Gaussian => 0,
                    config::Smearing::Mp => 1,
                    config::Smearing::Mv => -1,
                    config::Smearing::Fd => -99,
                }),
            ),
        ] {
            put(&mut v, &[FINE, "epw", "INPUTEPW", key], val);
        }
        for (prefix, grid) in [("nkf", i.k_grid), ("nqf", i.q_grid)] {
            for (n, value) in grid.iter().enumerate() {
                put(
                    &mut v,
                    &[FINE, "epw", "INPUTEPW", &format!("{prefix}{}", n + 1)],
                    json!(value),
                );
            }
        }
        put(
            &mut v,
            &[FINE, "workflow", "fermi_source"],
            json!(i.fermi_source),
        );
        let sc = &self.superconductivity;
        for (k, val) in [
            ("task", json!(sc.task)),
            ("mode", json!(sc.mode)),
            ("spectrum_index", json!(sc.spectrum_index)),
            ("iso", json!(sc.iso)),
            ("aniso", json!(sc.aniso)),
        ] {
            put(&mut v, &[SUPER, "workflow", k], val);
        }
        if sc.task == config::SuperconductivityTask::Tc {
            put(
                &mut v,
                &[SUPER, "workflow", "initial_gaps"],
                json!(sc.initial_gaps_mev.map(|x| x / 1000.)),
            );
        }
        let owner = if sc.mode == config::Mode::Aniso {
            "aniso"
        } else {
            "iso"
        };
        for (k, val) in [
            ("muc", json!(sc.mu_star)),
            ("wscut", json!(sc.frequency_cutoff_ev)),
            ("nsiter", json!(sc.max_iterations)),
            ("conv_thr_iaxis", json!(sc.convergence_relative)),
            ("broyden_beta", json!(sc.mixing_beta)),
            ("broyden_ndim", json!(sc.mixing_history)),
            ("a_gap0", json!(sc.initial_shape)),
            ("temps", json!(sc.gap_temperatures_k)),
        ] {
            put(&mut v, &[SUPER, owner, "epw", "INPUTEPW", k], val);
        }
        if sc.task == config::SuperconductivityTask::Gap {
            put(
                &mut v,
                &[SUPER, owner, "epw", "INPUTEPW", "gap_edge"],
                json!(sc.initial_gap() / 1000.),
            );
        }
        if let Some(c) = &self.comparison {
            put(
                &mut v,
                &[ELECTRONIC, "workflow", "points_per_edge"],
                json!(c.points_per_edge),
            );
            put(
                &mut v,
                &[WANNIER, "electron_dos", "postw90", "dos_smr_fixed_en_width"],
                json!(c.electronic_sigma_ev),
            );
            put(
                &mut v,
                &[&active, "workflow", "phonon_sigma_cm1"],
                json!(c.phonon_sigma_cm1),
            );
        }
        d.stages = serde_json::from_value(v)?;
        Ok(d)
    }
}

#[cfg(test)]
mod language_tests {
    use super::*;
    #[test]
    fn legacy_parameter_source_comment_does_not_change_native_input() {
        let value = serde_json::json!({
            "1.qe-scf": {"pw": {"CONTROL": {"calculation": "scf"}}, "workflow": {"derived": {}}}
        });
        let input = "&CONTROL\n calculation = 'scf'\n/\n";
        let legacy = format!("{}legacy provenance\n{input}", super::super::legacy_language::INPUT_SOURCE_PREFIX);
        assert_eq!(apply(&value, &[SCF, "pw"], &legacy).unwrap(), apply(&value, &[SCF, "pw"], input).unwrap());
    }
    #[test]
    fn generated_internal_bindings_preserve_legacy_scientific_identity() {
        let cfg: super::Config = toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        let expected: serde_json::Value = serde_json::from_str(include_str!("../../tests/fixtures/legacy-derived-rules.json")).unwrap();
        let scf: serde_json::Value = cfg.native_value(&[SCF, "workflow", "derived"]).unwrap();
        assert_eq!(scf["pw.CONTROL.outdir"], expected["outdir"]);
        assert_eq!(scf["pw.SYSTEM.ibrav"], expected["ibrav"]);
        let fine: serde_json::Value = cfg.native_value(&[FINE, "workflow", "derived"]).unwrap();
        assert_eq!(fine["epw.INPUTEPW.fermi_energy"], expected["fermi_energy"]);
    }

}
