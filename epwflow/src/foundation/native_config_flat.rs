//! 用户只填写同层科学调用；程序负责输入段归属和有限的自动绑定。
use super::{
    at, json, put, section_parameters, Document, Value, COARSE, ELECTRONIC, FINE, NSCF, SCF, SUPER,
    WANNIER,
};
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

#[path = "native_config_groups.rs"]
mod groups;

struct Call {
    name: String,
    path: Vec<String>,
    sections: Vec<&'static str>,
    derived: Vec<&'static str>,
}
impl Call {
    fn new(name: &str, path: &[&str], sections: &[&'static str], derived: &[&'static str]) -> Self {
        Self {
            name: name.into(),
            path: path.iter().map(|s| (*s).into()).collect(),
            sections: sections.to_vec(),
            derived: derived.to_vec(),
        }
    }
    fn refs(&self) -> Vec<&str> {
        self.path.iter().map(String::as_str).collect()
    }
    fn target(&self, key: &str) -> Result<Vec<&str>> {
        let section = self
            .sections
            .iter()
            .filter(|section| {
                section_parameters(section)
                    .is_some_and(|list| list.split_whitespace().any(|k| k == key))
                    || (**section == "cards" && ["K_POINTS", "ATOMIC_SPECIES"].contains(&key))
            })
            .copied()
            .collect::<Vec<_>>();
        ensure!(
            section.len() == 1,
            "configuration: stages.{}.{key}: unknown, ambiguous or unsupported native parameter",
            self.name
        );
        let mut target = self.refs();
        if section[0] != "cards" && !["wannier90", "postw90"].contains(&section[0]) {
            target.push(section[0]);
        }
        Ok(target)
    }
}

fn calls(ph: &str, spectra: &[String]) -> Vec<Call> {
    let pw = ["CONTROL", "SYSTEM", "ELECTRONS", "cards"];
    let pw_derived = ["outdir", "pseudo_dir", "ibrav", "nat", "ntyp"];
    let epw_derived = ["outdir", "dvscf_dir", "fermi_energy"];
    let mut calls = vec![
        Call::new(SCF, &[SCF, "pw"], &pw, &pw_derived),
        Call::new(NSCF, &[NSCF, "pw"], &pw, &pw_derived),
        Call::new(
            "3.qe-bands",
            &[ELECTRONIC, "qe_eband", "pw"],
            &pw,
            &pw_derived,
        ),
        Call::new(
            "3.qe-bands-post",
            &[ELECTRONIC, "qe_eband", "bands"],
            &["BANDS"],
            &["outdir", "filband"],
        ),
        Call::new(
            "3.qe-dos",
            &[ELECTRONIC, "qe_eledos", "dos"],
            &["DOS"],
            &["outdir", "fildos"],
        ),
        Call::new(
            "3.qe-pdos",
            &[ELECTRONIC, "qe_eledos", "projwfc"],
            &["PROJWFC"],
            &["outdir", "filpdos"],
        ),
        Call::new(ph, &[ph, "ph"], &["INPUTPH"], &["outdir", "fildyn"]),
        Call::new("4.qe-q2r", &[ph, "q2r"], &["INPUT"], &["fildyn", "flfrc"]),
        Call::new(
            "4.qe-dispersion",
            &[ph, "dispersion", "matdyn"],
            &["INPUT"],
            &["flfrc", "flfrq"],
        ),
        Call::new(
            "4.qe-phdos",
            &[ph, "dos", "matdyn"],
            &["INPUT"],
            &["flfrc", "fldos"],
        ),
        Call::new(
            WANNIER,
            &[WANNIER, "epw"],
            &["INPUTEPW"],
            &[
                "outdir",
                "dvscf_dir",
                "fermi_energy",
                "proj",
                "bands_skipped",
                "wdata",
                "dis_win_min",
                "dis_win_max",
                "dis_froz_min",
                "dis_froz_max",
            ],
        ),
        Call::new("5.wannier90", &[WANNIER, "wannier90"], &["wannier90"], &[]),
        Call::new(
            "5.postw90-dos",
            &[WANNIER, "electron_dos", "postw90"],
            &["postw90"],
            &["dos_energy_min", "dos_energy_max", "fermi_energy"],
        ),
        Call::new(COARSE, &[COARSE, "epw"], &["INPUTEPW"], &epw_derived),
        Call::new(
            "6.epw-comparison",
            &[COARSE, "comparison", "epw"],
            &["INPUTEPW"],
            &epw_derived,
        ),
        Call::new(FINE, &[FINE, "epw"], &["INPUTEPW"], &epw_derived),
        Call::new(
            "8.epw-iso",
            &[SUPER, "iso", "epw"],
            &["INPUTEPW"],
            &["outdir", "fila2f"],
        ),
        Call::new(
            "8.epw-aniso",
            &[SUPER, "aniso", "epw"],
            &["INPUTEPW"],
            &["outdir", "fila2f"],
        ),
    ];
    if !ph.ends_with("-nosplit") {
        let mut call = Call::new(
            &format!("{ph}-q"),
            &[ph, "q", "ph"],
            &["INPUTPH"],
            &["outdir", "fildyn"],
        );
        if ph.ends_with("-index") {
            call.derived.extend(["start_q", "last_q"]);
        }
        calls.push(call);
    }
    for name in spectra {
        calls.push(Call::new(
            &format!("{FINE}-{name}"),
            &[FINE, "spectra", name, "epw"],
            &["INPUTEPW"],
            &epw_derived,
        ));
    }
    calls
}

