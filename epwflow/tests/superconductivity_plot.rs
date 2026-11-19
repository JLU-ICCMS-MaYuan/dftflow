//! 五类图的生产入口测试；合成契约样本与本机真实数据分开验收。
use epwflow::superconductivity::plot::{self, io, DataFiles, PlotFormat, PlotTask};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Case {
    root: PathBuf,
    work: PathBuf,
}
impl Case {
    fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../bin/.build/sc-plot-integration")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        let work = root.join("run/8.epw-superconductivity/01.grid/aniso-mu0.1-Tc10-30");
        fs::create_dir_all(&work).unwrap();
        Self { root, work }
    }
    fn write(&self, name: &str, text: &str) {
        fs::write(self.work.join(name), text).unwrap();
    }
    fn fixture() -> Self {
        let c = Self::new();
        c.write("Test.lambda_pairs", "0.1 1\n0.2 2\n0.9 3\n1.1 4\n");
        c.write("Test.lambda_k_pairs", "2 1.0\n1 0.5\n");
        c.write(
            "Test.a2f",
            "# w a2f lambda\n0 0 0\n10 0.5 0.2\n20 1 0.9\n30 0 1\n",
        );
        for (t, g) in [(10., 0.012), (20., 0.008), (30., 0.001)] {
            let w = std::f64::consts::PI * 8.617333262e-5 * t;
            let text = format!(
                "{w} -0.1 2 {g}\n{w} 0.1 2 {}\n{} -0.1 2 {}\n{} 0.1 2 {}\n",
                g / 2.,
                3. * w,
                g / 2.,
                3. * w,
                g / 4.
            );
            c.write(&format!("Test.imag_aniso_{t:06.2}"), &text);
            c.write(
                &format!("Test.imag_aniso_gap0_{t:06.2}"),
                &format!("# distribution = rho(delta_nk)\n{t} {} {t} 0 0\n{} {} {t} 1 3\n{t} {} {t} 0 0\n", g * 500., t + 1., g * 1000., g * 1500.),
            );
            c.write(
                &format!("Test.imag_iso_{t:06.2}"),
                &format!("{w} 2 {g}\n{} 2 {}\n", 3. * w, g / 2.),
            );
        }
        c.write(
            "Test.pade_aniso_010.00",
            "# omega Z Z gap gap\n0 1 0 0.012 0\n0.01 1 0 0.008 0\n0.02 1 0 -0.001 0\n",
        );
        c.write(
            "Test.qdos_010.00",
            "# omega(meV) DOS\n-10 2\n0 0\n10 4\n20 2\n",
        );
        c.write("epw1.out", "DOS(states/spin/eV/Unit Cell) = 2.0\n");
        c
    }
    fn task(&self) -> PlotTask {
        PlotTask::mirror_to_results(&self.work, &self.root.join("run/results")).unwrap()
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
const NAMES: [&str; 5] = [
    "coupling_strength",
    "gap_imaginary_real",
    "aniso_gap_vs_T",
    "iso_gap_vs_T",
    "superconducting_dos",
];
#[test]
fn five_figures_mirror_svg_styles_and_physics() {
    let c = Case::fixture();
    let task = c.task();
    let r = task.execute().unwrap();
    assert_eq!(r.files.len(), 5, "{:?}", r.warnings);
    assert!(task
        .result_dir
        .ends_with("run/results/8.epw-superconductivity/01.grid/aniso-mu0.1-Tc10-30"));
    for name in NAMES {
        let text = fs::read_to_string(task.result_dir.join(format!("{name}.svg"))).unwrap();
        assert!(text.contains("<svg") && text.contains("</svg>"));
        assert!(text.len() > 1000);
    }
    let fig1 = fs::read_to_string(task.result_dir.join(format!("{}.svg", NAMES[0]))).unwrap();
    assert!(fig1.contains("width=\"1800\"") && fig1.matches("<g").count() >= 3);
    assert!(fig1.contains("#DC143C"));
    let fig2 = fs::read_to_string(task.result_dir.join(format!("{}.svg", NAMES[1]))).unwrap();
    assert!(fig2.contains("#808080") && fig2.contains("opacity=\"0.3\""));
    let panels = plot::dos::plot_superconducting_dos(&task).unwrap().unwrap();
    assert_eq!(
        panels[0].curves[0].points,
        [(-10., 1.), (0., 0.), (10., 2.), (20., 1.)]
    );
    let panels = plot::gap_temperature::plot_aniso_gap_vs_t(&task)
        .unwrap()
        .unwrap();
    assert_eq!(panels[0].curves.len(), 3);
    for curve in &panels[0].curves {
        assert_eq!(curve.points.len(), 3);
        assert_eq!(curve.points[1].0 - curve.points[0].0, 1.);
        assert!(curve.points.windows(2).all(|p| p[1].1 > p[0].1));
    }
    let panels = plot::gap_temperature::plot_iso_gap_vs_t(&task)
        .unwrap()
        .unwrap();
    assert_eq!(panels[0].curves.len(), 1);
}
#[test]
fn missing_files_skip_subpanel_and_fermi_dos_falls_back() {
    let c = Case::fixture();
    fs::remove_file(c.work.join("Test.lambda_pairs")).unwrap();
    fs::remove_file(c.work.join("epw1.out")).unwrap();
    let task = c.task();
    let r = task.execute().unwrap();
    assert_eq!(r.files.len(), 5);
    assert!(r.warnings.iter().any(|s| s.contains("missing lambda")));
    assert!(r.warnings.iter().any(|s| s.contains("N_F=1.0")));
    let panels = plot::dos::plot_superconducting_dos(&task).unwrap().unwrap();
    assert_eq!(panels[0].curves[0].points[0], (-10., 2.));
    assert!(fs::read_to_string(task.result_dir.join("plot.log"))
        .unwrap()
        .contains("N_F=1.0"));
}
#[test]
fn malformed_file_does_not_stop_other_figures() {
    let c = Case::fixture();
    c.write("Test.a2f", "0 NaN 0\n");
    c.write("Test.imag_aniso_020.00", "broken\n");
    let r = c.task().execute().unwrap();
    assert_eq!(r.files.len(), 4);
    assert!(r.warnings.iter().any(|s| s.contains("invalid a2f")));
    let c = Case::new();
    let r = c.task().execute().unwrap();
    assert!(r.files.is_empty());
    assert!(r.warnings.iter().any(|w| w.contains("fig1: missing a2f")));
    assert!(r.warnings.iter().any(|w| w.contains("fig5: missing qdos")));
}
#[test]
fn svg_png_pdf_use_shared_exporter() {
    let c = Case::fixture();
    let mut task = c.task();
    task.formats = vec![PlotFormat::Svg, PlotFormat::Png, PlotFormat::Pdf];
    let r = task.execute().unwrap();
    assert_eq!(r.files.len(), 15, "{:?}", r.warnings);
    let png = fs::read(task.result_dir.join(format!("{}.png", NAMES[0]))).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
    let reader = png::Decoder::new(std::io::Cursor::new(&png))
        .read_info()
        .unwrap();
    assert_eq!(reader.info().pixel_dims.unwrap().xppu, 11811);
    let pdf = fs::read(task.result_dir.join(format!("{}.pdf", NAMES[0]))).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
}
#[test]
fn plot_only_reads_no_scientific_configuration_and_changes_no_inputs() {
    let c = Case::fixture();
    let config = c.root.join("input.toml");
    fs::write(
        &config,
        "work_dir = 'run'\n[plot]\nformats = ['svg']\nelectronic_energy_min_ev = -3\n",
    )
    .unwrap();
    let before = fs::read(c.work.join("Test.imag_aniso_010.00")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_epw_superconductivity"))
        .args(["--plot-only", "--config"])
        .arg(&config)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("superconductivity_plots: 5"));
    assert!(!c.root.join("run/config").exists());
    assert!(!c.root.join("run/epw-state").exists());
    assert_eq!(
        before,
        fs::read(c.work.join("Test.imag_aniso_010.00")).unwrap()
    );
    for extra in ["--skip-plot", "--run", "--submit", "--check", "--status"] {
        let result = Command::new(env!("CARGO_BIN_EXE_epw_superconductivity"))
            .args(["--plot-only", extra, "--config"])
            .arg(&config)
            .output()
            .unwrap();
        assert!(!result.status.success(), "{extra}");
    }
}
#[test]
fn real_mgb2_available_data_and_gaps_are_not_fabricated() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/MgB2/ubuntu/run/8.epw-superconductivity");
    if !root.is_dir() {
        eprintln!("Local MgB2 data unavailable; real-data validation not run");
        return;
    }
    let work =
        root.join("01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV");
    let c = Case::new();
    let task = PlotTask::from_work_dir(&work, c.root.join("real")).unwrap();
    let r = task.execute().unwrap();
    assert!(r
        .files
        .iter()
        .any(|p| p.file_name().unwrap() == "gap_imaginary_real.svg"));
    assert!(r
        .files
        .iter()
        .any(|p| p.file_name().unwrap() == "aniso_gap_vs_T.svg"));
    if task.data.qdos.is_empty() {
        assert!(!r
            .files
            .iter()
            .any(|p| p.file_name().unwrap() == "superconducting_dos.svg"));
    }
    let gap = io::parse_imag_aniso(&work.join("MgB2.imag_aniso_020.00")).unwrap();
    assert!(gap.gaps_mev[0]
        .iter()
        .any(|v| v.is_finite() && v.abs() > 0.));
}

