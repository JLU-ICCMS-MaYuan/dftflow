//! df-vasp 后处理单元测试：轨道解析、dat 表解析、at-EF 插值。

use df_vasp::process::{
    interpolate_at_zero, orbital_group, parse_element_specs, read_dat_table,
};

#[test]
fn orbital_group_aggregates_components() {
    assert_eq!(orbital_group("s"), Some(vec!["s".to_string()]));
    assert_eq!(
        orbital_group("p"),
        Some(vec!["py".to_string(), "pz".to_string(), "px".to_string()])
    );
    assert_eq!(
        orbital_group("d"),
        Some(vec![
            "dxy".to_string(),
            "dyz".to_string(),
            "dz2".to_string(),
            "dxz".to_string(),
            "dx2-y2".to_string()
        ])
    );
    assert!(orbital_group("nope").is_none());
}

#[test]
fn parses_element_specs() {
    let specs = parse_element_specs(&["Mg:s,p".to_string(), "B:d".to_string()]).unwrap();
    assert_eq!(specs[0].0, "Mg");
    assert_eq!(specs[0].1, vec!["s".to_string(), "p".to_string()]);
    assert_eq!(specs[1].1, vec!["d".to_string()]);
    assert!(parse_element_specs(&["Mg".to_string()]).is_err());
    assert!(parse_element_specs(&["Mg:xyz".to_string()]).is_err());
}

#[test]
fn reads_dat_table_header_and_columns() {
    let dir = std::env::temp_dir().join(format!("df_vasp_proc_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("PDOS_Mg.dat");
    std::fs::write(
        &p,
        "# Energy(eV) s py pz px tot\n-1.0 0.01 0.02 0.02 0.02 0.12\n0.0 0.05 0.06 0.06 0.06 0.43\n",
    )
    .unwrap();
    let cols = read_dat_table(p.to_str().unwrap()).unwrap();
    assert_eq!(cols.get("energyev").unwrap(), &vec![-1.0, 0.0]);
    assert_eq!(cols.get("tot").unwrap(), &vec![0.12, 0.43]);
    assert_eq!(cols.get("py").unwrap(), &vec![0.02, 0.06]);
}

#[test]
fn interpolate_at_zero_exact_and_linear() {
    let e = vec![-1.0, 0.0, 1.0];
    assert_eq!(interpolate_at_zero(&e, &vec![0.1, 0.5, 0.2]), 0.5);
    let e2 = vec![-1.0, 1.0];
    // 在 0 处线性插值：0.1 + (0.2-0.1)/2 = 0.15
    assert!((interpolate_at_zero(&e2, &vec![0.1, 0.2]) - 0.15).abs() < 1e-9);
    let e3 = vec![0.5, 1.0];
    assert_eq!(interpolate_at_zero(&e3, &vec![0.7, 0.9]), 0.7);
}
