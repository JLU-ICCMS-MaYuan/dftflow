//! 经真实 CLI 验证叠图、可移植重绘和拒绝无效来源；样式数据明确为合成。
#[path = "support/cli.rs"]
mod stage_cli;

use epwflow::{
    delivery::comparison::{
        self, Artifact, Manifest, Metadata, PhononErrorMarker, Tick, KINDS,
    },
    foundation::store,
    interpolation::comparison::gaussian_dos,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Output,
};

const TABLES: [&str; 5] = ["electronic_bands.csv", "phonon_bands.csv", "phonon_reference_points.csv", "electronic_dos.csv", "phonon_dos.csv"];

fn fixture(root: &Path) -> PathBuf {
    let work = root.join("work");
    let dir = work.join("interpolation/comparison");
    fs::create_dir_all(&dir).unwrap();
    let mut electron = String::from("method,segment_id,point_id,distance,band,energy_ev\n");
    let mut phonon =
        String::from("method,reference_kind,segment_id,point_id,distance,mode,frequency_cm1\n");
    let mut edos = String::from("method,energy_ev,projection_kind,element,orbital,density\n");
    let mut pdos = String::from("method,frequency_cm1,projection_kind,element,density\n");
    for method in ["qe", "epw"] {
        let d = if method == "qe" { 0.0 } else { 0.008 };
        for i in 0..101 {
            let x = i as f64 / 100.0;
            for b in 0..4 {
                let e = (x * 6.0 + b as f64).sin() * 0.7 + (b as f64 - 1.5) * 0.18 + d;
                electron.push_str(&format!("{method},0,{i},{x},{b},{e}\n"));
            }
            for m in 0..9 {
                let f = 50.0 * m as f64
                    + ((x * 3.14159).sin().abs()) * (50.0 + m as f64 * 4.0)
                    + d * 40.0;
                phonon.push_str(&format!(
                    "{method},{},0,{i},{x},{m},{f}\n",
                    if method == "qe" { "qe_matdyn" } else { "epw" }
                ));
            }
            let e = -0.5 + x;
            let y = 1.0 + (-((e - 0.15 + d) / 0.12).powi(2)).exp() * 3.0;
            edos.push_str(&format!("{method},{e},total,,,{y}\n"));
            for element in ["Mg", "B"] {
                edos.push_str(&format!(
                    "{method},{e},element,{element},,{}\n",
                    y * if element == "B" { 0.6 } else { 0.3 }
                ));
                for orbital in ["s", "p"] {
                    edos.push_str(&format!(
                        "{method},{e},orbital,{element},{orbital},{}\n",
                        y * if orbital == "p" { 0.3 } else { 0.15 }
                    ));
                }
            }
            let f = x * 550.0;
            let y = (-((f - 200.0) / 55.0).powi(2)).exp() * 0.04
                + (-((f - 420.0 + d * 40.0) / 25.0).powi(2)).exp() * 0.055;
            pdos.push_str(&format!("{method},{f},total,,{y}\n"));
            for element in ["Mg", "B"] {
                pdos.push_str(&format!(
                    "{method},{f},element,{element},{}\n",
                    y * if element == "B" { 0.7 } else { 0.3 }
                ));
            }
        }
    }
    for (name, body) in [
        (TABLES[0], electron),
        (TABLES[1], phonon),
        (
            TABLES[2],
            "point_id,distance,mode,frequency_cm1,source\n0,0,1,0,synthetic_fixture\n".into(),
        ),
        (TABLES[3], edos),
        (TABLES[4], pdos),
    ] {
        fs::write(dir.join(name), body).unwrap();
    }
    let ticks = || {
        vec![
            Tick {
                distance: 0.0,
                label: "G".into(),
            },
            Tick {
                distance: 0.5,
                label: "M".into(),
            },
            Tick {
                distance: 1.0,
                label: "K".into(),
            },
        ]
    };
    let metadata = Metadata {
        title: "SYNTHETIC STYLE TEST - NOT MgB2 RESULTS".into(),
        fermi_ev: 7.0,
        window_ev: [-0.5, 0.5],
        disentanglement_window_ev: Some([6.5, 7.5]),
        frozen_window_ev: Some([6.5, 7.2]),
        electronic_ticks: ticks(),
        phonon_ticks: ticks(),
        electronic_energy_unit: "eV_relative_fermi".into(),
        phonon_frequency_unit: "cm-1".into(),
        electronic_density_unit: "states/(eV*cell)".into(),
        phonon_density_unit: "states/(cm-1*cell)".into(),
        elements: vec!["Mg".into(), "B".into()],
        orbitals: vec!["Mg:s".into(), "Mg:p".into(), "B:s".into(), "B:p".into()],
        electronic_projection: "atomic_projector_operator".into(),
        phonon_projection: "mass_weighted_eigenvectors".into(),
        phonon_error_markers: vec![
            PhononErrorMarker {
                scope: "sampled_maximum".into(),
                q_crystal: [1.0 / 12.0; 3],
                mode_sorted: 7,
                qe_cm1: 300.0,
                epw_cm1: 349.0,
                error_cm1: 49.0,
                path_distance: None,
                path_label: None,
            },
            PhononErrorMarker {
                scope: "path_maximum".into(),
                q_crystal: [0.5, 0.0, 0.0],
                mode_sorted: 8,
                qe_cm1: 400.0,
                epw_cm1: 410.0,
                error_cm1: 10.0,
                path_distance: Some(0.5),
                path_label: Some("M (high-symmetry point)".into()),
            },
        ],
        notes: vec!["Synthetic projections for renderer regression only".into()],
    };
    store::write(&dir.join("metadata.json"), &metadata).unwrap();
    store::write(
        &dir.join("metrics.json"),
        &serde_json::json!({"synthetic":true}),
    )
    .unwrap();
    let manifest = Manifest {
        schema_version: 1,
        run_id: "synthetic-style-test".into(),
        ph_mode: "index".into(),
        upstream_digest: "0".repeat(64),
        quality_status: Some("unverified".into()),
        missing: vec!["SYNTHETIC TEST DATA - no scientific acceptance".into()],
        files: vec![],
    };
    store::write(&dir.join("manifest.json"), &manifest).unwrap();
    rehash(&dir);
    work
}
fn rehash(dir: &Path) {
    let mut manifest: Manifest = store::read(&dir.join("manifest.json")).unwrap();
    let tables: &[&str] = if manifest.schema_version == 1 { &TABLES } else { &comparison::TABLES };
    manifest.files = tables
        .iter()
        .chain(["metadata.json", "metrics.json"].iter())
        .map(|n| Artifact::capture(dir, n).unwrap())
        .collect();
    store::write(&dir.join("manifest.json"), &manifest).unwrap();
}

