//! 用户明确排除完整性核对的 QE 电荷密度和原始波函数；只看名称，不访问文件。
use std::path::Path;

pub fn unchecked(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else { return false };
    if matches!(name, "charge-density.dat" | "charge-density.hdf5" | "charge-density.h5" | "charge-density.xml") {
        return true;
    }
    let numeric = |s: &str| s.is_empty() || s.bytes().all(|b| b.is_ascii_digit());
    for suffix in [".dat", ".hdf5", ".h5"] {
        if let Some(stem) = name.strip_suffix(suffix) {
            if stem.rsplit_once(".wfc").is_some_and(|(_, rest)| numeric(rest)) { return true; }
            if let Some(rest) = stem.strip_prefix("wfc").or_else(|| stem.strip_prefix("evc")) {
                let rest = rest.strip_prefix("up").or_else(|| rest.strip_prefix("dw"))
                    .or_else(|| rest.strip_prefix("dn")).unwrap_or(rest);
                if numeric(rest) { return true; }
            }
        }
    }
    name.rsplit_once(".wfc").is_some_and(|(_, rest)| numeric(rest))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_raw_payloads_without_excluding_other_scientific_data() {
        for name in ["charge-density.dat", "charge-density.hdf5", "wfc1.dat", "wfcup2.dat",
            "wfcdw3.hdf5", "MgB2.wfc1", "MgB2.wfc", "MgB2.wfc2.hdf5", "evc.dat"] {
            assert!(unchecked(Path::new(name)), "{name}");
        }
        for name in ["data-file-schema.xml", "Mg.UPF", "projwfc.out", "Mg.pdos_atm#1_wfc#1",
            "epw.wfc_summary", "wfc_band.dat", "dvscf", "epmatwp1"] {
            assert!(!unchecked(Path::new(name)), "{name}");
        }
    }
}
