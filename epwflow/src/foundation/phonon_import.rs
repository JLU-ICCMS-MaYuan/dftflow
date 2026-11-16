//! 原始 PH save 的显式导入。此模块没有任何 PW/PH 执行入口。
use super::{
    config::Config,
    layout, native_config,
    stages::{self, Step},
    store, workspace,
};
use crate::qe_data::{
    self,
    manifest::{Fingerprints, PhResponse, QeManifest},
    save,
    xml::{self, Geometry},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Source {
    schema_version: u32,
    root: PathBuf,
    prefix: String,
    files: Vec<save::TreeFile>,
}
pub fn binding(cfg: &Config) -> PathBuf {
    cfg.work_dir.join("epw-state/imports/phonon-source.json")
}
pub fn bound(cfg: &Config) -> bool {
    binding(cfg).is_file()
        || super::readable_state::read(&cfg.work_dir, Step::Phonon)
            .ok()
            .is_some_and(|d| d.get("Workspace", "External phonon source").is_some())
}

fn load_config(path: &Path) -> Result<(Config, bool)> {
    let raw: toml::Value = toml::from_str(&fs::read_to_string(path)?)?;
    if raw
        .get("stages")
        .and_then(|s| s.get(native_config::SCF))
        .and_then(|s| s.get("pw"))
        .is_some()
    {
        Ok((Config::load_nested(path)?, true))
    } else {
        Ok((Config::load(path)?, false))
    }
}
fn upgrade(path: &Path, cfg: &Config, _nested: bool, check: bool) -> Result<Config> {
    if !check {
        let version = super::readable_state::backup_config(path)?;
        super::readable_state::register_config(cfg, path, &version)?;
    }
    Ok(cfg.clone())
}

pub fn require_electronic(cfg: &Config) -> Result<()> {
    for s in [Step::Scf, Step::Nscf] {
        if !stages::reusable(cfg, s)? {
            let differences = super::readable_state::computation_config(&cfg.work_dir, s)
                .and_then(|p| super::readable_state::load_config(&p))
                .and_then(|old| {
                    Ok(parameter_differences(
                        "",
                        &stages::parameters(&old, s)?,
                        &stages::parameters(cfg, s)?,
                    ))
                })
                .unwrap_or_default();
            anyhow::bail!("required: import requires completed and verifiable {}; this path does not call PW/PH; resolve electronic data separately first. Parameter differences: {}",s.name(),if differences.is_empty(){"No configuration differences detected; inspect scientific output".into()}else{differences.join("、")});
        }
    }
    qe_data::nscf::verified_states(cfg, &cfg.work_dir)?;
    Ok(())
}
fn parameter_differences(
    prefix: &str,
    a: &serde_json::Value,
    b: &serde_json::Value,
) -> Vec<String> {
    if a == b {
        return vec![];
    }
    if let (Some(a), Some(b)) = (a.as_object(), b.as_object()) {
        let keys = a
            .keys()
            .chain(b.keys())
            .collect::<std::collections::BTreeSet<_>>();
        keys.into_iter()
            .flat_map(|key| {
                parameter_differences(
                    &format!("{prefix}.{key}"),
                    a.get(key).unwrap_or(&serde_json::Value::Null),
                    b.get(key).unwrap_or(&serde_json::Value::Null),
                )
            })
            .collect()
    } else {
        vec![prefix.trim_start_matches('.').into()]
    }
}
fn source(root: &Path) -> Result<Source> {
    let root = fs::canonicalize(root)?;
    let files = save::files(&root)?;
    let names = files
        .iter()
        .filter_map(|f| {
            f.relative_path
                .to_str()?
                .strip_suffix(".phsave/control_ph.xml")
        })
        .collect::<Vec<_>>();
    ensure!(
        names.len() == 1,
        "data_invalid: save requires exactly one prefix.phsave/control_ph.xml"
    );
    Ok(Source {
        schema_version: 1,
        prefix: names[0].into(),
        root,
        files,
    })
}
fn geometry(cfg: &Config) -> Result<Geometry> {
    Geometry::read(
        &layout::path(&cfg.work_dir, "scf/snapshot")
            .join(format!("{}.save/data-file-schema.xml", cfg.qe.prefix)),
    )
}
fn dyn0(source: &Source, cfg: &Config) -> Result<String> {
    let raw = fs::read_to_string(
        source
            .root
            .join(format!("{}.phsave/control_ph.xml", source.prefix)),
    )?;
    let doc = roxmltree::Document::parse(&raw)?;
    let root = doc.root_element();
    let grid = xml::numbers(xml::text(root, "MESH_DIMENSIONS")?)?;
    ensure!(
        grid == cfg.phonon.q_grid.map(f64::from),
        "input_conflict: control_ph.xml q grid differs from TOML"
    );
    let n: usize = xml::text(root, "NUMBER_OF_Q_POINTS")?.trim().parse()?;
    let coords = xml::numbers(xml::text(root, "Q-POINT_COORDINATES")?)?;
    ensure!(
        n > 0 && coords.len() == n * 3,
        "data_invalid: truncated control q-point list"
    );
    let mut out = format!(
        "{} {} {}\n{n}\n",
        cfg.phonon.q_grid[0], cfg.phonon.q_grid[1], cfg.phonon.q_grid[2]
    );
    for q in coords.chunks_exact(3) {
        out.push_str(&format!("{:.16e} {:.16e} {:.16e}\n", q[0], q[1], q[2]));
    }
    Ok(out)
}
fn dyn_path(s: &Source, n: usize) -> Result<PathBuf> {
    let candidates = [
        format!("{}.dyn_q{n}", s.prefix),
        format!("{}.dyn_q{n}.xml", s.prefix),
        format!("{}.dyn{n}", s.prefix),
        format!("{}.dyn{n}.xml", s.prefix),
    ];
    let found = candidates
        .iter()
        .map(|p| s.root.join(p))
        .filter(|p| p.is_file())
        .collect::<Vec<_>>();
    ensure!(found.len() == 1, "data_invalid: q{n} dynamical matrix missing or duplicated");
    Ok(found[0].clone())
}
fn canonical_dyn(raw: &str, g: &Geometry) -> Result<(String, Vec<[f64; 3]>)> {
    if !raw.trim_start().starts_with('<') {
        return qe_data::dynmat_text::convert(raw, g);
    }
    let doc = roxmltree::Document::parse(raw)?;
    let root = doc.root_element();
    let n: usize = xml::text(root, "NUMBER_OF_Q")?.trim().parse()?;
    ensure!(n > 0, "data_invalid: empty q-point star");
    let mut qs = vec![];
    for i in 1..=n {
        let mat = root
            .children()
            .find(|v| v.has_tag_name(format!("DYNAMICAL_MAT_.{i}").as_str()))
            .context("data_invalid: truncated q-point star")?;
        qs.push(xml::vector(xml::text(mat, "Q_POINT")?)?);
        for a in 1..=g.atoms.len() {
            for b in 1..=g.atoms.len() {
                ensure!(
                    xml::numbers(xml::text(mat, &format!("PHI.{a}.{b}"))?)?.len() == 18,
                    "data_invalid: truncated q-star matrix"
                );
            }
        }
    }
    for a in 1..=g.atoms.len() {
        let node = root
            .descendants()
            .find(|v| v.has_tag_name(format!("ATOM.{a}").as_str()))
            .context("data_invalid: atoms missing")?;
        ensure!(
            node.attribute("SPECIES") == Some(g.atoms[a - 1].0.as_str()),
            "data_invalid: XML element order mismatch"
        );
    }
    for m in 1..=g.atoms.len() * 3 {
        ensure!(
            xml::numbers(xml::text(root, &format!("DISPLACEMENT.{m}"))?)?.len()
                == g.atoms.len() * 6,
            "data_invalid: truncated XML modes"
        );
    }
    Ok((raw.into(), qs))
}

pub fn command(config: &Path, root: &Path, check: bool, submit: bool, resume: bool) -> Result<()> {
    ensure!(!resume || submit, "configuration: import recovery requires --resume -s");
    let (old, nested) = load_config(config)?;
    let source_path = if root.is_absolute() {
        root.into()
    } else {
        fs::canonicalize(config)?.parent().unwrap().join(root)
    };
    let source_path = fs::canonicalize(source_path)?;
    for managed in [
        old.work_dir.join("epw-state"),
        Step::Phonon.directory(&old),
        fs::canonicalize(config)?,
    ] {
        ensure!(
            !source_path.starts_with(&managed) && !managed.starts_with(&source_path),
            "input_conflict: phonon source overlaps a workflow output path {}",
            managed.display()
        );
    }
    println!("import: read-only validation of source and electronic evidence");
    let s = source(&source_path)?;
    ensure!(
        s.prefix == old.qe.prefix,
        "input_conflict: save prefix differs from current electronic data"
    );
    let _guard = if check {
        None
    } else {
        Some(store::Lock::acquire(
            &old.work_dir.join("epw-state/imports/control"),
        )?)
    };
    let cfg = upgrade(config, &old, nested, check)?;
    if check && !stages::reusable(&cfg, Step::Nscf)? {
        require_electronic(&crate::qe_data::direct::current(&cfg.work_dir)?)?;
    } else {
        require_electronic(&cfg)?;
    }
    if bound(&cfg) {
        let previous: Source = store::read(&binding(&cfg))?;
        ensure!(
            previous == s,
            "input_conflict: import source path or content changed; current data preserved; mixing phonon sources prohibited"
        );
    }
    let _ = dyn0(&s, &cfg)?;
    if check {
        // 临时目录仅承载转换，工作区和来源均不写入。
        let tmp = std::env::temp_dir().join(format!("epw-import-check-{}", store::token()));
        let result = inspect(&cfg, &s, &tmp);
        let cleanup = fs::remove_dir_all(&tmp);
        result?;
        cleanup?;
        println!("import_check: valid; historical SCF pseudopotentials/cutoffs unverified; input preparer must ensure consistency");
        return Ok(());
    }
    workspace::check_active(&cfg, None)?;
    super::native_jobs::check_groups(&cfg)?;
    store::write(&binding(&cfg), &s)?;
    super::readable_state::update(&cfg.work_dir, Step::Phonon, |d| {
        d.set("Workspace", "External phonon source", s.root.display());
        Ok(())
    })?;
    workspace::prepare_stage(config, Step::Phonon, resume, submit)
}

fn inspect(
    cfg: &Config,
    s: &Source,
    staging: &Path,
) -> Result<(Vec<qe_data::manifest::QPoint>, Vec<PhResponse>)> {
    fs::create_dir_all(staging)?;
    let g = geometry(cfg)?;
    let dyn0_path = staging.join("dyn0");
    store::atomic_bytes(&dyn0_path, dyn0(s, cfg)?.as_bytes())?;
    let points = qe_data::qmap::parse_dyn0(&dyn0_path, &g, cfg.phonon.q_grid, false)?;
    for file in &s.files {
        let name = file.relative_path.to_string_lossy();
        for base in [
            format!("{}.dyn_q", s.prefix),
            format!("{}.dvscf_q", s.prefix),
            format!("{}.phsave/patterns.", s.prefix),
        ] {
            if let Some(tail) = name.strip_prefix(&base) {
                let n: usize = tail
                    .trim_end_matches(".xml")
                    .parse()
                    .context("data_invalid: invalid phonon-file q index")?;
                ensure!(
                    (1..=points.len()).contains(&n),
                    "data_invalid: phonon-file q index exceeds the control list"
                );
            }
        }
    }
    let lists = [
        format!("{}.dyn0", s.prefix),
        format!("{}.dyn0.xml", s.prefix),
    ]
    .into_iter()
    .map(|p| s.root.join(p))
    .filter(|p| p.exists())
    .collect::<Vec<_>>();
    ensure!(lists.len() <= 1, "data_invalid: duplicate dyn0");
    if let Some(list) = lists.first() {
        let given = qe_data::qmap::parse_dyn0(list, &g, cfg.phonon.q_grid, false)?;
        ensure!(
            given.len() == points.len()
                && given.iter().zip(&points).all(|(a, b)| a
                    .cartesian
                    .iter()
                    .zip(b.cartesian)
                    .all(|(x, y)| (x - y).abs() < 1e-8)),
            "data_invalid: source dyn0 differs from control_ph.xml"
        );
    }
    let mut responses = vec![];
    let mut covered = BTreeSet::new();
    for q in &points {
        let source = dyn_path(s, q.global_index)?;
        let raw = fs::read_to_string(&source)?;
        let (canonical, stars) = canonical_dyn(&raw, &g)
            .with_context(|| format!("data_invalid: {}", source.display()))?;
        let doc = roxmltree::Document::parse(&canonical)?;
        let ntyp: usize = xml::text(doc.root_element(), "NUMBER_OF_TYPES")?
            .trim()
            .parse()?;
        ensure!(
            ntyp == cfg.pseudos.len(),
            "data_invalid: dyn element-type count mismatch"
        );
        for n in 1..=ntyp {
            let element = xml::text(doc.root_element(), &format!("TYPE_NAME.{n}"))?.trim();
            let mass = xml::numbers(xml::text(doc.root_element(), &format!("MASS.{n}"))?)?;
            let expected = qf_input::species::get_atomic_mass(
                element,
                cfg.atomic_masses.get(element).copied(),
            );
            ensure!(
                cfg.pseudos.contains_key(element)
                    && mass.len() == 1
                    && (mass[0] - expected).abs() < 1e-5,
                "input_conflict: dyn elements/atomic masses differ from current input"
            );
        }
        for star in stars {
            let crystal = g.crystal(star);
            let mut key = [0i64; 3];
            for i in 0..3 {
                let v = crystal[i] * cfg.phonon.q_grid[i] as f64;
                ensure!(
                    (v - v.round()).abs() < 1e-7,
                    "data_invalid: q-point star is outside the configured grid"
                );
                key[i] = (v.round() as i64).rem_euclid(cfg.phonon.q_grid[i] as i64);
            }
            ensure!(covered.insert(key), "data_invalid: duplicate physical q-point star");
        }
        let dyn_file = staging.join(format!("{}.dyn_q{}.xml", s.prefix, q.global_index));
        store::atomic_bytes(&dyn_file, canonical.as_bytes())?;
        let r = PhResponse {
            q_id: q.global_index,
            dyn_file,
            patterns: s.root.join(format!(
                "{}.phsave/patterns.{}.xml",
                s.prefix, q.global_index
            )),
            dvscf: Some(
                s.root
                    .join(format!("{}.dvscf_q{}", s.prefix, q.global_index)),
            ),
            control_metadata: s.root.join(format!("{}.phsave/control_ph.xml", s.prefix)),
            format: qe_data::native_ph::FORMAT.into(),
            perturbations: g.atoms.len() * 3,
        };
        qe_data::native_ph::verify(&r, q, &g)?;
        responses.push(r);
    }
    ensure!(
        covered.len()
            == cfg
                .phonon
                .q_grid
                .iter()
                .map(|n| *n as usize)
                .product::<usize>(),
        "data_invalid: q-point stars do not cover the complete coarse grid"
    );
    qe_data::native_ph::control(&responses[0].control_metadata, &points)?;
    Ok((points, responses))
}

pub(crate) fn collect(cfg: &Config, qe: &qe_data::qe_config::QeSettings) -> Result<QeManifest> {
    require_electronic(cfg)?;
    let s: Source = store::read(&binding(cfg))?;
    ensure!(
        s == source(&s.root)?,
        "input_conflict: source content changed after submission; import prohibited"
    );
    let staging = cfg.work_dir.join("epw-state/imports/canonical");
    let (points, responses) = inspect(cfg, &s, &staging)?;
    let fingerprints = Fingerprints {
        structure_sha256: store::hash_file(&cfg.structure)?,
        pseudo_sha256: cfg
            .pseudos
            .iter()
            .map(|(n, p)| Ok((n.clone(), store::hash_file(p)?)))
            .collect::<Result<_>>()?,
        program_sha256: BTreeMap::new(),
    };
    let scf = qe_data::scf::ScfResult {
        energy_ry: qe_data::scf::parse_scf_energy(&fs::read_to_string(layout::path(
            &cfg.work_dir,
            "scf/scf.out",
        ))?)?,
        snapshot_dir: layout::path(&cfg.work_dir, "scf/snapshot"),
        attempt_count: 1,
        success_count: 1,
    };
    let mut m = qe_data::scheduler::publish_data(
        &stages::scientific_key(cfg, Step::Phonon)?,
        fingerprints,
        qe,
        &cfg.work_dir,
        scf,
        qe_data::nscf::verified_states(cfg, &cfg.work_dir)?,
        points,
        responses,
    )?;
    // 保留 phsave 全部逐扰动元数据；已由归集器规范化的 control/patterns 不重复覆盖。
    let phsave = format!("{}.phsave", s.prefix);
    for file in &s.files {
        if file.relative_path.starts_with(&phsave) {
            let target = layout::path(&cfg.work_dir, "qe-data/save").join(&file.relative_path);
            if !m
                .artifacts
                .iter()
                .any(|a| a.relative_path == target.strip_prefix(&cfg.work_dir).unwrap())
            {
                fs::create_dir_all(target.parent().unwrap())?;
                qe_data::collect_ph::independent_copy(&s.root.join(&file.relative_path), &target)?;
            }
            if !m
                .artifacts
                .iter()
                .any(|a| a.relative_path == target.strip_prefix(&cfg.work_dir).unwrap())
            {
                ensure!(
                    store::hash_file(&target)? == file.sha256,
                    "input_conflict: phsave copy corrupted"
                );
                m.artifacts.push(qe_data::manifest::Artifact {
                    role: format!("external_phsave_{}", file.relative_path.display()),
                    relative_path: target.strip_prefix(&cfg.work_dir)?.into(),
                    byte_size: file.byte_size,
                    sha256: file.sha256.clone(),
                    producer_stage: "phonon".into(),
                    input_digest: store::hash_file(&binding(cfg))?,
                    format_version: "1".into(),
                });
            }
        }
    }
    ensure!(
        s == source(&s.root)?,
        "input_conflict: source file set or content changed during import"
    );
    let report = cfg.work_dir.join("epw-state/imports/external-source.json");
    store::write(
        &report,
        &json!({"schema_version":1,"source":s,"conversion":"QE text to io_dyn_mat XML; complete q stars; Fortran column-major complex matrices; AMU_RY to amu","not_checked":["historical SCF pseudopotentials","historical SCF cutoffs","historical smooth FFT dimensions absent in source"],"fft_check":"current SCF/NSCF dense and smooth grids agree; dvscf byte count agrees with current dense FFT and perturbation count"}),
    )?;
    m.artifacts.push(qe_data::manifest::Artifact {
        role: "external_source".into(),
        relative_path: report.strip_prefix(&cfg.work_dir)?.into(),
        byte_size: fs::metadata(&report)?.len(),
        sha256: store::hash_file(&report)?,
        producer_stage: "phonon".into(),
        input_digest: store::hash_file(&binding(cfg))?,
        format_version: "1".into(),
    });
    m.verify()?;

    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Read-only continuous diagnostics for visible only for old configurations"]
    fn historical_electronic_binding_diagnostic() {
        let path = PathBuf::from(std::env::var("EPWFLOW_IMPORT_DIAGNOSTIC").unwrap());
        let cfg = Config::load_nested(&path).unwrap();
        for s in [Step::Scf, Step::Nscf] {
            let c = super::super::readable_state::read(&cfg.work_dir, s).unwrap();
            eprintln!(
                "{} log={:?} current={} reusable={:?} validate={:?} params={}",
                s.name(),
                c,
                stages::scientific_key(&cfg, s).unwrap(),
                stages::reusable(&cfg, s),
                super::super::workspace_science::validate(&cfg, s),
                stages::parameters(&cfg, s).unwrap()
            );
        }
        if let Ok(p) = std::env::var("EPWFLOW_IMPORT_ORIGINAL") {
            let original = Config::load(Path::new(&p)).unwrap();
            eprintln!(
                "original scf={}",
                stages::parameters(&original, Step::Scf).unwrap()
            );
        }
    }
}