#[test]
fn public_plot_groups_close_path_labels_without_moving_scientific_ticks() {
    let root = temp();
    let work = fixture(&root);
    let source = work.join("interpolation/comparison");
    let mut metadata: Metadata = store::read(&source.join("metadata.json")).unwrap();
    for ticks in [&mut metadata.electronic_ticks, &mut metadata.phonon_ticks] {
        ticks[0].label = "GAMMA".into();
        ticks.insert(2, Tick { distance: 0.505, label: "L".into() });
    }
    store::write(&source.join("metadata.json"), &metadata).unwrap();
    rehash(&source);
    let before = fingerprints(&source);
    success(cli(&["collect", "-w", work.to_str().unwrap()], &root));
    let results = work.join("results");
    success(cli(&["plot", "--results", results.to_str().unwrap()], &root));
    for kind in ["electronic_bands_comparison", "phonon_dispersion_comparison", "comparison_overview"] {
        let svg = fs::read_to_string(results.join(format!("figures/{kind}.svg"))).unwrap();
        let doc = roxmltree::Document::parse(&svg).unwrap();
        let panels = if kind == "comparison_overview" { 2 } else { 1 };
        let labels: Vec<_> = doc.descendants().filter(|n| n.has_tag_name("text")).filter_map(|n| n.text()).map(str::trim).collect();
        assert_eq!(labels.iter().filter(|s| **s == "Γ").count(), panels);
        assert_eq!(labels.iter().filter(|s| **s == "M | L").count(), panels);
        assert!(!labels.contains(&"GAMMA"));
        let lines: Vec<_> = doc.descendants().filter(|n| n.has_tag_name("polyline") && n.attribute("stroke") == Some("#C8C8C8")).collect();
        assert_eq!(lines.len(), panels * 4,
            "显示分组不能删除或移动原高对称点竖线");
        for panel in lines.chunks_exact(4) {
            let xs: Vec<i32> = panel.iter().map(|n| n.attribute("points").unwrap().split(',').next().unwrap().trim().parse().unwrap()).collect();
            for (index, distance) in [(1, 0.5), (2, 0.505)] {
                assert!((xs[index] as f64 - xs[0] as f64 - (xs[3] - xs[0]) as f64 * distance).abs() <= 1.0,
                    "高对称点竖线仍应标记原来的物理路径距离");
            }
        }
    }
    assert_eq!(before, fingerprints(&source), "绘图不能改科学表或元数据");
    fs::remove_dir_all(root).unwrap();
}
fn temp() -> PathBuf {
    let p = std::env::temp_dir().join(format!("epw-comparison-{}", store::token()));
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn partial_format_replot_keeps_ownership_and_interrupted_publication_recovers() {
    let root=temp();
    let work=fixture(&root);
    let results=epwflow::delivery::comparison::collect(&work,None).unwrap();
    let plot=|formats:&[String]| epwflow::delivery::comparison_plot::plot(&results,None,formats).unwrap();
    plot(&["png".into(),"svg".into(),"pdf".into()]);
    let output=results.join("figures");
    let manifest=output.join("comparison-figures.json");
    let previous:serde_json::Value=store::read(&manifest).unwrap();
    let png=store::hash_file(&output.join("comparison_overview.png")).unwrap();
    plot(&["svg".into()]);
    let current:serde_json::Value=store::read(&manifest).unwrap();
    assert_eq!(current["files"].as_array().unwrap().len(),15);
    assert_eq!(png,store::hash_file(&output.join("comparison_overview.png")).unwrap());
    assert!(epwflow::delivery::comparison_plot::check_figure_formats(&results,&["png","svg","pdf"]).unwrap());
    // 使用真实发布协议构造“首张图片已替换、清单尚未写入”的中断。
    let stage=output.join(".native-comparison-interrupted");
    fs::create_dir(&stage).unwrap();
    let mut next=current.clone();
    let name=next["files"][0]["relative_path"].as_str().unwrap().to_string();
    let mut payload=fs::read(output.join(&name)).unwrap();
    payload.extend_from_slice(b"\n");
    fs::write(stage.join(&name),&payload).unwrap();
    next["files"][0]=serde_json::to_value(Artifact::capture(&stage,&name).unwrap()).unwrap();
    for f in next["files"].as_array().unwrap().iter().skip(1) {
        let name=f["relative_path"].as_str().unwrap();
        fs::copy(output.join(name),stage.join(name)).unwrap();
    }
    fs::rename(stage.join(&name),output.join(&name)).unwrap();
    let mut encoded=serde_json::to_vec_pretty(&next).unwrap();encoded.push(b'\n');
    store::write(&output.join(".comparison-figures.json.publish.json"),&serde_json::json!({
        "schema_version":1,"stage":".native-comparison-interrupted","manifest":"comparison-figures.json",
        "previous_digest":store::hash_file(&manifest).unwrap(),"payload":encoded,
        "files":next["files"],"previous_files":current["files"]
    })).unwrap();
    assert!(epwflow::delivery::comparison_plot::check_figures(&results).is_err());
    plot(&["png".into(),"svg".into(),"pdf".into()]);
    assert!(epwflow::delivery::comparison_plot::check_figure_formats(&results,&["png","svg","pdf"]).unwrap());
    assert!(!stage.exists());
    assert!(!output.join(".comparison-figures.json.publish.json").exists());
    assert_eq!(previous["comparison_digest"],current["comparison_digest"]);
    fs::remove_dir_all(root).unwrap();
}
fn cli(args: &[&str], cwd: &Path) -> Output {
    stage_cli::command(args, cwd)
        .current_dir(cwd)
        .env("PATH", cwd.join("traps"))
        .output()
        .unwrap()
}
fn success(o: Output) -> Output {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    o
}
fn fingerprints(dir: &Path) -> Vec<String> {
    TABLES
        .iter()
        .chain(["metadata.json", "metrics.json", "manifest.json"].iter())
        .map(|n| store::hash_file(&dir.join(n)).unwrap())
        .collect()
}

#[test]
fn error_markers_validate_semantics_and_old_metadata_remains_readable() {
    let root = temp();
    let work = fixture(&root);
    let dir = work.join("interpolation/comparison");
    let original: Metadata = store::read(&dir.join("metadata.json")).unwrap();
    let mut legacy = serde_json::to_value(&original).unwrap();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("phonon_error_markers");
    legacy
        .as_object_mut()
        .unwrap()
        .remove("disentanglement_window_ev");
    legacy.as_object_mut().unwrap().remove("frozen_window_ev");
    store::write(&dir.join("metadata.json"), &legacy).unwrap();
    rehash(&dir);
    assert!(comparison::load(&dir)
        .unwrap()
        .metadata
        .phonon_error_markers
        .is_empty());
    for case in 0..4 {
        let mut invalid = original.clone();
        match case {
            0 => invalid.phonon_error_markers[0].path_label = Some("M".into()),
            1 => invalid.phonon_error_markers[0].mode_sorted = 0,
            2 => invalid.phonon_error_markers[1].path_distance = Some(0.333333),
            _ => invalid.phonon_error_markers[1].scope = "sampled_maximum".into(),
        }
        store::write(&dir.join("metadata.json"), &invalid).unwrap();
        rehash(&dir);
        assert!(
            comparison::load(&dir).is_err(),
            "标注语义错误 {case} 未被拒绝"
        );
    }
}

/// 检查实际 SVG 的标签位置，包含总览子面板的坐标偏移。
fn check_path_axes(svg: &str, expected_ticks: usize) {
    let doc = roxmltree::Document::parse(svg).unwrap();
    let texts: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("text"))
        .map(|n| {
            (
                n.attribute("x").unwrap().parse::<i32>().unwrap(),
                n.attribute("y").unwrap().parse::<i32>().unwrap(),
                n.text().unwrap_or("").trim(),
            )
        })
        .collect();
    let lines: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("polyline") && n.attribute("stroke") == Some("#C8C8C8"))
        .collect();
    assert_eq!(lines.len(), expected_ticks);
    for line in lines {
        let coords: Vec<i32> = line
            .attribute("points")
            .unwrap()
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect();
        let x = coords[0];
        let bottom = coords[1].max(coords[3]);
        let labels: Vec<_> = texts
            .iter()
            .filter(|&&(tx, _, text)| tx == x && ["Γ", "M", "K"].contains(&text))
            .collect();
        assert!(
            labels
                .iter()
                .any(|&&(_, y, _)| y > bottom && y < bottom + 40),
            "高对称符号必须在底部轴外：x={x}, bottom={bottom}, labels={labels:?}"
        );
        assert!(
            !texts.iter().any(|&(tx, y, text)| tx == x
                && y > bottom
                && y < bottom + 40
                && text.parse::<f64>().is_ok()),
            "高对称点处不能显示路径数值"
        );
    }
    assert!(!svg.contains("k path"));
    assert!(!svg.contains("q path"));
    // 自动生成的数值横轴标签采用居中锚点；路径图不得保留任意数值刻度。
    if expected_ticks == 3 {
        assert!(!doc.descendants().any(|n| n.has_tag_name("text")
            && n.attribute("text-anchor") == Some("middle")
            && n.text().unwrap_or("").trim().parse::<f64>().is_ok()));
    }
}

