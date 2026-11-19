//! 一次绘制 SVG，按配置分别保存；转换失败不丢弃其他成功格式。
use anyhow::{ensure, Context, Result};
use qf_config::PlotFormat;
use std::{fs, path::Path};

/// 闭包通过 `SVGBackend::with_string` 填充 SVG，自行保留原图尺寸与布局。
/// 只写请求的格式，返回成功路径；全部失败时返回错误。
pub fn save_plot(
    base_path: &str,
    formats: &[PlotFormat],
    draw_svg: impl FnOnce(&mut String) -> Result<()>,
) -> Result<Vec<String>> {
    ensure!(!formats.is_empty(), "[plot].formats cannot be empty");
    let mut svg = String::new();
    draw_svg(&mut svg)?;
    let mut saved = Vec::new();
    let mut attempted = Vec::new();
    let mut failures = Vec::new();
    for &format in formats {
        if attempted.contains(&format) {
            continue;
        }
        attempted.push(format);
        // 基础路径可能含小数点；不能用 with_extension 截断文件名。
        let path = format!("{base_path}.{}", format.extension());
        let result = match format {
            PlotFormat::Svg => fs::write(&path, &svg).context("save SVG"),
            PlotFormat::Png => (|| {
                let tree = resvg::usvg::Tree::from_str(&svg, &crate::electronic::font_options())?;
                let size = tree.size().to_int_size();
                crate::electronic::png(&svg, Path::new(&path), size.width(), size.height())
            })(),
            PlotFormat::Pdf => svg_to_pdf(&svg, Path::new(&path)),
            PlotFormat::Jpg => svg_to_jpeg(&svg, Path::new(&path)),
        };
        match result {
            Ok(()) => saved.push(path),
            Err(error) => {
                let message = format!("{}: {error:#}", format.extension());
                eprintln!("Warning: failed to save plot {path}: {error:#}");
                failures.push(message);
            }
        }
    }
    ensure!(
        !saved.is_empty(),
        "Failed to save plot in any format: {}",
        failures.join("; ")
    );
    Ok(saved)
}

/// PNG 和 PDF 使用相同的内置字体，不要求计算节点安装系统字体。
pub fn svg_to_pdf(svg: &str, path: &Path) -> Result<()> {
    let tree = svg2pdf::usvg::Tree::from_str(svg, &crate::electronic::font_options())?;
    let pdf = svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .map_err(|e| anyhow::anyhow!("PDF conversion failed: {e}"))?;
    fs::write(path, pdf)?;
    Ok(())
}

fn svg_to_jpeg(svg: &str, path: &Path) -> Result<()> {
    let tree = resvg::usvg::Tree::from_str(svg, &crate::electronic::font_options())?;
    let size = tree.size().to_int_size();
    let mut pixels = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .context("JPEG pixel allocation failed")?;
    // JPEG 无透明通道，先在白底上合成，避免透明处变黑。
    pixels.fill(resvg::tiny_skia::Color::WHITE);
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    let rgb: Vec<u8> = pixels
        .data()
        .chunks_exact(4)
        .flat_map(|p| p[..3].iter().copied())
        .collect();
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(fs::File::create(path)?, 95);
    encoder.encode(
        &rgb,
        size.width(),
        size.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(())
}