fn workflow_names(ph: &str) -> Vec<(&str, &str)> {
    vec![
        (SCF, SCF),
        (NSCF, NSCF),
        ("3.qe-bands", ELECTRONIC),
        (ph, ph),
        (WANNIER, WANNIER),
        (COARSE, COARSE),
        (FINE, FINE),
        (SUPER, SUPER),
    ]
}

fn rule(call: &Call, key: &str) -> &'static str {
    if call.name == "5.postw90-dos" {
        return "Actual Wannier outer window and bound NSCF Fermi energy, absolute eV";
    }
    match key {
        "ibrav" | "nat" | "ntyp" => "CELL_PARAMETERS and ATOMIC_POSITIONS generated from the bound POSCAR",
        "fermi_energy" => "Bound stage-2 NSCF XML Fermi energy, eV",
        "proj" | "bands_skipped" | "wdata" | "dis_win_min" | "dis_win_max" | "dis_froz_min"
        | "dis_froz_max" => "Native Wannier90 parameters or automatic windows bound to NSCF energies; passed to the embedded library",
        "gap_edge" | "tc_linear" => "Current fixed-temperature and initial-gap requests for Tc search",
        "start_q" | "last_q" => "Current index in the validated q-point plan",
        "outdir" | "pseudo_dir" => "Current task working directory and validated bound data",
        _ => "Current task validated data copy and deterministic output filename",
    }
}

fn phonon_name(d: &Document, flat: bool) -> Result<String> {
    let candidates = [
        "4.qe-phonon-nosplit",
        "4.qe-phonon-index",
        "4.qe-phonon-coords",
    ];
    let names = candidates
        .into_iter()
        .filter(|name| d.stages.contains_key(*name))
        .collect::<Vec<_>>();
    ensure!(
        names.len() == 1,
        "configuration: stages: exactly one 4.qe-phonon-nosplit/index/coords stage is required{}",
        if flat {
            ", parameters are specified directly in this section"
        } else {
            ""
        }
    );
    Ok(names[0].into())
}

