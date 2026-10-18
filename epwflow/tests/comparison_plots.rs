//! 经真实 CLI 验证叠图、可移植重绘和拒绝无效来源；样式数据明确为合成。
use epwflow::{
    delivery::comparison::{self, Artifact, Manifest, Metadata, Tick, KINDS, TABLES},
    foundation::store,
    interpolation::comparison::gaussian_dos,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

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
        quality_status: "unverified".into(),
        missing: vec!["SYNTHETIC TEST DATA - no scientific acceptance".into()],
        files: vec![],
    };
    store::write(&dir.join("manifest.json"), &manifest).unwrap();
    rehash(&dir);
    work
}
fn rehash(dir: &Path) {
    let mut manifest: Manifest = store::read(&dir.join("manifest.json")).unwrap();
    manifest.files = TABLES
        .iter()
        .chain(["metadata.json", "metrics.json"].iter())
        .map(|n| Artifact::capture(dir, n).unwrap())
        .collect();
    store::write(&dir.join("manifest.json"), &manifest).unwrap();
}
fn temp() -> PathBuf {
    let p = std::env::temp_dir().join(format!("epw-comparison-{}", store::token()));
    fs::create_dir_all(&p).unwrap();
    p
}
fn cli(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_epwflow"))
        .args(args)
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
        let qe = svg.find("#2D2D37").unwrap();
        let epw = svg.find("#D34131").unwrap();
        assert!(qe < epw, "EPW 必须后绘");
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
fn native_export_preserves_failed_quality_and_checks_nonorthogonal_mesh() {
    use epwflow::interpolation::{
        comparison::{PathPoint, Sources},
        native_quality::{QualitySources, SourceFile},
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
    let zeros = vec![[0.0; 3]; 6];
    fs::write(root.join("quality.xml"), xml(&zeros)).unwrap();
    fs::write(root.join("quality.eig"), bands(&zeros, &[0.0])).unwrap();
    let frequencies: Vec<_> = (0..9).map(|i| 100.0 + i as f64 * 10.0).collect();
    let interpolated: Vec<_> = frequencies
        .iter()
        .map(|v| (v + 50.0) / 8.065543937)
        .collect();
    fs::write(
        root.join("quality-qe.freq"),
        bands(&zeros[..4], &frequencies),
    )
    .unwrap();
    fs::write(
        root.join("quality-epw.freq"),
        bands(&zeros[..4], &interpolated),
    )
    .unwrap();
    fs::write(root.join("plan.txt"), "SYNTHETIC native adapter test").unwrap();
    let file = |name: &str| SourceFile {
        path: name.into(),
        sha256: store::hash_file(&root.join(name)).unwrap(),
    };
    let quality = QualitySources {
        schema_version: 1,
        asr: "epw_simple_gamma_onsite".into(),
        window_ev: [-0.5, 0.5],
        fermi_reference: file("quality.xml"),
        electron_reference: file("quality.xml"),
        electron_interpolated: file("quality.eig"),
        phonon_reference_cm1: file("quality-qe.freq"),
        phonon_interpolated_mev: file("quality-epw.freq"),
        provenance: vec![file("plan.txt")],
    };
    store::write(&root.join("quality-reference.json"), &quality).unwrap();
    let path = vec![[0., 0., 0.], [0.25, 0., 0.], [0.5, 0., 0.]];
    let mesh = vec![[0., 0., 0.], [0.5, 0., 0.]];
    let cart = |p: &[f64; 3]| [p[0], p[0] * 0.5, 0.0];
    let p_cart: Vec<_> = path.iter().map(cart).collect();
    let m_cart: Vec<_> = mesh.iter().map(cart).collect();
    let all: Vec<_> = p_cart.iter().chain(&m_cart).copied().collect();
    fs::write(root.join("qe-path.xml"), xml(&p_cart)).unwrap();
    fs::write(root.join("qe-mesh.xml"), xml(&m_cart)).unwrap();
    fs::write(root.join("epw-path.eig"), bands(&all, &[0.0])).unwrap();
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
                .map(|v| v / 8.065543937)
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
        schema_version: 1,
        run_id: "native-fixture".into(),
        ph_mode: "index".into(),
        quality_reference: file("quality-reference.json"),
        fermi_reference: file("quality.xml"),
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
    assert_eq!(data.manifest.quality_status, "failed");
    assert!(!data.manifest.missing.is_empty());
    assert!(!data.direct.is_empty());
    let metrics: serde_json::Value = store::read(&output.join("metrics.json")).unwrap();
    assert!((metrics["phonon_dos"]["qe_integral"].as_f64().unwrap() - 9.0).abs() < 1e-8);
    assert_eq!(
        store::hash_file(&root.join("epw-path.eig")).unwrap(),
        sources.epw_bands.sha256
    );
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
}
