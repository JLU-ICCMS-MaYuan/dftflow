//! 用户只填写同层科学调用；程序负责输入段归属和有限的自动绑定。
use super::{
    at, json, put, section_parameters, Document, Value, COARSE, ELECTRONIC, FINE, NSCF, SCF, SUPER,
    WANNIER,
};
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

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
            "configuration: stages.{}.{key}: 未知、歧义或当前流程不支持的原生参数",
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
            "6.qe-q2r",
            &[COARSE, "comparison", "q2r"],
            &["INPUT"],
            &["fildyn", "flfrc"],
        ),
        Call::new(
            "6.qe-matdyn",
            &[COARSE, "comparison", "matdyn"],
            &["INPUT"],
            &["flfrc", "flfrq"],
        ),
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
        return "实际 Wannier 外窗和绑定 NSCF 费米能，绝对 eV";
    }
    match key {
        "ibrav" | "nat" | "ntyp" => "已绑定 POSCAR 生成 CELL_PARAMETERS 和 ATOMIC_POSITIONS",
        "fermi_energy" => "已绑定第二步 NSCF XML 的费米能，eV",
        "proj" | "bands_skipped" | "wdata" | "dis_win_min" | "dis_win_max" | "dis_froz_min"
        | "dis_froz_max" => "本阶段 Wannier90 原生参数或绑定 NSCF 能量的自动选窗；受控传给内嵌库",
        "gap_edge" | "tc_linear" => "当前 Tc 搜索的已固定温度及初值请求",
        "start_q" | "last_q" => "已核验 q 点计划中的当前序号",
        "outdir" | "pseudo_dir" => "当前任务工作目录和已核验绑定数据",
        _ => "当前任务已核验的数据副本和确定的输出文件名",
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
        "configuration: stages: 必须且只能设置一种 4.qe-phonon-nosplit/index/coords 阶段{}",
        if flat {
            "，参数直接写在该段内"
        } else {
            ""
        }
    );
    Ok(names[0].into())
}

pub(super) fn expand(mut d: Document) -> Result<Document> {
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
            "configuration: stages.{FINE}-{name}: 非法谱任务名"
        );
    }
    let calls = calls(&ph, &spectra);
    let mut expanded = json!({});
    for (public, internal) in workflow_names(&ph) {
        let settings = d.workflow.remove(public).unwrap_or_else(|| json!({}));
        ensure!(
            settings.is_object(),
            "configuration: workflow.{public}: 必须是表"
        );
        ensure!(
            settings.get("derived").is_none(),
            "configuration: workflow.{public}.derived: 自动绑定由程序内部登记，无需配置"
        );
        put(&mut expanded, &[internal, "workflow"], settings);
        put(&mut expanded, &[internal, "workflow", "derived"], json!({}));
    }
    ensure!(
        d.workflow.is_empty(),
        "configuration: workflow: 未知工作流配置段 {:?}",
        d.workflow.keys().collect::<Vec<_>>()
    );
    let tc = expanded[SUPER]["workflow"]["task"] == "tc";
    for (name, settings) in &d.stages {
        let call = calls.iter().find(|c| &c.name == name).with_context(|| {
            format!("configuration: stages.{name}: 未知科学调用；请使用同层独立配置段")
        })?;
        let settings = settings
            .as_object()
            .with_context(|| format!("configuration: stages.{name}: 必须是表"))?;
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
            ensure!(!value.is_object(), "configuration: stages.{name}.{key}: 不再支持多层配置，请把原生参数直接写在同层独立配置段中");
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
                json!(rule(call, key)),
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
                        "configuration: stages.{}.{key}: 原生参数重复",
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
    Ok(d)
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
    let re = regex::Regex::new(r"stages\.[A-Za-z0-9_.-]+").expect("固定位置表达式");
    re.replace_all(text, |c: &regex::Captures<'_>| location(&c[0]))
        .replace("workflow.derived", "内部推导规则")
}

pub(super) fn bindings(values: &Value, task: &[&str]) -> Result<BTreeMap<String, Value>> {
    let derived = at(values, &[task[0], "workflow", "derived"])?
        .as_object()
        .context("内部绑定表缺失")?;
    let prefix = format!("{}.", task[1..].join("."));
    let mut bindings = BTreeMap::new();
    for (key, rule) in derived {
        if key.starts_with(&prefix) {
            bindings.insert(location(&format!("stages.{}.{key}", task[0])), rule.clone());
        }
    }
    Ok(bindings)
}
