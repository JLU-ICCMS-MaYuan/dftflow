use plotters::prelude::*;
use qf_config::PlotFormat;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Output(PathBuf);
impl Output {
    fn new() -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "plot-formats-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn base(&self) -> String {
        self.0.join("plot_s0.25").to_str().unwrap().into()
    }
    fn file(&self, ext: &str) -> PathBuf {
        PathBuf::from(format!("{}.{}", self.base(), ext))
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn draw(svg: &mut String) -> anyhow::Result<()> {
    let root = SVGBackend::with_string(svg, (320, 160)).into_drawing_area();
    root.fill(&WHITE)?;
    root.draw(&Text::new("MgB2 Γ α²F", (15, 60), ("sans-serif", 30)))?;
    root.draw(&PathElement::new(
        vec![(0, 140), (319, 80)],
        BLUE.stroke_width(2),
    ))?;
    root.present()?;
    Ok(())
}
fn verify(output: &Output, format: PlotFormat) {
    let bytes = fs::read(output.file(format.extension())).unwrap();
    match format {
        PlotFormat::Svg => {
            let tree =
                resvg::usvg::Tree::from_data(&bytes, &qf_plot::electronic::font_options()).unwrap();
            assert_eq!((tree.size().width(), tree.size().height()), (320., 160.));
            assert!(String::from_utf8(bytes).unwrap().contains("MgB2 Γ α²F"));
        }
        PlotFormat::Png => {
            let decoder = png::Decoder::new(bytes.as_slice());
            let mut reader = decoder.read_info().unwrap();
            assert_eq!((reader.info().width, reader.info().height), (320, 160));
            let dpi = reader.info().pixel_dims.unwrap();
            assert_eq!((dpi.xppu, dpi.yppu), (11812, 11812));
            assert!(matches!(dpi.unit, png::Unit::Meter));
            let mut buffer = vec![0; reader.output_buffer_size()];
            reader.next_frame(&mut buffer).unwrap();
            // 文本区域有实际墨迹，内置字体不可静默丢字。
            assert!(buffer
                .chunks_exact(4)
                .take(320 * 75)
                .any(|p| p[0] < 100 && p[1] < 100 && p[2] < 100 && p[3] > 200));
        }
        PlotFormat::Pdf => {
            assert!(bytes.starts_with(b"%PDF-"));
            assert!(bytes.windows(5).any(|b| b == b"%%EOF"));
        }
        PlotFormat::Jpg => {
            let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg)
                .unwrap()
                .to_rgb8();
            assert_eq!(image.dimensions(), (320, 160));
            assert!(image.get_pixel(319, 0).0.iter().all(|v| *v > 245));
        }
    }
}
#[test]
fn each_single_format_writes_only_requested_file() {
    for format in [
        PlotFormat::Svg,
        PlotFormat::Png,
        PlotFormat::Pdf,
        PlotFormat::Jpg,
    ] {
        let output = Output::new();
        let saved = qf_plot::save_plot(&output.base(), &[format], draw).unwrap();
        assert_eq!(saved, [output.file(format.extension()).to_str().unwrap()]);
        assert_eq!(fs::read_dir(&output.0).unwrap().count(), 1);
        verify(&output, format);
    }
}
#[test]
fn multiple_formats_draw_once_and_keep_request_order() {
    for formats in [
        vec![PlotFormat::Svg, PlotFormat::Png],
        vec![PlotFormat::Svg, PlotFormat::Pdf],
        vec![
            PlotFormat::Png,
            PlotFormat::Svg,
            PlotFormat::Pdf,
            PlotFormat::Jpg,
        ],
    ] {
        let output = Output::new();
        let mut draws = 0;
        let saved = qf_plot::save_plot(&output.base(), &formats, |svg| {
            draws += 1;
            draw(svg)
        })
        .unwrap();
        assert_eq!(draws, 1);
        assert_eq!(saved.len(), formats.len());
        for (file, format) in saved.iter().zip(formats) {
            assert!(file.ends_with(format.extension()));
            verify(&output, format);
        }
    }
}
#[test]
fn failed_format_does_not_block_later_outputs() {
    let output = Output::new();
    fs::create_dir(output.file("pdf")).unwrap();
    let saved = qf_plot::save_plot(
        &output.base(),
        &[PlotFormat::Pdf, PlotFormat::Png, PlotFormat::Svg],
        draw,
    )
    .unwrap();
    assert_eq!(saved.len(), 2);
    verify(&output, PlotFormat::Png);
    verify(&output, PlotFormat::Svg);
}
#[test]
fn all_output_failures_are_reported() {
    let output = Output::new();
    fs::create_dir(output.file("svg")).unwrap();
    let error = qf_plot::save_plot(&output.base(), &[PlotFormat::Svg], draw).unwrap_err();
    assert!(error
        .to_string()
        .contains("Failed to save plot in any format"));
}
#[test]
fn empty_formats_are_rejected_before_drawing() {
    let output = Output::new();
    assert!(qf_plot::save_plot(&output.base(), &[], |_| panic!("must not draw")).is_err());
}
#[test]
fn duplicate_formats_are_saved_once() {
    let output = Output::new();
    assert_eq!(
        qf_plot::save_plot(&output.base(), &[PlotFormat::Svg, PlotFormat::Svg], draw)
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn drawing_failure_leaves_no_temporary_output() {
    let output = Output::new();
    assert!(
        qf_plot::save_plot(&output.base(), &[PlotFormat::Svg], |_| anyhow::bail!(
            "drawing failed"
        ))
        .is_err()
    );
    assert_eq!(fs::read_dir(&output.0).unwrap().count(), 0);
}

#[test]
fn jpeg_composites_transparency_on_white() {
    let output = Output::new();
    qf_plot::save_plot(&output.base(), &[PlotFormat::Jpg], |svg| {
        svg.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40"><rect width="20" height="40" fill="red" fill-opacity="0.5"/></svg>"#);
        Ok(())
    }).unwrap();
    let bytes = fs::read(output.file("jpg")).unwrap();
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg)
        .unwrap()
        .to_rgb8();
    assert!(image.get_pixel(39, 20).0.iter().all(|v| *v > 245));
    let red = image.get_pixel(5, 20).0;
    assert!(red[0] > 245 && (115..140).contains(&red[1]) && (115..140).contains(&red[2]));
}
