//! 物理常数（对齐 `scflow/constants.py`，单位尽量沿用 QE 习惯）。

/// 1 K = 0.0208366176361328 THz（频率换算）。
pub const K_T_THZ: f64 = 0.0208366176361328;
/// 1 Ry = 3289.84367720502 THz。
pub const K_RY_THZ: f64 = 3289.84367720502;
/// 1 Ry = 157887.6061 K。
pub const K_RY_K: f64 = 157887.6061;
/// 1 Ry = 13.6056980659 eV。
pub const K_RY_EV: f64 = 13.6056980659;
/// 1 Ry = 3289843.67720502 GHz。
pub const K_RY_GHZ: f64 = 3289843.67720502;
/// Boltzmann 常数 (J/K)。
pub const K_B: f64 = 1.380649e-23;
/// 1 Ry = 2.179872361103542e-18 J。
pub const K_RY_J: f64 = 2.179872361103542e-18;
/// 1 J = 6.241506363094e21 meV。
pub const K_J_MEV: f64 = 6.241506363094e21;
/// 阿伏伽德罗常数。
pub const N_A: f64 = 6.02214076e23;
/// 1 Å = 1e-8 cm。
pub const K_A_CM: f64 = 1e-8;

/// 频率 THz -> K 的换算系数。
pub const K_PER_THZ: f64 = 1.0 / K_T_THZ;
