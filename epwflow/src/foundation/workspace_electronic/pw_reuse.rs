//! 旧 PW 输入只允许赝势目录迁移；以实际保存的 UPF 内容证明兼容。
use super::*;
use std::collections::{BTreeMap, BTreeSet};

// 保留字符串中的空格、! 和 /；只忽略字符串外的空白与注释。
fn tokens(input: &str) -> Result<Vec<String>> {
    let bytes = input.as_bytes();
    let mut result = vec![];
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        match bytes[i] {
            b if b.is_ascii_whitespace() => i += 1,
            b'!' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            quote @ (b'\'' | b'"') => {
                i += 1;
                loop {
                    ensure!(i < bytes.len(), "input_conflict: PW 输入字符串未闭合");
                    if bytes[i] == quote {
                        i += 1;
                        if i < bytes.len() && bytes[i] == quote {
                            i += 1;
                        } else {
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                result.push(input[start..i].into());
            }
            b'&' | b'/' | b'=' | b',' => {
                i += 1;
                result.push(input[start..i].into());
            }
            _ => {
                while i < bytes.len()
                    && !bytes[i].is_ascii_whitespace()
                    && !b"!\'\"&/=,".contains(&bytes[i])
                {
                    i += 1;
                }
                result.push(input[start..i].into());
            }
        }
    }
    Ok(result)
}

fn comparable(input: &str) -> Result<Vec<String>> {
    let mut tokens = tokens(input)?;
    let mut control = false;
    let mut sections = 0;
    let mut fields = 0;
    for i in 0..tokens.len() {
        if tokens[i] == "&" {
            control = tokens
                .get(i + 1)
                .is_some_and(|s| s.eq_ignore_ascii_case("control"));
            sections += usize::from(control);
        } else if tokens[i] == "/" {
            control = false;
        } else if control && tokens[i].eq_ignore_ascii_case("pseudo_dir") {
            ensure!(
                tokens.get(i + 1).map(String::as_str) == Some("=")
                    && tokens
                        .get(i + 2)
                        .is_some_and(|s| s.starts_with(['\'', '"'])),
                "input_conflict: CONTROL.pseudo_dir 必须为字符串"
            );
            tokens[i + 2] = "'<已核验的赝势目录>'".into();
            fields += 1;
        }
    }
    ensure!(
        sections == 1 && fields == 1,
        "input_conflict: PW 输入必须只有一份 CONTROL.pseudo_dir"
    );
    Ok(tokens)
}

pub(super) fn validate(
    actual: &str,
    expected: &str,
    save: &Path,
    pseudos: &BTreeMap<String, PathBuf>,
) -> Result<()> {
    ensure!(
        comparable(actual)? == comparable(expected)?,
        "input_conflict: 已有 PW 能带输入与当前配置不符（仅允许已核验的赝势目录迁移）"
    );
    let raw = fs::read_to_string(save.join("data-file-schema.xml"))?;
    let doc = roxmltree::Document::parse(&raw)?;
    let species = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("output"))
        .and_then(|n| n.children().find(|n| n.has_tag_name("atomic_species")))
        .context("data_invalid: PW 保存 XML 缺 output.atomic_species，无法核验赝势迁移")?;
    let mut seen = BTreeSet::new();
    for node in species.children().filter(|n| n.has_tag_name("species")) {
        let element = node
            .attribute("name")
            .context("data_invalid: XML 赝势缺元素")?;
        let configured = pseudos
            .get(element)
            .context("input_conflict: XML 赝势元素与当前配置不符")?;
        let filename = node
            .children()
            .find(|n| n.has_tag_name("pseudo_file"))
            .and_then(|n| n.text())
            .context("data_invalid: XML 缺 pseudo_file")?
            .trim();
        ensure!(
            seen.insert(element)
                && configured.file_name().and_then(|s| s.to_str()) == Some(filename),
            "input_conflict: XML 赝势文件映射与当前配置不符：{element}"
        );
        let saved = save.join(filename);
        save::nonempty(&saved).with_context(|| {
            format!("data_invalid: 保存的 {element} 赝势缺失，不能验证目录迁移")
        })?;
        ensure!(
            store::hash_file(&saved)? == store::hash_file(configured)?,
            "input_conflict: 保存的 {element} 赝势内容与当前配置不符，拒绝复用 PW 能带"
        );
    }
    ensure!(
        seen.len() == pseudos.len(),
        "input_conflict: XML 赝势元素集合与当前配置不符"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_control_pseudo_directory_is_ignored_with_quotes_and_comments_preserved() {
        let old = "&CONTROL\n pseudo_dir='/deleted 目录/it''s!pp', ! old\n prefix='Si P'\n/\n&SYSTEM\n ecutwfc=60\n/\nK_POINTS crystal\n1\n0 0 0 1\n";
        let current = old.replace("/deleted 目录/it''s!pp", "/current/pp");
        assert_eq!(comparable(old).unwrap(), comparable(&current).unwrap());
        for changed in [
            current.replace("Si P", "SiP"),
            current.replace("ecutwfc=60", "ecutwfc=61"),
            current.replace("0 0 0 1", "0 0 1 1"),
            current.replace("&SYSTEM", "&SYSTEM\n pseudo_dir='/other'"),
        ] {
            assert_ne!(comparable(old).unwrap(), comparable(&changed).unwrap());
        }
        for invalid in [
            current.replace("prefix=", "pseudo_dir='/duplicate'\n prefix="),
            current.replace("&CONTROL", "&SYSTEM"),
            current.replace("'/current/pp'", "./unquoted"),
            current.replace("'/current/pp'", "'/unclosed"),
        ] {
            assert!(comparable(&invalid).is_err(), "{invalid}");
        }
    }
}
