//! QE 文件名派生（复刻 `qeflow/input_deck/filenames.py: derive_filenames`）。

/// 由 prefix 派生的 QE 中间文件名。
#[derive(Debug, Clone)]
pub struct Filenames {
    pub fildyn: String,
    pub dyn0: String,
    pub flfrc: String,
    pub flfrq: String,
    pub fldos: String,
}

/// 由 prefix 派生 QE 文件名（与 Python 版完全一致）。
pub fn derive_filenames(prefix: &str) -> Filenames {
    Filenames {
        fildyn: format!("{prefix}.dyn"),
        dyn0: format!("{prefix}.dyn0"),
        flfrc: format!("{prefix}.fc"),
        flfrq: format!("{prefix}.phband.freq"),
        fldos: format!("{prefix}.phdos.dat"),
    }
}