#[test]
fn real_mgb2_scan_finds_pade_qdos_and_imaginary_gaps() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/MgB2/ubuntu/run/8.epw-superconductivity");
    if !root.is_dir() {
        return;
    }
    let work = root
        .join("01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV")
        .join("aniso-mu0.1-Tc20-40-degaussq0.5meV");
    let data = DataFiles::scan(&work).unwrap();
    assert_eq!(data.imag_aniso.len(), 3, "{:?}", data.imag_aniso);
    assert_eq!(data.pade_aniso.len(), 3, "{:?}", data.pade_aniso);
    assert_eq!(data.qdos.len(), 3, "{:?}", data.qdos);

    // iso 目录应该有 imag_iso 和 qdos
    let iso_work = root
        .join("01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV")
        .join("iso-mu0.1-Tc20-40-degaussq0.5meV");
    let iso_data = DataFiles::scan(&iso_work).unwrap();
    assert_eq!(iso_data.imag_iso.len(), 3, "{:?}", iso_data.imag_iso);
    assert_eq!(iso_data.qdos.len(), 3, "{:?}", iso_data.qdos);
    assert!(
        iso_data.imag_aniso.is_empty(),
        "iso should not have imag_aniso"
    );
}
#[test]
fn iso_method_uses_imag_iso_for_fig2() {
    let c = Case::new();
    c.write(
        "Test.imag_iso_010.00",
        "# iw(eV) Z delta(eV)\n0.0027072150869 0.9 0.005\n0.0081216452607 0.85 0.0048\n",
    );
    c.write("Test.pade_iso_010.00", "0 1 0 0.005 0\n0.01 1 0 0.0048 0\n");
    c.write("epw.out", "DOS = 2");
    let task = c.task();
    let panels = plot::gap_frequency::plot_gap_imaginary_real(&task)
        .unwrap()
        .unwrap();
    assert_eq!(panels.len(), 2);
    assert_eq!(panels[0].curves[0].points.len(), 2);
    assert!((panels[0].curves[0].points[0].0 - 2.7072150869).abs() < 1e-9);
    assert_eq!(panels[0].curves[0].points[0].1, 5.0); // eV → meV
    assert!(
        !panels[1].curves.is_empty(),
        "iso Padé data must be plotted"
    );
}
#[test]
fn temperature_mismatch_and_native_gap0_fallback() {
    let c = Case::fixture();
    fs::rename(
        c.work.join("Test.pade_aniso_010.00"),
        c.work.join("Test.pade_aniso_020.00"),
    )
    .unwrap();
    c.write(
        "Test.imag_aniso_gap0_010.00",
        "# distribution = rho(delta)\n11 12 10 1 1\n",
    );
    let task = c.task();
    let panels = plot::gap_frequency::plot_gap_imaginary_real(&task)
        .unwrap()
        .unwrap();
    assert!(panels[1].curves.is_empty());
    let panels = plot::gap_temperature::plot_aniso_gap_vs_t(&task)
        .unwrap()
        .unwrap();
    assert_eq!(panels[0].curves[0].points[0], (11., 12.));
}
#[test]
fn sibling_parameter_panels_and_separate_normalization() {
    let c = Case::fixture();
    let sibling = c.work.parent().unwrap().join("aniso-mu0.2-Tc10-30");
    fs::create_dir(&sibling).unwrap();
    fs::copy(
        c.work.join("Test.qdos_010.00"),
        sibling.join("Test.qdos_010.00"),
    )
    .unwrap();
    fs::write(sibling.join("epw1.out"), "DOS = 4").unwrap();
    let panels = plot::dos::plot_superconducting_dos(&c.task())
        .unwrap()
        .unwrap();
    assert_eq!(panels[0].curves[0].points[0], (-10., 1.));
    assert!(
        panels[1].curves.is_empty(),
        "a method plot must not consume sibling data"
    );
}

