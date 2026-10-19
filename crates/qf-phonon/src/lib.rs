//! qf-phonon：`qeflow` 声子工作流的领域库（split 生成/merge/归档/表格/绘图）。
//! 复刻 `qeflow/phonon/{split_merge,archive,phonon_tables,phonon_plot,q_index_map}.py`。

pub mod archive;
pub mod epw;
pub mod band;
pub mod index_map;
pub mod merge;
pub mod plot;
pub mod split;
pub mod tables;
pub mod xml;

/// 将二维 `Vec<Vec<String>>` 写出为 CSV（首行作为表头）。
pub fn write_csv(path: &str, rows: &[Vec<String>]) -> std::io::Result<()> {
    let mut s = String::new();
    for row in rows {
        s.push_str(&row.join(","));
        s.push('\n');
    }
    std::fs::write(path, s)
}

/// 运行声子后处理：q2r + matdyn 已在外部完成，此函数负责归档、表格与绘图。
/// `run_q2r_matdyn` 为 true 时同时生成 q2r/matdyn 输入（已在 CLI 编排中处理）。
pub use archive::{collect_phband_outputs, collect_phdos_outputs};
pub use merge::merge_split_results;
pub use split::generate_split_inputs;
pub use tables::{build_phonon_band_tables, build_phonon_dos_tables};
