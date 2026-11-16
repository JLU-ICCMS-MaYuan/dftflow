//! 电子图共享配色与由矢量直接栅格化的高清输出。
use anyhow::{Context, Result};
use plotters::prelude::*;
use std::{fs, path::Path};
pub const RENDERER_VERSION: u32 = 58;
/// 比较曲线按下层／上层绘制；整数线宽 4:3 使下层约粗 30%。
#[derive(Clone, Copy)]
pub struct ComparisonStroke {
    pub dash: u32,
    pub gap: u32,
    pub width: u32,
}
/// QE 参考始终为实线；仅共享粗细，不提供可误用于参考线的虚线样式。
pub const REFERENCE_WIDTH: u32 = 4;
pub const INTERPOLATED_STROKE: ComparisonStroke = ComparisonStroke {
    dash: 6,
    gap: 6,
    width: 3,
};
/// 元素投影使用不同短虚线周期，防止多条完全重合时后绘元素完全覆盖先绘元素。
pub fn element_stroke(index: usize) -> ComparisonStroke {
    ComparisonStroke { dash: 6 + 2 * (index % ELEMENTS.len()) as u32, gap: 6, width: 3 }
}
pub const FERMI: RGBColor = RGBColor(220, 0, 0);
pub const INTERPOLATED: RGBColor = RGBColor(0, 70, 210);
pub const QE: RGBColor = RGBColor(40, 40, 40);
pub const WINDOWS: [RGBColor; 4] = [
    RGBColor(0, 115, 60),
    RGBColor(125, 85, 35),
    RGBColor(220, 135, 0),
    RGBColor(145, 65, 155),
];
pub const ELEMENTS: [RGBColor; 6] = [
    RGBColor(0, 115, 60),
    RGBColor(220, 135, 0),
    RGBColor(145, 65, 155),
    RGBColor(125, 85, 35),
    RGBColor(100, 110, 20),
    RGBColor(85, 85, 85),
];

/// SVG stroke 由 resvg 处理；避免位图后端把密集粗折线整体多边形化产生自交碎片。
/// 从数值矢量以目标像素重新渲染，不读取或放大旧 PNG。
pub fn png(svg: &str, file: &Path, width: u32, height: u32) -> Result<()> {
    let options = font_options();
    let tree = resvg::usvg::Tree::from_str(svg, &options)?;
    let mut pixels = resvg::tiny_skia::Pixmap::new(width, height).context("High-resolution image distribution failed")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            width as f32 / tree.size().width(),
            height as f32 / tree.size().height(),
        ),
        &mut pixels.as_mut(),
    );
    let mut encoder = png::Encoder::new(fs::File::create(file)?, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    // 11812 px/m = 300.0248 DPI，向上量化确保不低于 300。
    encoder.set_pixel_dims(Some(png::PixelDimensions {
        xppu: 11812,
        yppu: 11812,
        unit: png::Unit::Meter,
    }));
    encoder.write_header()?.write_image_data(pixels.data())?;
    Ok(())
}

/// PNG／PDF 共用确定字体，避免计算节点默默丢失文字。
pub fn font_options() -> resvg::usvg::Options<'static> {
    let mut options = resvg::usvg::Options::default();
    // 固定内置字体，计算节点无系统字体仍可重绘；不依赖宿主字体 mmap。
    let fonts = options.fontdb_mut();
    fonts.load_font_data(include_bytes!("../assets/fonts/DejaVuSans.ttf").to_vec());
    fonts.set_sans_serif_family("DejaVu Sans");
    fonts.set_serif_family("DejaVu Sans");
    options.font_family = "DejaVu Sans".into();
    options
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_font_renders_text_without_system_fonts() {
        let path =
            std::env::temp_dir().join(format!("qf-electronic-font-{}.png", std::process::id()));
        super::png(r##"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="100"><rect width="300" height="100" fill="white"/><text x="10" y="60" font-family="sans-serif" font-size="40">E_F = 0 eV Γ</text></svg>"##,&path,300,100).unwrap();
        let mut reader = png::Decoder::new(std::fs::File::open(&path).unwrap())
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert!(
            buf[..info.buffer_size()]
                .chunks_exact(4)
                .filter(|p| p[0] < 100 && p[3] > 100)
                .count()
                > 200,
            "Missing fonts cannot be silently output blank text"
        );
        std::fs::remove_file(path).unwrap();
    }
}
