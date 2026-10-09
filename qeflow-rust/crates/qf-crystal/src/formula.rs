//! 化学式生成（复刻 `qeflow/crystal/formula.py`）。

/// 由元素符号与计数生成简化化学式字符串，例如 ("Mg","B","2") -> "MgB2"。
pub fn get_formula(elements: &[String], counts: &[usize]) -> String {
    let mut formula = String::new();
    for (element, count) in elements.iter().zip(counts.iter()) {
        formula.push_str(element);
        if *count >= 1 {
            formula.push_str(&count.to_string());
        }
    }
    formula
}