#[test]
fn mori_gap0_distribution_preserves_native_offsets_and_rejects_bad_rows() {
    let c = Case::fixture();
    let path = c.work.join("Test.imag_aniso_gap0_010.00");
    c.write(
        "Test.imag_aniso_gap0_010.00",
        "# distribution\n10 2 10 0 0\n11 3 10 1 4\n10 4 10 0 0\n",
    );
    assert_eq!(
        io::parse_gap0_distribution(&path).unwrap(),
        [(10., 2.), (11., 3.), (10., 4.)]
    );
    let panels = plot::gap_temperature::plot_aniso_gap_vs_t(&c.task())
        .unwrap()
        .unwrap();
    assert_eq!(
        panels[0].curves[0].points,
        [(10., 2.), (11., 3.), (10., 4.)]
    );
    for bad in [
        "10 2 10 0",
        "10 2 20 0 0",
        "10 2 10 1 4",
        "11 2 10 -1 4",
        "10 NaN 10 0 0",
    ] {
        c.write("Test.imag_aniso_gap0_010.00", bad);
        assert!(io::parse_gap0_distribution(&path).is_err(), "{bad}");
    }
}

#[test]
fn automatic_hook_respects_skip_and_format_is_not_scientific_identity() {
    use epwflow::foundation::{
        config::Config,
        stages::{self, Step},
    };
    let c = Case::fixture();
    let mut cfg: Config = toml::from_str(include_str!("fixtures/input.toml")).unwrap();
    cfg.work_dir = c.root.join("run");
    let key = stages::scientific_key(&cfg, Step::Superconductivity).unwrap();
    cfg.plot.formats = vec!["svg".into()];
    plot::set_skip(true);
    plot::auto_plot(&cfg);
    assert!(!c.root.join("run/results").exists());
    plot::set_skip(false);
    plot::auto_plot(&cfg);
    assert!(c
        .task()
        .result_dir
        .join("superconducting_dos.svg")
        .is_file());
    cfg.plot.formats = vec!["png".into(), "pdf".into()];
    assert_eq!(
        key,
        stages::scientific_key(&cfg, Step::Superconductivity).unwrap()
    );
}
#[test]
fn empty_or_zero_weight_distribution_keeps_other_coupling_panels() {
    for text in ["", "0.1 0\n0.2 0\n"] {
        let c = Case::fixture();
        c.write("Test.lambda_pairs", text);
        let panels = plot::coupling::plot_coupling_triple(&c.task())
            .unwrap()
            .unwrap();
        assert!(panels[0].curves.iter().all(|c| c.points.is_empty()));
        assert!(!panels[1].curves.is_empty());
        assert!(panels[2].secondary.is_some());
    }
}
#[cfg(unix)]
#[test]
fn linked_spectrum_reuses_only_its_companion_distributions() {
    let c = Case::new();
    let fine = c.root.join("run/7.epw-fine/01.grid");
    fs::create_dir_all(&fine).unwrap();
    fs::write(fine.join("Test.a2f"), "1 1 0.1\n2 1 0.2\n").unwrap();
    fs::write(fine.join("Test.lambda_pairs"), "0.1\n").unwrap();
    fs::write(fine.join("Test.lambda_k_pairs"), "1 0.2\n").unwrap();
    std::os::unix::fs::symlink(fine.join("Test.a2f"), c.work.join("source.a2f")).unwrap();
    let data = plot::DataFiles::scan(&c.work).unwrap();
    assert_eq!(
        data.lambda_pairs.unwrap(),
        fs::canonicalize(fine.join("Test.lambda_pairs")).unwrap()
    );
    assert_eq!(
        data.lambda_k_pairs.unwrap(),
        fs::canonicalize(fine.join("Test.lambda_k_pairs")).unwrap()
    );
}

