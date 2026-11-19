//! 公共字段只回填适用调用；数组在配置边界展开，算法仍消费既有原生字段。
use super::{Call, Document, Value};
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

const COMMON: [&str; 8] = [
    "ATOMIC_SPECIES",
    "prefix",
    "ecutwfc",
    "ecutrho",
    "Emin",
    "Emax",
    "DeltaE",
    "fermi_source",
];
const GRIDS: [&str; 4] = ["nk", "nq", "nkf", "nqf"];

fn species(value: &Value, path: &str) -> Result<Value> {
    if value.is_string() {
        return Ok(value.clone());
    }
    let rows = value.as_array().with_context(|| {
        format!("configuration: {path}: species triplets or native card text required")
    })?;
    ensure!(
        !rows.is_empty(),
        "configuration: {path}: species must not be empty"
    );
    let mut names = std::collections::BTreeSet::new();
    let mut result = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let row = row.as_array().filter(|v| v.len() == 3).with_context(|| {
            format!("configuration: {path}[{i}]: expected [element, mass, pseudo_file]")
        })?;
        let element = row[0]
            .as_str()
            .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic()))
            .with_context(|| format!("configuration: {path}[{i}]: invalid element"))?;
        let mass = row[1]
            .as_f64()
            .filter(|v| v.is_finite() && *v > 0.)
            .with_context(|| {
                format!("configuration: {path}[{i}]: mass must be finite and positive")
            })?;
        let file = row[2]
            .as_str()
            .filter(|s| {
                !s.is_empty()
                    && !s
                        .chars()
                        .any(|c| c.is_whitespace() || ['\0', '\'', '"'].contains(&c))
            })
            .with_context(|| {
                format!(
                    "configuration: {path}[{i}]: nonempty native pseudopotential filename required"
                )
            })?;
        ensure!(
            names.insert(element),
            "configuration: {path}[{i}]: duplicate element {element}"
        );
        result.push(format!("{element} {mass} {file}"));
    }
    Ok(Value::String(result.join("\n")))
}

fn common(base: &BTreeMap<String, Value>) -> Result<BTreeMap<String, Value>> {
    let mut result = BTreeMap::new();
    for (key, value) in base {
        ensure!(COMMON.contains(&key.as_str()), "configuration: baseparams.{key}: only ATOMIC_SPECIES, prefix, ecutwfc, ecutrho, Emin, Emax, DeltaE and fermi_source are allowed; grids belong to their stage");
        let path = format!("baseparams.{key}");
        let normalized = if key == "ATOMIC_SPECIES" {
            species(value, &path)?
        } else if key == "fermi_source" {
            fermi_source(value, &path)?;
            value.clone()
        } else {
            super::super::validate_parameter(key, value, &path)?;
            if key == "prefix" {
                ensure!(
                    value.as_str().is_some_and(|s| !s.trim().is_empty()),
                    "configuration: {path}: empty prefix"
                );
            } else {
                let n = value
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .with_context(|| format!("configuration: {path}: finite number required"))?;
                ensure!(
                    !["ecutwfc", "ecutrho", "DeltaE"].contains(&key.as_str()) || n > 0.,
                    "configuration: {path}: must be positive"
                );
            }
            value.clone()
        };
        result.insert(key.clone(), normalized);
    }
    Ok(result)
}

fn fermi_source(value: &Value, path: &str) -> Result<()> {
    serde_json::from_value::<super::super::config::FermiSource>(value.clone())
        .with_context(|| format!("configuration: {path}: expected f2 or fine_fixed_electrons"))?;
    Ok(())
}

