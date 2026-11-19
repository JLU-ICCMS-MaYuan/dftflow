//! 0076：经公开 CLI 验证自动出图、跳过选项和绘图失败的科学结果隔离。
use super::*;

#[test]
fn completed_stage_skips_then_replots_without_repeating_science() {
    use epwflow::foundation::{readable_state as logs, stages::Step};
    let c = Case::gap_batch(config::Mode::Iso, vec![20., 30.], None);
    c.accounting(Some(777));
    success(c.numbered_cli(
        2,
        &["--stage", "superconductivity", "--skip-plot", "-r"],
        Some(777),
        "0076-skip",
    ));
    let results = c.root.join("results/8.epw-superconductivity");
    assert!(!results.exists());
    let calls = c.calls();
    c.accounting(Some(778));
    let output = c.numbered_cli(
        2,
        &["--stage", "superconductivity", "--resume", "-r"],
        Some(778),
        "0076-auto",
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("superconductivity_plots:"));
    success(output);
    assert_eq!(calls, c.calls());
    assert_eq!(
        logs::read(&c.root.join("run"), Step::Superconductivity)
            .unwrap()
            .get("Stage", "Scientific status"),
        Some("completed")
    );
    fn find_figure(dir: &std::path::Path) -> Option<PathBuf> {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().unwrap() == "fig4_iso_gap_vs_T.svg" {
                return Some(path);
            }
            if path.is_dir() {
                if let Some(found) = find_figure(&path) {
                    return Some(found);
                }
            }
        }
        None
    }
    let figure =
        find_figure(&results).expect("automatic CLI hook must render the temperature plot");
    // 在隔离输出路径制造真实写入失败；不替换绘图器，不修改科学输入。
    fs::remove_file(&figure).unwrap();
    fs::create_dir(&figure).unwrap();
    c.accounting(Some(779));
    let output = c.numbered_cli(
        2,
        &["--stage", "superconductivity", "--resume", "-r"],
        Some(779),
        "0076-render-failure",
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Warning: superconductivity plot:"));
    success(output);
    assert_eq!(calls, c.calls());
    assert_eq!(
        logs::read(&c.root.join("run"), Step::Superconductivity)
            .unwrap()
            .get("Stage", "Scientific status"),
        Some("completed")
    );

    // 方法输出目录自身无法创建时，也必须将具体失败原因告知用户。
    let method_results = figure.parent().unwrap();
    fs::remove_dir_all(method_results).unwrap();
    fs::write(method_results, "blocked output directory").unwrap();
    c.accounting(Some(781));
    let output = c.numbered_cli(
        2,
        &["--stage", "superconductivity", "--resume", "-r"],
        Some(781),
        "0076-directory-failure",
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Warning: superconductivity plot:"),
        "directory creation failures must be reported: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    success(output);
    assert_eq!(calls, c.calls());
}

#[test]
fn invalid_gap_data_do_not_trigger_automatic_plotting() {
    use epwflow::foundation::{readable_state as logs, stages::Step};
    let c = Case::gap_batch(config::Mode::Iso, vec![20., 30.], None);
    c.put("gap-fault", "missing");
    c.accounting(Some(780));
    let output = c.numbered_cli(
        2,
        &["--stage", "superconductivity", "-r"],
        Some(780),
        "0076-invalid",
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("superconductivity_plots:"));
    success(output);
    assert!(!c.root.join("results/8.epw-superconductivity").exists());
    assert_ne!(
        logs::read(&c.root.join("run"), Step::Superconductivity)
            .unwrap()
            .get("Stage", "Scientific status"),
        Some("completed")
    );
}
