//! 旧记录语言与内部身份兼容；正常输出直接使用英文，不运行翻译服务。
use regex::Regex;
use std::{borrow::Cow, collections::BTreeMap, sync::LazyLock};

pub(super) const INPUT_SOURCE_PREFIX: &str = "! 参数来源 ";
pub(super) const FOOTER: &str = "记录结束\n";
pub(super) const JOB_HEADER: &str = "子任务\tSlurm 作业号\n";
static PHRASES: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("legacy-language.json"))
        .expect("invalid legacy language catalog")
});
static HOLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{[^{}]*\}").unwrap());
struct Template {
    pattern: Regex,
    parts: Vec<String>,
}
static TEMPLATES: LazyLock<Vec<Template>> = LazyLock::new(|| {
    let mut phrases: Vec<_> = PHRASES.iter().filter(|(old, _)| HOLE.is_match(old)).collect();
    // 优先匹配更具体的旧文案，避免短格式吞掉详细错误。
    phrases.sort_by_key(|(old, _)| std::cmp::Reverse(old.len()));
    phrases.into_iter().filter_map(|(old, english)| {
        let source: Vec<_> = HOLE.split(old).collect();
        let parts: Vec<String> = HOLE.split(english).map(str::to_owned).collect();
        if source.len() != parts.len() { return None; }
        let pattern = format!("(?s)^{}$", source.into_iter().map(regex::escape).collect::<Vec<_>>().join("(.*?)"));
        Some(Template { pattern: Regex::new(&pattern).expect("invalid legacy phrase pattern"), parts })
    }).collect()
});

pub(super) fn translate(text: &str) -> Cow<'_, str> {
    if !text.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)) {
        return Cow::Borrowed(text);
    }
    if let Some(english) = PHRASES.get(text) {
        return Cow::Borrowed(english);
    }
    for (old, english) in PHRASES.iter().filter(|(old, _)| old.ends_with(" · ")) {
        if let Some(rest) = text.strip_prefix(old.as_str()) {
            return Cow::Owned(format!("{english}{rest}"));
        }
    }
    for template in TEMPLATES.iter() {
        if let Some(captures) = template.pattern.captures(text) {
            let mut result = template.parts[0].clone();
            for (i, part) in template.parts.iter().enumerate().skip(1) {
                // 占位符捕获的路径、参数、数值和原始错误原样保留。
                result.push_str(&captures[i]);
                result.push_str(part);
            }
            return Cow::Owned(result);
        }
    }
    // 未登记的用户原文不猜译，也不丢弃。
    Cow::Borrowed(text)
}

pub(super) fn value<'a>(key: &str, text: &'a str) -> Cow<'a, str> {
    if matches!(key, "Original directory" | "Current input" | "Current input version"
        | "Calculation configuration" | "Configuration used" | "Calculation directory"
        | "Script" | "Command" | "Run command" | "Scientific output" | "Comparison path"
        | "Configuration" | "Original SHA-256")
        || key.starts_with("Scientific input") || key.starts_with("Artifact")
    {
        return Cow::Borrowed(text);
    }
    translate(text)
}

/// 旧的内部推导说明参与既有科学指纹；保留其身份值，不作为输出文案。
pub(super) fn rule_identity(english: &str) -> &str {
    PHRASES.iter().find_map(|(old, translated)| {
        (translated == english).then_some(old.as_str())
    }).unwrap_or(english)
}