#[test]
fn volumetric_gap_files_are_not_temperature_tables() {
    let c = Case::fixture();
    c.write("Test.imag_aniso_gap0_010.00_1.cube", "volumetric");
    c.write("Test.imag_aniso_gap0_010.00.frmsf", "volumetric");
    let data = plot::DataFiles::scan(&c.work).unwrap();
    assert_eq!(data.aniso_gap0.len(), 3);
    c.write(
        "Test.imag_iso_010.00",
        &format!(
            "w [eV] znorm(w) delta(w) [eV]\n{}",
            fs::read_to_string(c.work.join("Test.imag_iso_010.00")).unwrap()
        ),
    );
    let panels = plot::gap_temperature::plot_iso_gap_vs_t(&c.task())
        .unwrap()
        .unwrap();
    assert_eq!(panels[0].curves[0].points.len(), 3);
}

#[test]
fn plot_only_reports_output_directory_creation_failures() {
    let c = Case::fixture();
    let output_dir = c.task().result_dir;
    fs::create_dir_all(output_dir.parent().unwrap()).unwrap();
    fs::write(&output_dir, "blocked directory").unwrap();
    let config = c.root.join("input.toml");
    fs::write(&config, "work_dir = 'run'\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_epw_superconductivity"))
        .args(["--plot-only", "--config"])
        .arg(config)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("plot_warnings: 1"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Warning: superconductivity plot:"),
        "{stderr}"
    );
    assert!(
        stderr.contains("iso-mu") || stderr.contains("aniso-mu"),
        "{stderr}"
    );
    assert!(!c.root.join("run/epw-state").exists());
}
