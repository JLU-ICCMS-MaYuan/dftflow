use anyhow::Error;
use serde_json::{json, Value};
use std::process::exit;

/// 持久错误类别；stderr 写类别/阶段/原因，JSON 状态读成功也要显示 workflow_status。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Configuration,
    UnsupportedVersion,
    InputConflict,
    Environment,
    SchedulerUnknown,
    BudgetDenied,
    StorageDenied,
    DataInvalid,
    ExecutionFailed,
    NotImplemented,
    Unknown,
}
impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Configuration => "configuration",
            Category::UnsupportedVersion => "unsupported_version",
            Category::InputConflict => "input_conflict",
            Category::Environment => "environment",
            Category::SchedulerUnknown => "scheduler_unknown",
            Category::BudgetDenied => "budget_denied",
            Category::StorageDenied => "storage_denied",
            Category::DataInvalid => "data_invalid",
            Category::ExecutionFailed => "execution_failed",
            Category::NotImplemented => "not_implemented",
            Category::Unknown => "unknown",
        }
    }
    pub fn classify(msg: &str) -> Self {
        if msg.starts_with("configuration:") {
            Category::Configuration
        } else if msg.starts_with("unsupported_version:") {
            Category::UnsupportedVersion
        } else if msg.starts_with("input_conflict:") {
            Category::InputConflict
        } else if msg.starts_with("environment:") {
            Category::Environment
        } else if msg.starts_with("scheduler_unknown:") {
            Category::SchedulerUnknown
        } else if msg.starts_with("budget_denied:") {
            Category::BudgetDenied
        } else if msg.starts_with("storage_denied:") {
            Category::StorageDenied
        } else if msg.starts_with("data_invalid:") {
            Category::DataInvalid
        } else if msg.starts_with("execution_failed:") {
            Category::ExecutionFailed
        } else if msg.starts_with("not_implemented:") {
            Category::NotImplemented
        } else {
            Category::Unknown
        }
    }
}

/// 错误退出：写类别化 stderr 并以非零退出；不得把命令退出零当作计算完成。
pub fn fail(err: Error) -> ! {
    let msg = format!("{:#}", err);
    let cat = Category::classify(&msg);
    eprintln!("{}: {}", cat.as_str(), msg);
    exit(1);
}

/// 输出带格式版本的结构化结果；区分操作成功与流程成功。
pub fn print_json(v: &Value) {
    println!("{}", serde_json::to_string_pretty(v).expect("序列化 JSON 失败"));
}

/// 构造统一 JSON 信封（schema_version、operation_status、workflow_status、diagnostics）。
pub fn envelope(
    operation_status: &str,
    workflow_status: &str,
    extra: Value,
) -> Value {
    let mut v = json!({
        "schema_version": 1u32,
        "operation_status": operation_status,
        "workflow_status": workflow_status,
        "diagnostics": [],
    });
    if let Value::Object(map) = &mut v {
        if let Value::Object(e) = extra {
            for (k, val) in e {
                map.insert(k, val);
            }
        }
    }
    v
}