fn check_electronic_reference_lines(svg: &str, panel: usize, horizontal: bool) {
    let doc = roxmltree::Document::parse(svg).unwrap();
    let points = |n: roxmltree::Node<'_, '_>| -> Vec<(i32, i32)> {
        n.attribute("points")
            .unwrap()
            .split_whitespace()
            .map(|p| {
                let (x, y) = p.split_once(',').unwrap();
                (x.parse().unwrap(), y.parse().unwrap())
            })
            .collect()
    };
    let axes: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("polyline") && n.attribute("stroke") == Some("#000000"))
        .map(points)
        .filter(|p| p.len() == 2 && p[0].1 == p[1].1 && p[1].0 - p[0].0 > 100)
        .collect();
    let bottom = &axes[panel];
    let top = doc
        .descendants()
        .filter(|n| n.has_tag_name("polyline") && n.attribute("stroke") == Some("#000000"))
        .map(points)
        .find(|p| p.len() == 2 && p[0].0 == bottom[0].0 - 1 && p[1].1 == bottom[0].1 - 1)
        .unwrap()[0]
        .1;
    for (name, color, relative) in [
        ("dis_win_min", "#2772C5", -0.5),
        ("dis_win_max", "#008B8B", 0.5),
        ("dis_froz_min", "#E67D22", -0.5),
        ("dis_froz_max", "#AE49A6", 0.2),
        ("E_F", "#2F7D3C", 0.0),
    ] {
        if name == "E_F" {
            assert!(svg.contains("E_F: E - E_F = 0 eV"));
            assert!(doc.descendants().any(|n| n.has_tag_name("text")
                && n.attribute("fill") == Some(color)
                && n.text().unwrap_or("").trim() == "E_F"));
        } else {
            assert!(svg.contains(&format!("{name}: {relative:.3} eV (E-EF)")));
        }
        let position = if horizontal {
            (bottom[0].1 - 1) as f64 - (relative + 0.5) * (bottom[0].1 - 1 - top) as f64
        } else {
            bottom[0].0 as f64 + (relative + 0.5) * (bottom[1].0 - bottom[0].0) as f64
        };
        let segments = doc
            .descendants()
            .filter(|n| n.has_tag_name("polyline") && n.attribute("stroke") == Some(color))
            .map(points)
            .filter(|p| p.len() == 2)
            .filter(|p| {
                if horizontal {
                    p[0].1 == p[1].1 && (p[0].1 as f64 - position).abs() <= 1.0
                } else {
                    p[0].0 == p[1].0 && (p[0].0 as f64 - position).abs() <= 1.0
                }
            })
            .count();
        assert!(segments >= 3, "{name} 必须在正确能量位置画出多段彩色虚线");
    }
}