pub(super) fn expand(document: &mut Document, calls: &[Call]) -> Result<()> {
    let base = common(&document.baseparams)?;
    for name in [super::WANNIER, super::COARSE, super::FINE] {
        let path = format!("workflow.{name}.fermi_source");
        let workflow = document
            .workflow
            .entry(name.into())
            .or_insert_with(|| serde_json::json!({}));
        let workflow = workflow
            .as_object_mut()
            .with_context(|| format!("configuration: workflow.{name}: must be a table"))?;
        if !workflow.contains_key("fermi_source") {
            if let Some(value) = base.get("fermi_source") {
                workflow.insert("fermi_source".into(), value.clone());
                document
                    .input_locations
                    .insert(path.clone(), "baseparams.fermi_source".into());
            }
        }
        let value = workflow.get("fermi_source")
            .with_context(|| format!("configuration: {path}: required parameter missing; set stage value or baseparams.fermi_source"))?;
        fermi_source(value, &path)?;
    }
    for (name, settings) in &mut document.stages {
        let Some(call) = calls.iter().find(|c| c.name == *name) else {
            continue;
        };
        let settings = settings
            .as_object_mut()
            .with_context(|| format!("configuration: stages.{name}: must be a table"))?;
        for (key, value) in &base {
            if call.target(key).is_ok() && !settings.contains_key(key) {
                settings.insert(key.clone(), value.clone());
                document
                    .input_locations
                    .insert(format!("stages.{name}.{key}"), format!("baseparams.{key}"));
            }
        }
        if let Some(value) = settings.get_mut("ATOMIC_SPECIES") {
            let path = format!("stages.{name}.ATOMIC_SPECIES");
            if value.is_array() {
                document.input_arrays.insert(path.clone(), value.clone());
            }
            *value = species(value, &path)?;
        }
        for grid in GRIDS {
            let Some(value) = settings.remove(grid) else {
                continue;
            };
            let path = format!("stages.{name}.{grid}");
            let dimensions = value
                .as_array()
                .filter(|a| {
                    a.len() == 3
                        && a.iter()
                            .all(|n| n.as_u64().is_some_and(|n| n > 0 && n <= u32::MAX as u64))
                })
                .with_context(|| {
                    format!("configuration: {path}: exactly three positive integers required")
                })?;
            for (i, n) in dimensions.iter().enumerate() {
                let key = format!("{grid}{}", i + 1);
                call.target(&key).with_context(|| {
                    format!("configuration: {path}: grid is not supported by this call")
                })?;
                ensure!(
                    !settings.contains_key(&key),
                    "configuration: {path}: cannot mix array with stages.{name}.{key}"
                );
                settings.insert(key.clone(), n.clone());
                document
                    .input_locations
                    .insert(format!("stages.{name}.{key}"), path.clone());
            }
            document.input_arrays.insert(path, value);
        }
    }
    Ok(())
}

pub(super) fn compact(document: &mut Document) -> Result<()> {
    let base = common(&document.baseparams)?;
    for name in [super::WANNIER, super::COARSE, super::FINE] {
        let path = format!("workflow.{name}.fermi_source");
        if document
            .input_locations
            .get(&path)
            .is_some_and(|p| p == "baseparams.fermi_source")
        {
            if let Some(workflow) = document
                .workflow
                .get_mut(name)
                .and_then(Value::as_object_mut)
            {
                if workflow.get("fermi_source") == base.get("fermi_source") {
                    workflow.remove("fermi_source");
                }
            }
        }
    }
    for (name, settings) in &mut document.stages {
        let settings = settings
            .as_object_mut()
            .context("configuration: stage must be a table")?;
        for (key, value) in &base {
            let path = format!("stages.{name}.{key}");
            if document.input_locations.get(&path) == Some(&format!("baseparams.{key}"))
                && settings.get(key) == Some(value)
            {
                settings.remove(key);
            }
        }
        for grid in GRIDS {
            if !document
                .input_arrays
                .contains_key(&format!("stages.{name}.{grid}"))
            {
                continue;
            }
            let keys = (1..=3).map(|i| format!("{grid}{i}")).collect::<Vec<_>>();
            if let Some(values) = keys
                .iter()
                .map(|k| settings.get(k).cloned())
                .collect::<Option<Vec<_>>>()
            {
                for key in keys {
                    settings.remove(&key);
                }
                settings.insert(grid.into(), Value::Array(values));
            }
        }
        let path = format!("stages.{name}.ATOMIC_SPECIES");
        if let Some(original) = document.input_arrays.get(&path) {
            if settings.get("ATOMIC_SPECIES") == Some(&species(original, &path)?) {
                settings.insert("ATOMIC_SPECIES".into(), original.clone());
            }
        }
    }
    Ok(())
}
