//! 常规 QE q2r 对比所需的同源 Γ onsite 修正及 XML 格式转换。
use crate::qe_data::xml;
use anyhow::{ensure, Context, Result};
use std::ops::Range;

fn one<'a, 'b>(root: roxmltree::Node<'a, 'b>, name: &str) -> Result<roxmltree::Node<'a, 'b>> {
    let nodes: Vec<_> = root
        .descendants()
        .filter(|n| n.has_tag_name(name))
        .collect();
    ensure!(nodes.len() == 1, "data_invalid: PH XML 必须有唯一 {name}");
    Ok(nodes[0])
}
fn value_range(node: roxmltree::Node<'_, '_>) -> Result<Range<usize>> {
    let nodes: Vec<_> = node.children().filter(|n| n.is_text()).collect();
    ensure!(nodes.len() == 1, "data_invalid: PH XML 数值内容不唯一");
    Ok(nodes[0].range())
}
fn matrix(root: roxmltree::Node<'_, '_>, a: usize, b: usize) -> Result<Vec<f64>> {
    let n = one(root, &format!("PHI.{a}.{b}"))?;
    let values = xml::numbers(n.text().context("data_invalid: PHI 缺数值")?)?;
    ensure!(values.len() == 18, "data_invalid: PHI 不是复数 3x3 矩阵");
    Ok(values)
}

/// QE xmltools 对复数矩阵直接 READ 后续记录；标签与数值必须分行。
/// 修正矩阵的规范串仍用于检查点摘要，文件布局单独在消费边界转换。
pub(super) fn qe_xml_records(raw: &str) -> Result<String> {
    let doc = roxmltree::Document::parse(raw)?;
    let mut edits = Vec::new();
    for node in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name().starts_with("PHI."))
    {
        let values = xml::numbers(node.text().context("data_invalid: PHI 缺数值")?)?;
        ensure!(values.len() == 18, "data_invalid: PHI 不是复数 3x3 矩阵");
        let records = values
            .chunks_exact(2)
            .map(|pair| format!("  {:.17e} {:.17e}\n", pair[0], pair[1]))
            .collect::<String>();
        edits.push((value_range(node)?, format!("\n{records}    ")));
    }
    ensure!(!edits.is_empty(), "data_invalid: PH XML 缺矩阵");
    let mut output = raw.to_owned();
    for (range, value) in edits.into_iter().rev() {
        output.replace_range(range, &value);
    }
    Ok(output)
}