pub(super) fn expand(mut d: Document) -> Result<Document> {
    // 历史冻结配置仅在读取边界丢弃已删除的比较专用选项，不能恢复执行。
    for name in ["6.qe-q2r", "6.qe-matdyn"] { d.stages.remove(name); }
    if let Some(stage) = d.stages.get_mut(COARSE).and_then(Value::as_object_mut) {
        stage.remove("q2r"); stage.remove("matdyn");
        if let Some(v) = stage.get_mut("comparison").and_then(Value::as_object_mut) {
            v.remove("q2r"); v.remove("matdyn");
        }
        if let Some(v) = stage.get_mut("workflow").and_then(Value::as_object_mut) { v.remove("onsite_asr"); }
    }
    if let Some(v) = d.workflow.get_mut(COARSE).and_then(Value::as_object_mut) { v.remove("onsite_asr"); }

    d.eight_stages = d.stages.contains_key(ELECTRONIC) || d.stages.contains_key(SUPER);
    let grouped = d.eight_stages;
    let ph = phonon_name(&d, false).unwrap_or_default();
    let result = if grouped { groups::ungroup(d).and_then(expand_calls) } else { expand_calls(d) };
    if grouped {
        result.map_err(|e| anyhow::anyhow!(groups::message(&format!("{e:#}"), &ph)))
    } else {
        result
    }
}

fn expand_calls(mut d: Document) -> Result<Document> {
    // 未指定时限制为 500 轮；EPW 与内嵌 Wannier90 使用同一次拟合的迭代数。
    if let Some(wannier) = d.stages.get_mut("5.wannier90").and_then(Value::as_object_mut) {
        wannier.entry("num_iter").or_insert(json!(500));
    }
    let iterations = d.stages.get("5.wannier90").and_then(|w| w.get("num_iter")).cloned();
    if let (Some(iterations), Some(epw)) = (iterations, d.stages.get_mut(WANNIER).and_then(Value::as_object_mut)) {
        epw.entry("num_iter").or_insert(iterations);
    }
    let ph = phonon_name(&d, true)?;
    let spectra = d
        .stages
        .keys()
        .filter_map(|s| s.strip_prefix(&format!("{FINE}-")).map(str::to_owned))
        .collect::<Vec<_>>();
    for name in &spectra {
        ensure!(
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "configuration: stages.{FINE}-{name}: invalid spectrum task name"
        );
    }
    let calls = calls(&ph, &spectra);
    let mut expanded = json!({});
    for (public, internal) in workflow_names(&ph) {
        let settings = d.workflow.remove(public).unwrap_or_else(|| json!({}));
        ensure!(
            settings.is_object(),
            "configuration: workflow.{public}: must be a table"
        );
        ensure!(
            settings.get("derived").is_none(),
            "configuration: workflow.{public}.derived: automatic bindings are registered internally; no configuration needed"
        );
        put(&mut expanded, &[internal, "workflow"], settings);
        put(&mut expanded, &[internal, "workflow", "derived"], json!({}));
    }
    ensure!(
        d.workflow.is_empty(),
        "configuration: workflow: unknown workflow section {:?}",
        d.workflow.keys().collect::<Vec<_>>()
    );
    let tc = expanded[SUPER]["workflow"]["task"] == "tc";
    for (name, settings) in &d.stages {
        let call = calls.iter().find(|c| &c.name == name).with_context(|| {
            format!("configuration: stages.{name}: unknown scientific call; use an independent section at the same level")
        })?;
        let settings = settings
            .as_object()
            .with_context(|| format!("configuration: stages.{name}: must be a table"))?;
        for section in &call.sections {
            if *section != "cards" {
                let mut target = call.refs();
                if !["wannier90", "postw90"].contains(section) {
                    target.push(section);
                }
                put(&mut expanded, &target, json!({}));
            }
        }
        for (key, value) in settings {
            ensure!(!value.is_object(), "configuration: stages.{name}.{key}: nested configuration is no longer supported; put native parameters directly in independent sections at the same level");
            let mut target = call.target(key)?;
            target.push(key);
            put(&mut expanded, &target, value.clone());
        }
        let mut keys = call.derived.clone();
        if tc && name.starts_with("8.epw-") {
            keys.extend(["gap_edge", "tc_linear"]);
        }
        for key in keys {
            let target = call.target(key)?;
            let location = target[1..]
                .iter()
                .copied()
                .chain([key])
                .collect::<Vec<_>>()
                .join(".");
            put(
                &mut expanded,
                &[&call.path[0], "workflow", "derived", &location],
                json!(super::super::legacy_language::rule_identity(rule(call, key))),
            );
        }
    }
    d.stages = serde_json::from_value(expanded)?;
    Ok(d)
}

