//! substitutionflow 移植的集成测试：case 生成、状态机、formula 与目录命名对齐 Python。

use std::fs;
use std::path::Path;

fn temp_root(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("df_sub_test_{}_{}", tag, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_config(dir: &Path) {
    fs::write(
        dir.join("input.toml"),
        "[substitutionflow]\nXA = \"11\"\nXB = \"13,14\"\npressures = [20, 50]\nstructure_filename = \"structure.vasp\"\n",
    )
    .unwrap();
    fs::write(
        dir.join("structure.vasp"),
        "Mg2B1-XA-XB\n1.0\n3.0 0 0\n0 3.0 0\n0 0 3.0\nXA XB Si\n1 1 1\nDirect\n0.0 0.0 0.0\n0.5 0.5 0.5\n0.25 0.25 0.25\n",
    )
    .unwrap();
}

#[test]
fn generate_cases_matches_python_layout() {
    let dir = temp_root("gen");
    write_config(&dir);
    let input = dir.join("structure.vasp").to_string_lossy().into_owned();
    let config = dir.join("input.toml").to_string_lossy().into_owned();
    let out = dir.join("out").to_string_lossy().into_owned();
    let result = df_substitution::structure::generate_cases(&input, &config, &out).unwrap();
    df_substitution::status::create_initial_status(&out).unwrap();
    // 1 XA × 2 XB = 2 个 case；目录命名 {XA_Z:02}.XA_{sym}/period_{P}/{XB_Z:02}.{formula}
    assert_eq!(result.cases.len(), 2);
    assert!(dir.join("out/11.XA_Na/period_3/13.NaAlSi").is_dir());
    assert!(dir.join("out/11.XA_Na/period_3/14.NaSi2").is_dir());
    assert!(dir.join("out/11.XA_Na/period_3/workflow_manifest.json").is_file());
    assert!(dir.join("out/11.XA_Na/period_3/13.NaAlSi/case_status.json").is_file());

    // 替换后的结构：XA→Na、XB→Al，元素分组合并
    let poscar = fs::read_to_string(dir.join("out/11.XA_Na/period_3/13.NaAlSi/structure.vasp")).unwrap();
    let first = poscar.lines().next().unwrap();
    assert_eq!(first, "NaAlSi");
    let element_line = poscar.lines().nth(5).unwrap();
    assert_eq!(element_line, "  Na  Al  Si");

    // case_status.json：无 mattersim 时步骤顺序不含 mattersim_opt，全 unstarted
    let status: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("out/11.XA_Na/period_3/13.NaAlSi/case_status.json")).unwrap())
            .unwrap();
    let steps = status["pressures"]["20"].as_object().unwrap();
    let keys: Vec<&String> = steps.keys().collect();
    assert_eq!(
        keys,
        vec!["vasp_opt", "vasp_scf", "vasp_eledos", "vasp_process_eledos", "qe_phonon"]
    );
    assert!(steps.values().all(|v| v == "unstarted"));

    // workflow_status.json 汇总
    let summary: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("out/workflow_status.json")).unwrap()).unwrap();
    assert_eq!(summary["cases"].as_object().unwrap().len(), 2);
}

#[test]
fn status_heuristics_detect_outcar_states() {
    let dir = temp_root("heur");
    // 压强子目录 20 下的 vasp_opt 步骤
    let pressure_dir = dir.join("20/vasp_opt");
    fs::create_dir_all(&pressure_dir).unwrap();
    // 无痕迹 → unstarted
    assert_eq!(
        df_substitution::status::detect_case_pressure_status(dir.to_str().unwrap(), "20")
            .get("vasp_opt")
            .unwrap(),
        "unstarted"
    );
    // 有 OUTCAR 但未收敛 → error
    fs::write(pressure_dir.join("OUTCAR"), "some partial output\n").unwrap();
    assert_eq!(
        df_substitution::status::detect_case_pressure_status(dir.to_str().unwrap(), "20")
            .get("vasp_opt")
            .unwrap(),
        "error"
    );
    // 收敛标记 + CONTCAR → finished
    fs::write(
        pressure_dir.join("OUTCAR"),
        "reached required accuracy - stopping structural energy minimisation\n",
    )
    .unwrap();
    fs::write(pressure_dir.join("CONTCAR"), "direct\n1.0\n").unwrap();
    assert_eq!(
        df_substitution::status::detect_case_pressure_status(dir.to_str().unwrap(), "20")
            .get("vasp_opt")
            .unwrap(),
        "finished"
    );
}

#[test]
fn qe_phonon_status_reduction_matches_python() {
    use df_substitution::status::reduce_qe_phonon_workflow_status;
    let all_finished = serde_json::json!({
        "scf": "finished", "ph": {"state": "finished"},
        "q2r": "finished", "matdyn_phband": "finished",
        "matdyn_phdos": "finished", "phonon_plot": "finished"
    });
    assert_eq!(
        reduce_qe_phonon_workflow_status("qe_phonon_nosplit", &all_finished).unwrap(),
        "finished"
    );
    let with_error = serde_json::json!({
        "scf": "finished", "ph": {"state": "error"},
        "q2r": "unstarted", "matdyn_phband": "unstarted",
        "matdyn_phdos": "unstarted", "phonon_plot": "unstarted"
    });
    assert_eq!(
        reduce_qe_phonon_workflow_status("qe_phonon_nosplit", &with_error).unwrap(),
        "error"
    );
    let split_status = serde_json::json!({
        "scf": "finished",
        "ph": {"dyn0": "finished", "tasks": {"01": "finished", "02": "unstarted"}},
        "merge": "unstarted", "q2r": "unstarted", "matdyn_phband": "unstarted",
        "matdyn_phdos": "unstarted", "phonon_plot": "unstarted"
    });
    assert_eq!(
        reduce_qe_phonon_workflow_status("qe_phonon_split_index", &split_status).unwrap(),
        "unstarted"
    );
}

#[test]
fn selection_and_formula_helpers() {
    assert_eq!(df_substitution::structure::build_formula(&["Na".into(), "Al".into(), "Si".into()], &[1, 1, 1]), "NaAlSi");
    assert_eq!(df_substitution::structure::build_formula(&["Na".into(), "Cl".into()], &[2, 2]), "NaCl");
    assert_eq!(
        df_substitution::structure::build_case_relative_path("Na", "Al", "NaAlSi").unwrap(),
        "11.XA_Na/period_3/13.NaAlSi"
    );
    assert_eq!(df_substitution::pressure_key(&serde_json::json!(20)), "20");
    assert_eq!(df_substitution::pressure_key(&serde_json::json!(12.5)), "12.5");
}
