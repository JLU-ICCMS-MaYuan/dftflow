# 接口契约：统一绘图格式配置

## 配置

```toml
[plot]
formats = ["svg", "png"]
```

默认 `['svg']`，接受 png/svg/pdf/jpg/jpeg，大小写不敏感，按首次出现顺序去重；jpeg 的输出扩展名为 jpg。空列表报 `[plot].formats cannot be empty`，未知格式报 `Unsupported plot format`。配置通过 `qf_config::load` 加载时校验。

本次仅 QE 声子三类图与 scflow 接入。scflow 额外保留 PDF；不自动删除旧格式文件。

## 公共输出

```rust
pub fn save_plot(
    base_path: &str,
    formats: &[PlotFormat],
    draw_svg: impl FnOnce(&mut String) -> anyhow::Result<()>,
) -> anyhow::Result<Vec<String>>;
```

闭包使用 `SVGBackend::with_string` 绘制一次，保留调用方设置的像素尺寸。基础路径不带扩展名，允许文件名含小数点；输出按追加 `.ext` 命名。返回所有成功路径；重复格式只写一次。空列表在绘制前报错，绘制错误不产生输出。单格式保存失败在 stderr 告警并继续，全部失败返回错误。

- SVG：原矢量文本直接保存。
- PNG：resvg 渲染，复用 `electronic::png`，RGBA、8 位、11812 px/m（约 300 DPI）；不缩放像素尺寸。
- PDF：svg2pdf，使用同一内置字体。
- JPG：resvg 渲染后白底 RGB、JPEG 质量 95。
- 不创建临时 SVG 或 PNG，不要求外部转换命令。

## 声子图

三个函数末尾都接收 `formats: &[PlotFormat]`，返回 `Result<Vec<String>>`：

- `plot_phonon_band(workspace, prefix, formats)` → `phband/phonon_band.ext`，1200×760。
- `plot_phonon_dos(workspace, prefix, elements, counts, sigma, formats)` → `phdos/phonon_dos.ext`，720×760。
- `plot_phonon_combined(workspace, prefix, elements, counts, sigma, formats)` → `phonon_combined.ext`，1600×760。

路径、虚频、投影聚合、高斯平滑与 CSV 数值保持原有规则。自动链、nosplit 及 `--plot` 均由根配置传入格式，续算清单同时包含配置要求的三类图片。

## scflow 图

原 `plot_system`、`plot_article_view`、`plot_summary` 的参数及 `Result<String>` 主 PDF 返回值保留，兼容库调用。

新增 `plot_system_with_formats`、`plot_article_view_with_formats`、`plot_summary_with_formats`：在原参数后追加 `formats: &[PlotFormat]`，返回 `Result<Vec<String>>`。内部先列 PDF，再追加配置格式，并去重。生产 `run` 使用这些新接口，默认 PDF+SVG；即使仅配置 PNG 仍保留 PDF。PDF 是必需产物，失败时返回错误；额外格式失败不覆盖已成功 PDF。

基础文件名沿用原来的参数命名，包括小数点。三类图画布分别为 1200×1600、1200×600、1200×600。

## 历史兼容与范围外

EPW 历史结果清单重绘保持原 PNG/JPG 文件名；EPW 专属图不读取本次格式配置。VASP、替位 DOS、Wannier 后续接入，本次不宣称支持。