#[test]
fn cli_collect_plot_move_and_replot_without_f4_or_external_calls() {
    let root = if let Ok(p) = std::env::var("EPWFLOW_PREVIEW_DIR") {
        let p = PathBuf::from(p);
        fs::create_dir_all(&p).unwrap();
        p
    } else {
        temp()
    };
    let work = fixture(&root);
    let source = work.join("interpolation/comparison");
    let before = fingerprints(&source);
    let traps = root.join("traps");
    fs::create_dir_all(&traps).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for name in ["pw.x", "ph.x", "epw.x", "sbatch"] {
            let f = traps.join(name);
            fs::write(
                &f,
                format!(
                    "#!/bin/sh\n/bin/touch '{}'\nexit 99\n",
                    root.join("external-called").display()
                ),
            )
            .unwrap();
            fs::set_permissions(&f, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    success(cli(&["collect", "-w", work.to_str().unwrap()], &root));
    // 相同输入重复收集幂等，不改写科学表。
    success(cli(&["collect", "-w", work.to_str().unwrap()], &root));
    let results = work.join("results");
    success(cli(
        &["plot", "--results", results.to_str().unwrap()],
        &root,
    ));
    let figures = results.join("figures");
    for kind in KINDS {
        assert!(fs::read(figures.join(format!("{kind}.png")))
            .unwrap()
            .starts_with(b"\x89PNG"));
        assert!(fs::read(figures.join(format!("{kind}.pdf")))
            .unwrap()
            .starts_with(b"%PDF"));
        let svg = fs::read_to_string(figures.join(format!("{kind}.svg"))).unwrap();
        assert!(svg.contains("SYNTHETIC STYLE TEST"));
        if kind == "electronic_bands_comparison" || kind == "comparison_overview" {
            check_electronic_reference_lines(&svg, 0, true);
        }
        if kind == "electronic_dos_comparison" || kind == "comparison_overview" {
            check_electronic_reference_lines(
                &svg,
                usize::from(kind == "comparison_overview"),
                false,
            );
        }
        if kind == "phonon_dispersion_comparison" || kind == "phonon_dos_comparison" {
            assert!(!svg.contains("E_F: E - E_F = 0 eV"));
        }
        match kind {
            "electronic_bands_comparison" | "phonon_dispersion_comparison" => {
                check_path_axes(&svg, 3);
            }
            "comparison_overview" => check_path_axes(&svg, 6),
            _ => {
                let doc = roxmltree::Document::parse(&svg).unwrap();
                assert!(
                    doc.descendants().any(|n| n.has_tag_name("text")
                        && n.attribute("text-anchor") == Some("middle")
                        && n.text().unwrap_or("").trim().parse::<f64>().is_ok()),
                    "DOS 必须保留物理横轴的数值刻度"
                );
            }
        }
        assert!(svg.contains("dis_win_min = 6.50000; dis_win_max = 7.50000"));
        assert!(svg.contains("electronic window = [-0.500, 0.500] eV relative to EF"));
        let qe = svg.find("#2D2D37").unwrap();
        let epw = svg.find("#D34131").unwrap();
        assert!(qe < epw, "EPW 必须后绘");
        if kind == "phonon_dispersion_comparison" || kind == "comparison_overview" {
            assert!(svg.contains("#8A2BE2"));
            assert!(svg.contains("Sampled maximum (OFF PATH): 49.00"));
            assert!(svg.contains("mode 7 (frequency sorted)"));
            assert!(svg.contains("(1/12, 1/12, 1/12)"));
            assert!(svg.contains("M (high-symmetry point)"));
        }
    }
    let moved = root.join("portable-results");
    fs::rename(&results, &moved).unwrap();
    let o = success(cli(
        &[
            "results",
            "check",
            "--results",
            moved.to_str().unwrap(),
            "--json",
        ],
        &root,
    ));
    let report: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(report["workflow_complete"], false);
    assert_eq!(report["comparison_complete"], false);
    success(cli(
        &[
            "plot",
            "--results",
            moved.to_str().unwrap(),
            "--output",
            root.join("redraw").to_str().unwrap(),
        ],
        &root,
    ));
    assert_eq!(before, fingerprints(&source));
    assert_eq!(before, fingerprints(&moved.join("comparison")));
    assert!(!root.join("external-called").exists());
    fs::create_dir_all(root.join("conflict")).unwrap();
    fs::write(
        root.join("conflict/electronic_bands_comparison.png"),
        "user data",
    )
    .unwrap();
    assert!(!cli(
        &[
            "plot",
            "--results",
            moved.to_str().unwrap(),
            "--output",
            root.join("conflict").to_str().unwrap()
        ],
        &root
    )
    .status
    .success());
    assert_eq!(
        fs::read_to_string(root.join("conflict/electronic_bands_comparison.png")).unwrap(),
        "user data"
    );
    println!(
        "style_preview={}",
        moved.join("figures/comparison_overview.png").display()
    );
}

#[test]
fn malformed_units_axes_projections_versions_and_tampering_are_rejected() {
    for case in [
        "tamper",
        "unit",
        "axis",
        "projection",
        "version",
        "nan",
        "reference",
        "complete",
        "traversal",
    ] {
        let root = temp();
        let work = fixture(&root);
        let dir = work.join("interpolation/comparison");
        match case {
            "tamper" => {
                fs::write(dir.join(TABLES[0]), "changed").unwrap();
            }
            "unit" | "projection" => {
                let mut m: Metadata = store::read(&dir.join("metadata.json")).unwrap();
                if case == "unit" {
                    m.phonon_frequency_unit = "meV".into()
                } else {
                    m.electronic_projection = "bond_center_WF".into()
                };
                store::write(&dir.join("metadata.json"), &m).unwrap();
                rehash(&dir);
            }
            "axis" | "nan" | "reference" => {
                let name = if case == "reference" {
                    TABLES[1]
                } else {
                    TABLES[3]
                };
                let text = fs::read_to_string(dir.join(name)).unwrap();
                let text = match case {
                    "axis" => text.replacen("epw,-0.5,", "epw,-0.499,", 1),
                    "nan" => text.replacen("qe,-0.5,", "qe,NaN,", 1),
                    _ => text.replacen("qe_matdyn", "qe_ph_direct", 1),
                };
                fs::write(dir.join(name), text).unwrap();
                rehash(&dir);
            }
            _ => {
                let mut m: Manifest = store::read(&dir.join("manifest.json")).unwrap();
                match case {
                    "version" => m.schema_version = 99,
                    "complete" => {
                        m.missing.clear();
                        fs::write(
                            dir.join(TABLES[4]),
                            "method,frequency_cm1,projection_kind,element,density\n",
                        )
                        .unwrap();
                    }
                    _ => m.files[0].relative_path = "../escape".into(),
                };
                store::write(&dir.join("manifest.json"), &m).unwrap();
                if case == "complete" {
                    rehash(&dir);
                }
            }
        }
        assert!(comparison::load(&dir).is_err(), "未拒绝 {case}");
    }
}

#[test]
fn gaussian_weights_units_and_frequency_sign_preserve_state_count() {
    let axis: Vec<_> = (0..20001).map(|i| -100.0 + i as f64 * 0.01).collect();
    let states = vec![vec![-15.0, 10.0, 30.0], vec![-13.0, 12.0, 32.0]];
    let density = gaussian_dos(&axis, &states, &[0.25, 0.75], 1.0).unwrap();
    let integral: f64 = density.windows(2).map(|y| (y[0] + y[1]) * 0.005).sum();
    assert!((integral - 3.0).abs() < 1e-10);
    // 换轴必须同时除密度；不能取虚频绝对值再累计。
    let factor = 8.065543937;
    let converted: Vec<_> = axis.iter().map(|x| x * factor).collect();
    let converted_density: Vec<_> = density.iter().map(|y| y / factor).collect();
    let integral: f64 = converted
        .windows(2)
        .zip(converted_density.windows(2))
        .map(|(x, y)| (x[1] - x[0]) * (y[1] + y[0]) * 0.5)
        .sum();
    assert!((integral - 3.0).abs() < 1e-10);
    let negative = axis
        .iter()
        .zip(&density)
        .filter(|(x, _)| **x < 0.0)
        .map(|(_, y)| y * 0.01)
        .sum::<f64>();
    assert!((negative - 1.0).abs() < 1e-6);
    assert!(gaussian_dos(&axis, &states, &[1.0], 1.0).is_err());
    assert!(gaussian_dos(&axis, &states, &[0.5, 0.5], 0.0).is_err());
}

#[test]
fn native_export_reports_four_maxima_and_checks_nonorthogonal_mesh() {
    use epwflow::interpolation::{
        comparison::{PathPoint, Sources, SourceFile},
    };
    let root = temp();
    let xml = |points: &[[f64; 3]]| {
        let ks:String=points.iter().map(|p|format!("<ks_energies><k_point weight='1'>{} {} {}</k_point><eigenvalues size='1'>0</eigenvalues></ks_energies>",p[0],p[1],p[2])).collect();
        format!("<output><atomic_structure nat='3'><atomic_positions><atom name='Mg'>0 0 0</atom><atom name='B'>1 0 0</atom><atom name='B'>0 1 0</atom></atomic_positions></atomic_structure><band_structure><nbnd>1</nbnd><nks>{}</nks><fermi_energy>0</fermi_energy>{ks}</band_structure></output>",points.len())
    };
    let bands = |points: &[[f64; 3]], values: &[f64]| {
        let text: String = points
            .iter()
            .map(|p| {
                format!(
                    "{} {} {}\n{}\n",
                    p[0],
                    p[1],
                    p[2],
                    values
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            })
            .collect();
        format!(
            "&plot nbnd= {}, nks= {} /\n{text}",
            values.len(),
            points.len()
        )
    };
    fs::write(root.join("fermi.xml"), xml(&[[0.; 3]])).unwrap();
    let frequencies: Vec<_> = (0..9).map(|i| 100.0 + i as f64 * 100.0).collect();
    fs::write(root.join("plan.txt"), "SYNTHETIC native adapter test").unwrap();
    let file = |name: &str| SourceFile {
        path: name.into(),
        sha256: store::hash_file(&root.join(name)).unwrap(),
    };
    let path = vec![[0., 0., 0.], [0.25, 0., 0.], [0.5, 0., 0.]];
    let mesh = vec![[0., 0., 0.], [0.5, 0., 0.]];
    let cart = |p: &[f64; 3]| [p[0], p[0] * 0.5, 0.0];
    let p_cart: Vec<_> = path.iter().map(cart).collect();
    let m_cart: Vec<_> = mesh.iter().map(cart).collect();
    let all: Vec<_> = p_cart.iter().chain(&m_cart).copied().collect();
    fs::write(root.join("qe-path.xml"), xml(&p_cart)).unwrap();
    fs::write(root.join("qe-mesh.xml"), xml(&m_cart)).unwrap();
    fs::write(root.join("epw-path.eig"), bands(&all, &[0.1])).unwrap();
    let matdyn = bands(&all, &frequencies)
        .lines()
        .map(|line| {
            if line.split_whitespace().count() == 3 {
                format!("{line} 0\n")
            } else {
                format!("{line}\n")
            }
        })
        .collect::<String>();
    fs::write(root.join("qe-path.freq"), matdyn).unwrap();
    fs::write(
        root.join("epw-path.freq"),
        bands(
            &all,
            &frequencies
                .iter()
                .map(|v| (v + 60.) / 8.065543937)
                .collect::<Vec<_>>(),
        ),
    )
    .unwrap();
    fs::write(
        root.join("epw.out"),
        "Program EPW v.5.9 \nTotal program execution\n EPW : 1s CPU 1s WALL\n",
    )
    .unwrap();
    let mut sources = Sources {
        schema_version: 2,
        run_id: "native-fixture".into(),
        ph_mode: "index".into(),
        fermi_reference: file("fermi.xml"),
        qe_bands_xml: file("qe-path.xml"),
        qe_mesh_xml: file("qe-mesh.xml"),
        epw_bands: file("epw-path.eig"),
        qe_phonon_bands: file("qe-path.freq"),
        epw_phonon_bands: file("epw-path.freq"),
        epw_log: file("epw.out"),
        provenance: vec![file("plan.txt")],
        path: path
            .into_iter()
            .enumerate()
            .map(|(i, crystal)| PathPoint {
                segment_id: 0,
                point_id: i,
                distance: i as f64 * 0.2795084971874737,
                crystal,
            })
            .collect(),
        ticks: vec![],
        mesh,
        mesh_shape: [2, 1, 1],
        reciprocal: [[1., 0.5, 0.], [0., 1.2, 0.], [0., 0., 0.9]],
        window_ev: [-0.5, 0.5],
        electronic_sigma_ev: 0.1,
        wannier_wout: None,
        phonon_sigma_cm1: 4.0,
    };
    let config = root.join("sources.json");
    store::write(&config, &sources).unwrap();
    let output = root.join("exported");
    success(cli(
        &[
            "interpolation",
            "export-comparison",
            "--source",
            config.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ],
        &root,
    ));
    let data = comparison::load(&output).unwrap();
    assert!(data.manifest.quality_status.is_none());
    assert!(data.direct.is_empty());
    assert!(data.metadata.phonon_error_markers.is_empty());
    let differences = data.differences.as_ref().unwrap();
    for (stat, expected) in [
        (&differences.electronic_path, 0.1),
        (&differences.electronic_mesh, 0.1),
        (&differences.phonon_path, 60.),
        (&differences.phonon_mesh, 60.),
    ] {
        assert!((stat.maximum.as_ref().unwrap().absolute_difference - expected).abs() < 1e-8);
        assert_eq!(stat.maximum.as_ref().unwrap().crystal, [0.; 3]);
    }
    assert_eq!(differences.electronic_path.paired_states, 3);
    assert_eq!(differences.phonon_mesh.paired_states, 18);
    // 重新登记哈希也不能让自相矛盾的最大差值通过语义校验。
    let saved: serde_json::Value = store::read(&output.join("metrics.json")).unwrap();
    let mut invalid = saved.clone();
    invalid["differences"]["electronic_path"]["maximum"]["absolute_difference"] = 0.into();
    store::write(&output.join("metrics.json"), &invalid).unwrap();
    rehash(&output);
    assert!(comparison::load(&output).is_err());
    store::write(&output.join("metrics.json"), &saved).unwrap();
    rehash(&output);
    let metrics: serde_json::Value = store::read(&output.join("metrics.json")).unwrap();
    assert!((metrics["phonon_dos"]["qe_integral"].as_f64().unwrap() - 9.0).abs() < 1e-8);
    assert_eq!(
        store::hash_file(&root.join("epw-path.eig")).unwrap(),
        sources.epw_bands.sha256
    );
    // 实际默认外窗来自原生日志；显示窗内重新统计常规对比数据。
    fs::write(
        root.join("test.wout"),
        " | Outer: -2.0D+0 to 3.0D+0 (eV) |\n | Inner: -2.0D+0 to 2.0D+0 (eV) |\n",
    )
    .unwrap();
    sources.wannier_wout = Some(file("test.wout"));
    store::write(&config, &sources).unwrap();
    let outer = root.join("outer-exported");
    success(cli(
        &[
            "interpolation",
            "export-comparison",
            "--source",
            config.to_str().unwrap(),
            "--output",
            outer.to_str().unwrap(),
        ],
        &root,
    ));
    let wide = comparison::load(&outer).unwrap();
    assert_eq!(wide.metadata.disentanglement_window_ev, Some([-2.0, 3.0]));
    assert_eq!(wide.metadata.frozen_window_ev, Some([-2.0, 2.0]));
    let expected = [-2.0 - wide.metadata.fermi_ev, 3.0 - wide.metadata.fermi_ev];
    assert_eq!(wide.metadata.window_ev, expected);
    assert_eq!(
        wide.electronic_dos[0].points.first().unwrap().0,
        expected[0]
    );
    assert_eq!(wide.electronic_dos[0].points.last().unwrap().0, expected[1]);
    assert_eq!(wide.manifest.quality_status, data.manifest.quality_status);
    let mut invalid = wide.metadata.clone();
    invalid.window_ev[0] -= 1.0;
    store::write(&outer.join("metadata.json"), &invalid).unwrap();
    rehash(&outer);
    assert!(comparison::load(&outer).is_err());
    // 不能只改显示窗元数据、沿用原来窄窗的 DOS 数据。
    invalid.disentanglement_window_ev.as_mut().unwrap()[0] -= 1.0;
    store::write(&outer.join("metadata.json"), &invalid).unwrap();
    rehash(&outer);
    assert!(comparison::load(&outer)
        .err()
        .unwrap()
        .to_string()
        .contains("覆盖"));
    fs::write(root.join("test.wout"), "altered").unwrap();
    assert!(!cli(
        &[
            "interpolation",
            "export-comparison",
            "--source",
            config.to_str().unwrap(),
            "--output",
            root.join("tampered-outer").to_str().unwrap()
        ],
        &root
    )
    .status
    .success());
    sources.wannier_wout = None;
    sources.reciprocal[0][1] = 0.4;
    store::write(&config, &sources).unwrap();
    let o = cli(
        &[
            "interpolation",
            "export-comparison",
            "--source",
            config.to_str().unwrap(),
            "--output",
            root.join("invalid").to_str().unwrap(),
        ],
        &root,
    );
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("坐标"));
    assert!(!root.join("invalid/manifest.json").exists());
    sources.reciprocal[0][1] = 0.5;
    sources.mesh[1] = [0.0; 3];
    store::write(&config, &sources).unwrap();
    assert!(!cli(
        &[
            "interpolation",
            "export-comparison",
            "--source",
            config.to_str().unwrap(),
            "--output",
            root.join("invalid-mesh").to_str().unwrap()
        ],
        &root
    )
    .status
    .success());
    sources.mesh[1]=[0.5,0.,0.];
    sources.window_ev=[2.,3.];
    store::write(&config,&sources).unwrap();
    let empty = root.join("empty-exported");
    let out=cli(&["interpolation","export-comparison","--source",config.to_str().unwrap(),
        "--output",empty.to_str().unwrap()],&root);
    assert!(String::from_utf8_lossy(&out.stdout).contains("无可配对数据"));
    success(out);
    let data=comparison::load(&empty).unwrap();
    let stats=data.differences.as_ref().unwrap();
    assert!(stats.electronic_path.maximum.is_none());
    assert_eq!(stats.electronic_path.paired_states,0);
    let results=root.join("portable-empty");
    fs::create_dir_all(root.join("interpolation")).unwrap();
    fs::rename(&empty,root.join("interpolation/comparison")).unwrap();
    epwflow::delivery::comparison::collect(&root,None).unwrap();
    fs::rename(root.join("results"),&results).unwrap();
    success(cli(&["plot","--results",results.to_str().unwrap()],&root));
    assert!(epwflow::delivery::comparison_plot::check_figure_formats(&results,&["png","svg","pdf"]).unwrap());
    let svg=fs::read_to_string(results.join("figures/comparison_overview.svg")).unwrap();
    assert!(svg.contains("no paired data"));
    assert!(svg.contains("DOS mesh"));
    assert!(!svg.contains("quality:"));
    if let Ok(destination)=std::env::var("DFTFLOW_COMPARISON_VISUAL_ROOT") {
        fs::write(Path::new(&destination),results.to_string_lossy().as_bytes()).unwrap();
    }

}

#[test]
fn outer_window_requires_one_finite_ordered_native_record() {
    use epwflow::interpolation::comparison::parse_outer_window;
    assert_eq!(
        parse_outer_window(" | Outer: -4.90710 to 32.23753 (eV) |\n").unwrap(),
        [-4.90710, 32.23753]
    );
    for text in [
        "",
        " | Outer: NaN to 3 (eV)",
        " | Outer: 3 to 2 (eV)",
        " | Outer: 2 to 2 (eV)",
        " | Outer: -1 to 3 (eV)\n | Outer: -1 to 3 (eV)",
    ] {
        assert!(parse_outer_window(text).is_err(), "未拒绝 {text}");
    }
}

#[test]
fn frozen_window_rejects_invalid_records_and_metadata_without_inventing_legacy_bounds() {
    use epwflow::interpolation::comparison::parse_frozen_window;
    assert_eq!(
        parse_frozen_window(" | Outer: -2 to 3 (eV) |\n").unwrap(),
        None
    );
    assert_eq!(
        parse_frozen_window(" | Inner: -2.0D+0 to 1.0D+0 (eV) |\n").unwrap(),
        Some([-2.0, 1.0])
    );
    for text in [
        " | Inner: NaN to 3 (eV)",
        " | Inner: -1 to inf (eV)",
        " | Inner: 3 to 2 (eV)",
        " | Inner: 2 to 2 (eV)",
        " | Inner: -1 to 3 (Ry)",
        " | Inner: broken",
        " | Inner: -1 to 3 (eV)\n | Inner: -1 to 3 (eV)",
    ] {
        assert!(parse_frozen_window(text).is_err(), "未拒绝 {text}");
    }
    let root = temp();
    let work = fixture(&root);
    let dir = work.join("interpolation/comparison");
    let original: Metadata = store::read(&dir.join("metadata.json")).unwrap();
    for frozen in [[6.4, 7.2], [6.8, 7.6], [7.2, 6.8], [7.2, 7.2]] {
        let mut metadata = original.clone();
        metadata.frozen_window_ev = Some(frozen);
        store::write(&dir.join("metadata.json"), &metadata).unwrap();
        rehash(&dir);
        assert!(comparison::load(&dir).is_err(), "未拒绝 {frozen:?}");
    }
    let mut metadata = original.clone();
    metadata.disentanglement_window_ev = None;
    store::write(&dir.join("metadata.json"), &metadata).unwrap();
    rehash(&dir);
    assert!(comparison::load(&dir).is_err());
    metadata = original;
    metadata.frozen_window_ev = None;
    store::write(&dir.join("metadata.json"), &metadata).unwrap();
    rehash(&dir);
    success(cli(&["collect", "-w", work.to_str().unwrap()], &root));
    success(cli(
        &[
            "plot",
            "--results",
            work.join("results").to_str().unwrap(),
            "--formats",
            "svg",
        ],
        &root,
    ));
    let svg =
        fs::read_to_string(work.join("results/figures/electronic_bands_comparison.svg")).unwrap();
    assert!(svg.contains("Frozen window not recorded"));
    assert!(!svg.contains("dis_froz_min:"));
    assert!(svg.contains("dis_win_min:"));
}
