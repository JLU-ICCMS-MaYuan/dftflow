use super::*;
use std::collections::BTreeMap;

fn fixture() -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("epw-decay-{}", store::token()));
    let native = root.join("6.epw-coarse");
    fs::create_dir_all(&native).unwrap();
    fs::write(native.join("epw.out"), "verified scientific checkpoint").unwrap();
    for (i, name) in NATIVE.iter().enumerate() {
        let rows = match i {
            1 => "0 0\n2 0\n",
            2 => "0 1D-5\n",
            3 => "0 0.2\n2 0.2\n",
            _ => "# Ang Ry\n0 1D0\n2 1d-2 extra\n2 2e-2\n4 0\n",
        };
        fs::write(native.join(name), rows).unwrap();
    }
    let receipt = native.join("completion.json");
    let c = Completion {
        schema_version: 1,
        step: Step::Coarse,
        scientific_key: "fixture-coarse".into(),
        upstream: BTreeMap::new(),
        outputs: NATIVE
            .iter()
            .copied()
            .chain(["epw.out"])
            .map(|n| Identity::capture(&native.join(n)).unwrap())
            .collect(),
        provenance: None,
    };
    store::write(&receipt, &c).unwrap();
    let output = root.join("bundle");
    (root, receipt, output)
}

#[test]
fn native_rows_preserve_duplicates_zero_and_fortran_exponents_and_reject_bad_rows() {
    let rows = parse("# units\n\n0 1D0 ignored\n2 1d-2\n2 2e-2\n4 0\n", "decay.H").unwrap();
    assert_eq!(rows, vec![[0., 1.], [2., 0.01], [2., 0.02], [4., 0.]]);
    for bad in [
        "",
        "# no data",
        "-1 2",
        "1 -2",
        "0 NaN",
        "0 inf",
        "0 1e999",
        "0 nope",
        "1",
    ] {
        let error = parse(bad, "decay.H").unwrap_err().to_string();
        assert!(error.contains("decay.H"), "{bad}: {error}");
        if !bad.is_empty() && !bad.starts_with('#') {
            assert!(error.contains(":1"), "{error}");
        }
    }
}

