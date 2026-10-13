//! QE 输入数值格式化（复刻 `qeflow/input_deck/values.py: to_qe_value`）。

use regex::Regex;
use serde_json::Value;

/// 将配置值格式化为 QE 输入可接受的字符串表示。
pub fn to_qe_value(value: &Value) -> String {
    match value {
        Value::Bool(b) => {
            if *b {
                ".true.".to_string()
            } else {
                ".false.".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => format_string(s),
        other => other.to_string(),
    }
}

fn format_string(s: &str) -> String {
    if s.starts_with('\'') || s.starts_with('"') {
        return s.to_string();
    }
    let low = s.to_lowercase();
    if low == ".true." || low == ".false." {
        return low;
    }
    let re = Regex::new(r"^[+-]?(\d+(\.\d*)?|\.\d+)([deDE][+-]?\d+)?$").unwrap();
    if re.is_match(s) {
        return s.to_string();
    }
    format!("'{s}'")
}
