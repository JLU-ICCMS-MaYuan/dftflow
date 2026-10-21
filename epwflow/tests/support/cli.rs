//! 旧回归用例的动作表示映射到三个真实程序；生产不保留旧命令入口。
use std::{path::Path, process::Command};
pub fn command(args: &[&str], cwd: &Path) -> Command {
    let mut args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut phase = "qe-data".to_string();
    if let Some(i) = args.iter().position(|s| s == "--until") {
        phase = args[i + 1].clone();
        args.drain(i..i + 2);
        if !["qe-data", "interpolation", "superconductivity", "all"].contains(&phase.as_str()) {
            args.push("--invalid-boundary".into());
        }
    } else if args.first().is_some_and(|s| s == "__execute") {
        if let Some(i) = args.iter().position(|s| s == "-w") {
            let p = cwd.join(&args[i + 1]).join("epw-state/workflow.json");
            let record: serde_json::Value =
                serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
            phase = record["until"].as_str().unwrap().to_string();
        }
    }
    if let Some(first) = args.first().cloned() {
        match first.as_str() {
            "qe-data" | "interpolation" | "superconductivity" => {
                phase = args.remove(0);
                let action = args.remove(0);
                match action.as_str() {
                    "check" => args.insert(0, "--check".into()),
                    "import" => {
                        let i = args.iter().position(|s| s == "--source").unwrap();
                        args[i] = "--import".into();
                    }
                    "export-comparison" => {
                        let i = args.iter().position(|s| s == "--source").unwrap();
                        args[i] = "--export-comparison".into();
                    }
                    _ => panic!("未知测试动作 {action}"),
                }
            }
            "delivery" => {
                phase = "all".into();
                args.remove(0);
                let action = args.remove(0);
                if action == "collect" {
                    args.insert(0, "--collect".into());
                } else if action == "results" {
                    args.insert(0, "--results-check".into());
                    let i = args.iter().position(|s| s == "-w").unwrap();
                    args[i] = "--results".into();
                    args[i + 1] = Path::new(&args[i + 1])
                        .join("results")
                        .to_string_lossy()
                        .into();
                } else {
                    panic!("未知测试动作 {action}");
                }
            }
            "budget" => {
                args[0] = "--budget".into();
                if args.get(1).is_none_or(|s| s == "--help") {
                    args.insert(1, "status".into());
                }
            }
            "results" => {
                phase = "interpolation".into();
                args.drain(0..2);
                args.insert(0, "--results-check".into());
            }
            "collect" | "plot" => {
                phase = "interpolation".into();
                args[0] = format!("--{first}");
            }
            "__execute" => {
                args.remove(0);
                let i = args.iter().position(|s| s == "--attempt").unwrap();
                args[i] = "--execute-attempt".into();
            }
            "check" | "status" | "resume" | "probe" => args[0] = format!("--{first}"),
            "prepare" => args[0] = "--prepare-only".into(),
            "submit" => args[0] = "--submit".into(),
            "run" => args[0] = "--run".into(),
            _ => panic!("未知测试动作 {first}"),
        }
    }
    let binary = match phase.as_str() {
        "qe-data" => env!("CARGO_BIN_EXE_epw_phonon"),
        "interpolation" => env!("CARGO_BIN_EXE_epw_wannier"),
        _ => env!("CARGO_BIN_EXE_epw_superconductivity"),
    };
    let mut command = Command::new(binary);
    command.args(args);
    command
}