#[test]
fn missing_diagnostics_report_all_missing_without_invalidating_science_or_accepting_tampering() {
    let (root, receipt, output) = fixture();
    let c: Completion = store::read(&receipt).unwrap();
    let original = fs::read(&receipt).unwrap();
    for name in [NATIVE[0], NATIVE[2]] {
        fs::remove_file(receipt.parent().unwrap().join(name)).unwrap();
    }
    assert!(c.available().unwrap());
    c.verify().unwrap();
    let error = collect(&receipt, &output).unwrap_err().to_string();
    assert!(
        error.contains(NATIVE[0]) && error.contains(NATIVE[2]) && error.contains("不自动重跑 EPW"),
        "{error}"
    );
    assert_eq!(fs::read(&receipt).unwrap(), original);
    fs::write(receipt.parent().unwrap().join(NATIVE[1]), "0 123\n").unwrap();
    assert!(c.available().is_err());
    assert!(c.verify().is_err());
    assert!(collect(&receipt, &output).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fifteen_figures_keep_native_samples_and_recover_after_moving_without_original_sources() {
    let (root, receipt, output) = fixture();
    let before = fs::read(&receipt).unwrap();
    collect(&receipt, &output).unwrap();
    let (data, manifest) = load(&output, true).unwrap();
    assert_eq!(manifest.files.len(), 20);
    assert_eq!(
        data.panels[0],
        vec![[0., 1.], [2., 0.01], [2., 0.02], [4., 0.]]
    );
    for name in FIGURES {
        assert!(fs::read(output.join(format!("{name}.png")))
            .unwrap()
            .starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(fs::read(output.join(format!("{name}.pdf")))
            .unwrap()
            .starts_with(b"%PDF"));
        let svg = fs::read_to_string(output.join(format!("{name}.svg"))).unwrap();
        assert!(svg.contains("#AA00CC") || name == "decay-dynmat");
        assert!(svg.contains("(Ry)") && svg.contains("(Å)"));
    }
    let svg = fs::read_to_string(output.join("decay-H.svg")).unwrap();
    assert_eq!(svg.matches("<circle").count(), 3);
    assert!(svg.contains("Zero values omitted: 1"));
    assert!(fs::read_to_string(output.join("decay-dynmat.svg"))
        .unwrap()
        .contains("No positive values"));
    let c: Completion = store::read(&receipt).unwrap();
    c.verify().unwrap();
    collect(&receipt, &output).unwrap();
    assert_eq!(before, fs::read(&receipt).unwrap());
    let moved = root.join("portable");
    fs::rename(&output, &moved).unwrap();
    fs::rename(receipt.parent().unwrap(), root.join("unavailable-science")).unwrap();
    let data_hash = store::hash_file(&moved.join(DATA)).unwrap();
    fs::remove_file(moved.join("decay-overview.png")).unwrap();
    redraw(&moved).unwrap();
    load(&moved, true).unwrap();
    assert_eq!(data_hash, store::hash_file(&moved.join(DATA)).unwrap());
    // 即使更新 JSON 哈希，也不能将与原生副本不符的样本作为合法重绘输入。
    let mut altered: Data = store::read(&moved.join(DATA)).unwrap();
    altered.panels[0][0][1] = 0.7;
    store::write(&moved.join(DATA), &altered).unwrap();
    let mut m: Manifest = store::read(&moved.join(MANIFEST)).unwrap();
    *m.files
        .iter_mut()
        .find(|f| f.relative_path == Path::new(DATA))
        .unwrap() = Artifact::capture(&moved, DATA).unwrap();
    store::write(&moved.join(MANIFEST), &m).unwrap();
    assert!(redraw(&moved).unwrap_err().to_string().contains("不一致"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupted_decay_publication_resumes_from_journal_and_rejects_unknown_target() {
    let (root, receipt, output) = fixture();
    collect(&receipt, &output).unwrap();
    let (_, old) = load(&output, true).unwrap();
    let stage = output.join(".native-interrupted");
    fs::create_dir(&stage).unwrap();
    for name in names() {
        fs::copy(output.join(&name), stage.join(name)).unwrap();
    }
    let files = names()
        .iter()
        .map(|n| Artifact::capture(&stage, n).unwrap())
        .collect::<Vec<_>>();
    let next = Manifest {
        schema_version: 1,
        files: files.clone(),
    };
    let mut payload = serde_json::to_vec_pretty(&next).unwrap();
    payload.push(b'\n');
    store::write(
        &output.join(format!(".{MANIFEST}.publish.json")),
        &serde_json::json!({
            "schema_version":1,"stage":".native-interrupted","manifest":MANIFEST,
            "previous_digest":store::hash_file(&output.join(MANIFEST)).unwrap(),"payload":payload,
            "files":files,"previous_files":old.files
        }),
    )
    .unwrap();
    fs::rename(stage.join("decay-H.png"), output.join("decay-H.png")).unwrap();
    let original = fs::read(output.join("decay-H.svg")).unwrap();
    fs::write(output.join("decay-H.svg"), "user data").unwrap();
    assert!(redraw(&output).is_err());
    assert_eq!(
        fs::read_to_string(output.join("decay-H.svg")).unwrap(),
        "user data"
    );
    fs::write(output.join("decay-H.svg"), original).unwrap();
    redraw(&output).unwrap();
    load(&output, true).unwrap();
    assert!(!output.join(format!(".{MANIFEST}.publish.json")).exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "需要显式指定真实粗网格回执及全新派生输出目录；只读科学数据"]
fn real_bound_native_decay_files_render_without_scientific_calls() {
    let receipt =
        PathBuf::from(std::env::var_os("DFTFLOW_DECAY_REAL_COMPLETION").expect("真实回执"));
    let output = PathBuf::from(std::env::var_os("DFTFLOW_DECAY_REAL_OUTPUT").expect("全新输出"));
    assert!(!output.exists());
    let c: Completion = store::read(&receipt).unwrap();
    let before = fs::read(&receipt).unwrap();
    collect(&receipt, &output).unwrap();
    load(&output, true).unwrap();
    for id in &c.outputs {
        id.verify().unwrap();
    }
    assert_eq!(before, fs::read(&receipt).unwrap());
    eprintln!("真实来源只读衰减图：{}", output.display());
}
