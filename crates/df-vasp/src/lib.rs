//! df-vasp：VASP 输入生成核心（INCAR/KPOINTS/POTCAR/setup 编排）。
//! 后处理（EIGENVAL/OUTCAR/DOSCAR/COHPCAR.lobster/LOCPOT/CHGCAR 解析与绘图）为后续阶段。

pub mod incar;
pub mod kpoints;
pub mod lobsterin;
pub mod opt;
pub mod potcar;
pub mod process;
pub mod setup;
pub mod templates;
