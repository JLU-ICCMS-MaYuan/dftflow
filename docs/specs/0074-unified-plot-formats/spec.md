# 规格：统一绘图格式配置

**Issue**：[#74](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/74)（`type:feature`）

**实施授权**：用户要求接续已完成的阶段 1，完成阶段 2–6；范围采用该 Issue 的核心需求及下文范围外约定。

## 背景与目标

当前 dftflow 各绘图模块输出格式不统一：
- 声子单独图输出 PNG
- 声子联合图输出 JPG（与单独图不一致）
- 超导图输出 PDF（SVG 转换）
- VASP 处理图输出 PNG

用户无法自定义输出格式，且 JPG 有损压缩不适合科学图表。需要提供全局统一的格式配置，默认使用 SVG 矢量格式，支持用户自定义输出 PNG、SVG、PDF、JPG 的任意组合。

## 用户故事

### US1：全局配置绘图输出格式（P1）

用户在 `input.toml` 中配置：
```toml
[plot]
formats = ["svg", "png"]
```

本次接入的 QE 声子三类图自动输出 SVG 和 PNG；scflow 三类图保留 PDF，并输出 SVG 和 PNG。VASP、替位 DOS、Wannier 与 EPW 专属图暂不接入。

### US2：默认使用 SVG 矢量格式（P1）

用户不配置 `[plot].formats` 时，本次接入的 QE 声子图默认输出 SVG 格式（矢量、无损、可缩放）；scflow 默认保留 PDF 并生成 SVG。

### US3：只输出特定格式（P2）

用户只需要 PNG 用于演示：
```toml
[plot]
formats = ["png"]
```

QE 声子图只生成 PNG；scflow 保留必需的 PDF，另生成 PNG。已有其他格式文件不自动删除。

### 独立验收场景

- US1：给定已有合法声子后处理数据或完整 scflow 测试输入，当根级配置 SVG+PNG 并执行绘图时，三类图都生成配置格式，scflow 额外保留 PDF。
- US2：给定同类数据而省略 `[plot]`，当执行绘图时，QE 声子新生成 SVG，scflow 新生成 PDF+SVG，不再默认生成声子 JPG。
- US3：给定仅请求 PNG 的配置与空输出目录，当执行绘图时，QE 声子只新生成 PNG，scflow 为 PDF+PNG；改用空列表或未知格式时，配置加载失败并报告原因。

## 功能需求

### 配置层 (qf-config)

- **FR-001**: 在 `Config` 结构中添加 `plot: PlotSection` 字段
- **FR-002**: `PlotSection` 包含 `formats: Vec<String>` 字段，默认值为 `["svg"]`
- **FR-003**: 支持的格式：`png`、`svg`、`pdf`、`jpg`（大小写不敏感）
- **FR-004**: `PlotSection::validate_formats()` 方法校验格式合法性并去重
- **FR-005**: 配置加载时自动调用 `validate_formats()` 校验，非法格式报错
- **FR-006**: 空格式列表报错：`[plot].formats cannot be empty`
- **FR-007**: 提供 `PlotFormat` 枚举（`Png`, `Svg`, `Pdf`, `Jpg`）及 `extension()` 方法

### 绘图层 (qf-plot)

- **FR-008**: `qf-plot` 提供统一的多格式输出函数：
  - `save_plot(base_path, formats, draw_fn) -> Result<Vec<String>>`
  - 接受基础路径（不含扩展名）、格式列表、绘图函数
  - 返回所有生成的文件路径列表

- **FR-009**: SVG 作为主输出格式：
  - 绘图函数使用 `SVGBackend` 生成 SVG
  - SVG → PNG: 使用 `resvg` + `tiny_skia` 光栅化（300 DPI）
  - SVG → PDF: 使用 `svg2pdf` 转换
  - SVG → JPG: 从与 PNG 相同的 RGBA 像素白底合成后编码为 JPEG

- **FR-010**: 只请求 PNG 时也从同一 SVG 渲染，保持原画布像素尺寸与布局，不留下 SVG 中间文件；不要求与原位图后端逐像素相同。

- **FR-011**: 复用 `qf-plot::electronic::png()` 的字体和 DPI 设置（300 DPI，内嵌字体）

### 各模块集成

- **FR-012**: `qf-phonon` 声子绘图函数接受 `&[PlotFormat]` 参数（实际子目录为 `phband/` 和 `phdos/`）：
  - `plot_phonon_band(workspace, prefix, formats) -> Result<Vec<String>>`
  - `plot_phonon_dos(workspace, prefix, elements, counts, sigma, formats) -> Result<Vec<String>>`
  - `plot_phonon_combined(workspace, prefix, elements, counts, sigma, formats) -> Result<Vec<String>>`

- **FR-013**: 修复联合图格式：`phonon_combined.jpg` → 遵循配置的格式

- **FR-014**: `qf-scflow` 超导绘图保持当前 PDF 输出逻辑，同时支持配置的额外格式

- **FR-015**: `df-vasp` 处理绘图集成格式配置（后续 PR，本 spec 不要求）

- **FR-016**: `df-substitution` 电子 DOS 绘图集成格式配置（后续 PR，本 spec 不要求）

## 技术约束

- **TC-001**: 不修改 plotters 库的使用方式
- **TC-002**: SVG 转换使用纯 Rust 依赖（`resvg`、`svg2pdf`），不依赖外部工具
- **TC-003**: 保持当前绘图内容、样式、布局不变
- **TC-004**: 多格式输出失败不阻塞主格式成功
- **TC-005**: 配置文件向后兼容：未配置 `[plot]` 时使用默认值

## 成功标准

### 配置验收

- **SC-001**: 用户配置 `formats = ["svg", "png"]` 后，QE 声子三类图输出两种格式；scflow 三类图输出 PDF、SVG、PNG
- **SC-002**: 默认不配置时，QE 声子图输出 SVG；scflow 输出 PDF、SVG
- **SC-003**: 配置非法格式（如 `"gif"`）时加载配置报错
- **SC-004**: 配置空列表时加载配置报错
- **SC-005**: 大小写混用（`"PNG"`、`"Svg"`）正常工作

### 声子绘图验收

- **SC-006**: `plot_phonon_band` 按配置输出指定格式
- **SC-007**: `plot_phonon_dos` 按配置输出指定格式
- **SC-008**: `plot_phonon_combined` 不再硬编码 JPG，仅显式请求时输出 JPG
- **SC-009**: SVG 输出为合法的 SVG 文件，可在浏览器打开
- **SC-010**: PNG 输出为 300 DPI，包含正确的 DPI 元数据

### 格式转换验收

- **SC-011**: SVG → PNG 转换后图像尺寸正确
- **SC-012**: SVG → PDF 转换后文件可用 PDF 阅读器打开
- **SC-013**: 文字渲染正确（使用内嵌字体，计算节点无系统字体也能正确渲染）
- **SC-014**: 多格式输出时，某个格式失败不影响其他格式成功

### 兼容性验收

- **SC-015**: 不配置 `[plot]` 时现有测试通过（输出 SVG 替代原 PNG/JPG）
- **SC-016**: 配置 `formats = ["png"]` 时仅生成 PNG，保留原图画布尺寸、内容和布局；允许 SVG 与位图后端的抗锯齿差异

## 实现计划

见 [plan.md](plan.md)

## 范围外

- 不修改绘图内容、配色、样式、字体大小
- 不修改超导绘图的核心逻辑（保持 SVG → PDF）
- 不在本 spec 中集成 VASP 和电子 DOS 绘图（后续独立 PR）
- 不支持每个模块单独覆盖格式配置（保持全局统一）
- 不支持 GIF、WEBP 等其他格式

## 边界与异常

- 大小写及 `jpeg` 别名按首次出现顺序去重。空列表、未知格式在配置加载时拒绝。
- 各格式逐项保存，某项失败告警并继续，全部失败返回错误；scflow 的必需 PDF 失败仍报告错误。
- 不自动删除历史图；仅保证本次只生成所请求的格式（scflow 的 PDF 除外）。
- 不修改科学输入、CSV 数值、路径标签、投影与高斯平滑；不触发新的科学计算。
- EPW 历史结果包的旧清单重绘维持其原 PNG/JPG 文件名；EPW 专属配置与图包不纳入本次统一格式设置。

## 需求一致性核对

原阶段稿中“所有模块”与 FR-015/016 的后续集成边界、“只 PNG”与 scflow 必需 PDF、FR-010 位图直出与 Issue/Plan 的 SVG 主源互有冲突。本次按 Issue 核心范围和既有 Plan 的 SVG 主源实现统一描述，保留 scflow 的明确兼容例外，不扩展接入模块。既有 scflow 库接口继续返回主 PDF，新增带格式参数的接口返回所有成功路径。

## 原始需求记录

> 我理解你现在没有新的请求。
>
> 我刚才已经完成了 **Issue #74（统一绘图格式配置）** 的**阶段 1：配置层实现**，包括：
>
> 1. ✅ 在 `qf-config/src/lib.rs` 添加了 `PlotSection` 和 `PlotFormat` 枚举
> 2. ✅ 实现了 `validate_formats()` 方法（支持大小写不敏感、去重、错误检测）
> 3. ✅ 在 `Config::load()` 中集成了格式校验
> 4. ✅ 编写了 9 个单元测试
>
> 但由于 Rust 工具链配置问题，我无法运行测试验证。
>
> **接下来的工作**：
>
> - **阶段 2**：在 `qf-plot` 中实现 `save_plot()` 多格式输出函数
> - **阶段 3**：修改 `qf-phonon` 声子绘图函数支持多格式
> - **阶段 4**：`qf-scflow` 超导绘图集成
> - **阶段 5**：文档更新
> - **阶段 6**：回归测试与验收
>
> 你来继续完成。

无图片附件。以上为当前用户提供的交接与实施要求；Issue 原有背景、FR/SC 来自既有稿件。实施细节与核对解释不作为用户原话。

## 文档影响

- 更新根 README 和 CLAUDE.md，说明配置及本次接入范围。
- 更新 `docs/overview/qeflow/`、声子旧入口、`docs/overview/scflow/` 及导航，反映当前默认格式和 scflow PDF 例外。
- 维护本目录 Spec、设计、任务、快速验证和验收；不复制远端 Issue 状态，不自行推送。

## 依赖与确认边界

依赖仓库已有 plotters/resvg/svg2pdf/image 和本机既有构建环境。用户的“你来继续完成”授权接续已建 #74 的阶段 2–6；没有新增业务范围。远端 Spec 链接尚未推送，维持未验证，不以本地任务完成代替远端 Issue 关闭。