pub(super) fn flatten(mut d: Document) -> Result<Document> {
    let ph = phonon_name(&d, false)?;
    let values = serde_json::to_value(&d.stages)?;
    let spectra = values[FINE]
        .get("spectra")
        .and_then(Value::as_object)
        .map(|o| o.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let mut flat = BTreeMap::new();
    for call in calls(&ph, &spectra) {
        let Ok(node) = at(&values, &call.refs()) else {
            continue;
        };
        let mut settings = serde_json::Map::new();
        for section in &call.sections {
            let group = if ["cards", "wannier90", "postw90"].contains(section) {
                node
            } else {
                &node[*section]
            };
            if let Some(group) = group.as_object() {
                for (key, value) in group {
                    if value.is_object() {
                        continue;
                    }
                    if *section == "cards"
                        && !["K_POINTS", "ATOMIC_SPECIES"].contains(&key.as_str())
                    {
                        continue;
                    }
                    ensure!(
                        settings.insert(key.clone(), value.clone()).is_none(),
                        "configuration: stages.{}.{key}: duplicate native parameter",
                        call.name
                    );
                }
            }
        }
        flat.insert(call.name, Value::Object(settings));
    }
    for (public, internal) in workflow_names(&ph) {
        let mut settings = values[internal]["workflow"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        settings.remove("derived");
        if !settings.is_empty() {
            d.workflow.insert(public.into(), Value::Object(settings));
        }
    }
    d.stages = flat;
    if d.eight_stages { groups::group(d) } else { Ok(d) }
}

/// 错误与参数回执使用同一外部位置映射。
pub(super) fn location(path: &str) -> String {
    for ph in [
        "4.qe-phonon-nosplit",
        "4.qe-phonon-index",
        "4.qe-phonon-coords",
    ] {
        let spectra = path
            .strip_prefix(&format!("stages.{FINE}.spectra."))
            .and_then(|s| s.split('.').next())
            .map(|s| vec![s.to_owned()])
            .unwrap_or_default();
        let mut names = Vec::new();
        for call in calls(ph, &spectra) {
            let base = format!("stages.{}", call.path.join("."));
            for section in &call.sections {
                let prefix = if ["cards", "wannier90", "postw90"].contains(section) {
                    base.clone()
                } else {
                    format!("{base}.{section}")
                };
                names.push((prefix, format!("stages.{}", call.name)));
            }
            names.push((base, format!("stages.{}", call.name)));
        }
        for (public, internal) in workflow_names(ph) {
            names.push((
                format!("stages.{internal}.workflow"),
                format!("workflow.{public}"),
            ));
        }
        names.sort_by_key(|(key, _)| std::cmp::Reverse(key.len()));
        for (prefix, public) in names {
            if path == prefix {
                return public;
            }
            if let Some(rest) = path.strip_prefix(&(prefix + ".")) {
                return format!("{public}.{rest}");
            }
        }
    }
    path.into()
}

pub(super) fn message(text: &str) -> String {
    // 诊断中的位置被标点或值隔开；参数本身不包含这些字符。
    let re = regex::Regex::new(r"stages\.[A-Za-z0-9_.-]+").expect("Fixed position expression");
    re.replace_all(text, |c: &regex::Captures<'_>| location(&c[0]))
        .replace("workflow.derived", "Internal derivation rule")
}

pub(super) fn format_message(text: &str, grouped: bool, ph: &str) -> String {
    let text = message(text);
    if grouped { groups::message(&text, ph) } else { text }
}