/// 全 q 星用于 q2r；按 QE dyndiag 约定平均 onsite 上下三角。
pub(super) fn corrected_stars(raw: &str, gamma: &str, nat: usize) -> Result<String> {
    let doc = roxmltree::Document::parse(raw)?;
    let stars: Vec<_> = doc
        .root_element()
        .children()
        .filter(|n| n.is_element() && n.tag_name().name().starts_with("DYNAMICAL_MAT_."))
        .collect();
    let count: usize = xml::text(doc.root_element(), "NUMBER_OF_Q")?
        .trim()
        .parse()?;
    ensure!(
        count > 0 && count == stars.len(),
        "data_invalid: PH 星数量不同"
    );
    let mut edits = vec![];
    for (i, star) in stars.iter().enumerate() {
        ensure!(
            star.tag_name().name() == format!("DYNAMICAL_MAT_.{}", i + 1),
            "data_invalid: PH 星编号不同"
        );
        // 单 q 入口拥有 Γ 行和、原子数和 Fortran 布局的唯一实现。
        let first = stars[0];
        let mut input = raw.to_string();
        if i > 0 {
            let replacement = raw[star.range()].replace(star.tag_name().name(), "DYNAMICAL_MAT_.1");
            input.replace_range(first.range(), &replacement);
        }
        let corrected = corrected_first_star(&input, gamma, nat)?;
        let corrected_doc = roxmltree::Document::parse(&corrected)?;
        let first = one(corrected_doc.root_element(), "DYNAMICAL_MAT_.1")?;
        for a in 1..=nat {
            let mut onsite = matrix(first, a, a)?;
            for j in 0..3 {
                onsite[2 * (j + 3 * j) + 1] = 0.0;
                for i in 0..j {
                    let upper = 2 * (i + 3 * j);
                    let lower = 2 * (j + 3 * i);
                    onsite[upper] = 0.5 * (onsite[upper] + onsite[lower]);
                    onsite[upper + 1] = 0.5 * (onsite[upper + 1] - onsite[lower + 1]);
                    onsite[lower] = onsite[upper];
                    onsite[lower + 1] = -onsite[upper + 1];
                }
            }
            let target = one(*star, &format!("PHI.{a}.{a}"))?;
            edits.push((
                value_range(target)?,
                onsite
                    .iter()
                    .map(|v| format!("{v:.17e}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            ));
        }
    }
    // 旧频率／本征矢不属于派生矩阵。
    for node in doc.root_element().children().filter(|n| {
        n.is_element()
            && matches!(
                n.tag_name().name(),
                "FREQUENCIES_THZ_CMM1" | "DISPLACEMENTS"
            )
    }) {
        edits.push((node.range(), String::new()));
    }
    edits.sort_by_key(|e| e.0.start);
    ensure!(
        edits.windows(2).all(|w| w[0].0.end <= w[1].0.start),
        "data_invalid: PH 修改重叠"
    );
    let mut output = raw.to_string();
    for (range, value) in edits.into_iter().rev() {
        output.replace_range(range, &value);
    }
    roxmltree::Document::parse(&output)?;
    Ok(output)
}

/// PHI 为 Fortran 列主序；EPW 只对 jpol>=ipol 的 onsite 减去同源 Γ 行和。
/// QE dyndiag 随后负责与 EPW 相同的 Hermitian 对称化和质量归一化。
fn corrected_first_star(raw: &str, gamma: &str, nat: usize) -> Result<String> {
    ensure!(nat > 0, "data_invalid: ASR 原子数为零");
    let doc = roxmltree::Document::parse(raw)?;
    let gdoc = roxmltree::Document::parse(gamma)?;
    for tree in [&doc, &gdoc] {
        ensure!(
            xml::text(tree.root_element(), "NUMBER_OF_ATOMS")?
                .trim()
                .parse::<usize>()?
                == nat,
            "input_conflict: Γ 与目标 PH 原子数不同"
        );
    }
    let first = one(doc.root_element(), "DYNAMICAL_MAT_.1")?;
    let gfirst = one(gdoc.root_element(), "DYNAMICAL_MAT_.1")?;
    let q = xml::vector(xml::text(gfirst, "Q_POINT")?)?;
    ensure!(
        q.iter().all(|v| v.abs() < 1e-8),
        "input_conflict: onsite 来源不是 Γ"
    );
    let _ = xml::vector(xml::text(first, "Q_POINT")?)?;
    let mut edits = Vec::new();
    for a in 1..=nat {
        let mut sum = [0.0; 18];
        for b in 1..=nat {
            let g = matrix(gfirst, a, b)?;
            matrix(first, a, b)?;
            for i in 0..18 {
                sum[i] += g[i];
            }
        }
        let node = one(first, &format!("PHI.{a}.{a}"))?;
        let mut onsite = matrix(first, a, a)?;
        for j in 0..3 {
            for i in 0..=j {
                let n = 2 * (i + 3 * j);
                onsite[n] -= sum[n];
                onsite[n + 1] -= sum[n + 1];
            }
        }
        ensure!(
            onsite.iter().all(|v| v.is_finite()),
            "data_invalid: onsite 修正溢出"
        );
        edits.push((
            value_range(node)?,
            onsite
                .iter()
                .map(|v| format!("{v:.17e}"))
                .collect::<Vec<_>>()
                .join(" "),
        ));
    }
    let count = one(doc.root_element(), "NUMBER_OF_Q")?;
    let stars = doc
        .root_element()
        .children()
        .filter(|n| n.is_element() && n.tag_name().name().starts_with("DYNAMICAL_MAT_."))
        .collect::<Vec<_>>();
    ensure!(
        count
            .text()
            .context("data_invalid: 缺 PH 星数")?
            .trim()
            .parse::<usize>()?
            == stars.len()
            && !stars.is_empty(),
        "data_invalid: PH 星数量不符"
    );
    edits.push((value_range(count)?, "1".into()));
    ensure!(stars[0] == first, "data_invalid: PH 星必须从 .1 开始");
    for (i, node) in stars.iter().enumerate().skip(1) {
        ensure!(
            node.tag_name().name() == format!("DYNAMICAL_MAT_.{}", i + 1),
            "data_invalid: PH 星编号重复或缺失"
        );
        edits.push((node.range(), String::new()));
    }
    // 旧频率和位移属于未修正矩阵，不能留在派生文件中冒充重新对角化结果。
    for node in doc.root_element().children().filter(|n| {
        n.is_element()
            && matches!(
                n.tag_name().name(),
                "FREQUENCIES_THZ_CMM1" | "DISPLACEMENTS"
            )
    }) {
        edits.push((node.range(), String::new()));
    }
    edits.sort_by_key(|e| e.0.start);
    ensure!(
        edits.windows(2).all(|w| w[0].0.end <= w[1].0.start),
        "data_invalid: PH XML 修改范围重叠"
    );
    let mut output = raw.to_owned();
    for (range, replacement) in edits.into_iter().rev() {
        output.replace_range(range, &replacement);
    }
    roxmltree::Document::parse(&output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(q: [f64; 3], offset: usize, stars: usize) -> String {
        let mut xml = format!("<Root><GEOMETRY_INFO><NUMBER_OF_ATOMS>2</NUMBER_OF_ATOMS></GEOMETRY_INFO><NUMBER_OF_Q>{stars}</NUMBER_OF_Q><DYNAMICAL_MAT_.1><Q_POINT>{} {} {}</Q_POINT>",q[0],q[1],q[2]);
        for a in 1..=2 {
            for b in 1..=2 {
                xml.push_str(&format!(
                    "<PHI.{a}.{b}>{}</PHI.{a}.{b}>",
                    (0..18)
                        .map(|i| (offset + a * 100 + b * 10 + i).to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                ));
            }
        }
        xml.push_str("</DYNAMICAL_MAT_.1>");
        for i in 2..=stars {
            xml.push_str(&format!(
                "<DYNAMICAL_MAT_.{i}><Q_POINT>0 0 0</Q_POINT></DYNAMICAL_MAT_.{i}>"
            ));
        }
        xml.push_str(
            "<FREQUENCIES_THZ_CMM1><OMEGA.1>1 100</OMEGA.1></FREQUENCIES_THZ_CMM1></Root>",
        );
        xml
    }

    #[test]
    fn all_stars_use_same_gamma_and_qe_hermitian_average_without_cropping() {
        let gamma = sample([0.; 3], 0, 1);
        let mut raw = sample([0.25, 0., 0.], 1000, 2);
        let second = sample([0., 0.25, 0.], 2000, 1);
        let second_doc = roxmltree::Document::parse(&second).unwrap();
        let star = one(second_doc.root_element(), "DYNAMICAL_MAT_.1").unwrap();
        raw = raw.replace(
            "<DYNAMICAL_MAT_.2><Q_POINT>0 0 0</Q_POINT></DYNAMICAL_MAT_.2>",
            &second[star.range()].replace("DYNAMICAL_MAT_.1", "DYNAMICAL_MAT_.2"),
        );
        let before = raw.clone();
        let result = corrected_stars(&raw, &gamma, 2).unwrap();
        assert_eq!(raw, before);
        let doc = roxmltree::Document::parse(&result).unwrap();
        let original = roxmltree::Document::parse(&raw).unwrap();
        let gd = roxmltree::Document::parse(&gamma).unwrap();
        let g = one(gd.root_element(), "DYNAMICAL_MAT_.1").unwrap();
        assert_eq!(
            one(doc.root_element(), "NUMBER_OF_Q").unwrap().text(),
            Some("2")
        );
        assert!(!result.contains("FREQUENCIES_THZ_CMM1"));
        for n in 1..=2 {
            let old = one(original.root_element(), &format!("DYNAMICAL_MAT_.{n}")).unwrap();
            let new = one(doc.root_element(), &format!("DYNAMICAL_MAT_.{n}")).unwrap();
            assert_eq!(
                xml::text(old, "Q_POINT").unwrap(),
                xml::text(new, "Q_POINT").unwrap()
            );
            for a in 1..=2 {
                assert_eq!(
                    matrix(new, a, 3 - a).unwrap(),
                    matrix(old, a, 3 - a).unwrap()
                );
                let old = matrix(old, a, a).unwrap();
                let actual = matrix(new, a, a).unwrap();
                let g1 = matrix(g, a, 1).unwrap();
                let g2 = matrix(g, a, 2).unwrap();
                for j in 0..3 {
                    for i in 0..=j {
                        let u = 2 * (i + 3 * j);
                        let l = 2 * (j + 3 * i);
                        let corrected_re = old[u] - g1[u] - g2[u];
                        let expected_re = if i == j {
                            corrected_re
                        } else {
                            0.5 * (corrected_re + old[l])
                        };
                        assert_eq!(actual[u], expected_re);
                        assert_eq!(actual[l], expected_re);
                        let expected_im = if i == j {
                            0.
                        } else {
                            0.5 * (old[u + 1] - g1[u + 1] - g2[u + 1] - old[l + 1])
                        };
                        assert_eq!(actual[u + 1], expected_im);
                        if i != j {
                            assert_eq!(actual[l + 1], -expected_im);
                        }
                    }
                }
            }
        }
        assert!(corrected_stars(&sample([0.; 3], 0, 2), &gamma, 2).is_err());
        assert!(
            corrected_stars(&raw.replace("<NUMBER_OF_Q>2", "<NUMBER_OF_Q>3"), &gamma, 2).is_err()
        );
    }

    #[test]
    fn asr_matches_epw_upper_complex_onsite_and_preserves_raw() {
        let raw = sample([0.25, 0., 0.], 1000, 2);
        let gamma = sample([0.; 3], 0, 1);
        let before = raw.clone();
        let corrected = corrected_first_star(&raw, &gamma, 2).unwrap();
        assert_eq!(raw, before);
        let original = roxmltree::Document::parse(&raw).unwrap();
        let gdoc = roxmltree::Document::parse(&gamma).unwrap();
        let doc = roxmltree::Document::parse(&corrected).unwrap();
        let first = one(doc.root_element(), "DYNAMICAL_MAT_.1").unwrap();
        let old = one(original.root_element(), "DYNAMICAL_MAT_.1").unwrap();
        let g = one(gdoc.root_element(), "DYNAMICAL_MAT_.1").unwrap();
        for a in 1..=2 {
            for b in 1..=2 {
                let values = matrix(first, a, b).unwrap();
                let old_values = matrix(old, a, b).unwrap();
                for j in 0..3 {
                    for i in 0..3 {
                        for complex in 0..2 {
                            let index = 2 * (i + 3 * j) + complex;
                            let subtraction = if a == b && j >= i {
                                matrix(g, a, 1).unwrap()[index] + matrix(g, a, 2).unwrap()[index]
                            } else {
                                0.
                            };
                            assert_eq!(
                                values[index],
                                old_values[index] - subtraction,
                                "{a} {b} {i} {j} {complex}"
                            );
                        }
                    }
                }
            }
        }
        assert_eq!(xml::text(doc.root_element(), "NUMBER_OF_Q").unwrap(), "1");
        assert!(!corrected.contains("DYNAMICAL_MAT_.2"));
        assert!(!corrected.contains("FREQUENCIES_THZ_CMM1"));
    }

    #[test]
    fn qe_matrix_records_keep_values_and_separate_fortran_read_from_tags() {
        let raw = corrected_first_star(&sample([0.25, 0., 0.], 1000, 1), &sample([0.; 3], 0, 1), 2)
            .unwrap();
        let native = qe_xml_records(&raw).unwrap();
        let old = roxmltree::Document::parse(&raw).unwrap();
        let new = roxmltree::Document::parse(&native).unwrap();
        for a in 1..=2 {
            for b in 1..=2 {
                let name = format!("PHI.{a}.{b}");
                let node = one(new.root_element(), &name).unwrap();
                let text = node.text().unwrap();
                assert!(text.starts_with('\n') && text.trim_end().lines().skip(1).count() == 9);
                assert!(text
                    .lines()
                    .skip(1)
                    .take(9)
                    .all(|line| line.split_whitespace().count() == 2));
                assert_eq!(
                    xml::numbers(text).unwrap(),
                    xml::numbers(one(old.root_element(), &name).unwrap().text().unwrap()).unwrap()
                );
            }
        }
        assert!(qe_xml_records(&raw.replace("1120 1121", "NaN 1121")).is_err());
    }

    #[test]
    fn invalid_asr_sources_are_rejected() {
        let raw = sample([0.25, 0., 0.], 1000, 2);
        let gamma = sample([0.; 3], 0, 1);
        assert!(corrected_first_star(&raw, &sample([0.01, 0., 0.], 0, 1), 2).is_err());
        assert!(corrected_first_star(&raw, &gamma, 1).is_err());
        for bad in [
            raw.replace("1110 1111", "NaN 1111"),
            raw.replace("1110 1111", "1111"),
            raw.replace("<NUMBER_OF_Q>2", "<NUMBER_OF_Q>3"),
            raw.replace("DYNAMICAL_MAT_.2", "DYNAMICAL_MAT_.3"),
            raw.replace("DYNAMICAL_MAT_.1", "DYNAMICAL_MAT_.2"),
        ] {
            assert!(corrected_first_star(&bad, &gamma, 2).is_err());
        }
    }
}
