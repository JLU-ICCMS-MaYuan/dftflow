//! qf-phonon: 复刻 Quantum ESPRESSO `ph.x` 生成 `prefix.dyn0` 的不可约 q 点网格算法。
//!
//! 设计来源（`external/q-e`）：
//! - `PHonon/PH/q_points.f90`  — 写出 dyn0 的驱动；调用 `kpoint_grid` 后再做 `cryst_to_cart`。
//! - `PW/src/kpoint_grid.f90` — `kpoint_grid`：Monkhorst-Pack 网格 + 对称折叠生成不可约 q 点。
//! - `Modules/cryst_to_car.f90` — `cryst_to_cart(iflag=1)`：晶体坐标 -> 笛卡尔坐标。
//!
//! 关键结论（与真实 `Mg1B2.dyn0` 字节级比对验证）：
//! 1. dyn0 中 q 点坐标为**笛卡尔坐标**（tpiba = 2π/a₀），由 `cryst_to_cart` 得到，并非晶体坐标。
//! 2. `kpoint_grid` 内部所有 `NINT` 为“半数远离零”取整（与 Rust `f64::round()` 一致）。
//! 3. dyn0 字段格式为 Fortran `(3i4)` / `(i4)` / `(3e24.15)`：整数 4 宽右对齐、坐标 24 宽右对齐、
//!    每个坐标 15 位小数、指数两位、大写 `E`。本模块 `format_e24_15` 精确复现该格式。
//!
//! 这样即可**跳过 `ph.x` 单独跑出 dyn0 再杀掉的环节**，直接从 SCF 输出 XML 算出与 ph.x 完全相同的
//! q 点列表，使整条声子链可以一口气跑完。

pub mod dyn0;
pub mod qgrid;
pub mod xml;

pub use dyn0::{format_e24_15, write_dyn0, write_q_index_map};
pub use qgrid::{generate_q_points, QGridParams, QPoint};
pub use xml::{parse_qe_save_xml, QeGridInfo};

use std::path::Path;

/// 一站式：解析 SCF save XML -> 生成不可约 q 点 -> 写出 `prefix.dyn0`（与 ph.x 字节一致）
/// 以及 `q_index_map.json`（含晶体坐标，作为拆分步骤的规范来源）。
pub fn generate_dyn0(
    xml_path: &Path,
    out_dir: &Path,
    prefix: &str,
    params: &QGridParams,
) -> anyhow::Result<Vec<QPoint>> {
    let info = parse_qe_save_xml(xml_path)?;
    let effective = QGridParams {
        time_reversal: params.time_reversal.unwrap_or(info.time_reversal),
        ..*params
    };
    let qpoints = generate_q_points(&info, &effective);
    std::fs::create_dir_all(out_dir)?;
    write_dyn0(&out_dir.join(format!("{prefix}.dyn0")), &qpoints, params.nk1, params.nk2, params.nk3)?;
    write_q_index_map(&out_dir.join("q_index_map.json"), &qpoints, params)?;
    Ok(qpoints)
}
