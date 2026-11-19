# 研究与设计核对

## 实施基线

用户授权接续 #74 阶段 2–6。远端 #74 为开放的 `type:feature`；既有阶段 1 未提交代码已核实。遵循 Issue 的 SVG 主源、既有 Plan 的统一转换、scflow 必需 PDF 例外及 VASP/替位 DOS 后续接入边界。

## 决策与理由

1. `save_plot(base_path, formats, draw_svg)` 的绘图闭包接收内存 SVG 字符串，调用方使用 `SVGBackend::with_string` 自行确定尺寸。现有稿三参数函数未提供尺寸，固定尺寸会破坏声子三图和 scflow 三图原有布局；内存字符串避免临时文件覆盖、清理遗漏及只请求 PNG 时遗留 SVG。
2. PNG 使用已有 `electronic::png` 和内置 DejaVu Sans、11812 px/m 元数据，不放大原 PNG。JPEG 同样从 SVG 渲染，先白底合成再以质量 95 编码，无需落盘中间 PNG。PDF 复用相同内置字体，不依赖计算节点系统字体。
3. 不新增依赖版本：`svg2pdf 0.12`、`image 0.25`、`resvg 0.43` 已在锁文件中。仅补充本次 crate 的直接依赖边，并移除 scflow 对 svg2pdf 的直接依赖。
4. scflow 新增 `*_with_formats` 接口，返回成功文件列表；保留原 `Result<String>` 接口作为仅 PDF 的兼容包装。`from_config` 将全局格式传到完整生产链，包括汇总图。
5. 续算产物清单按配置生成，覆盖三类图，后处理验证 CSV，图片保存失败由公共输出函数报告。续算完整性检查要求全部请求格式存在，缺失时允许恢复后处理；额外格式失败不使已成功的绘图直接报错。
6. EPW 历史清单重绘必须继续生成记录的 PNG/JPG 名称；此分支显式使用旧格式，不把 EPW 专属图伪称为已统一配置。
7. 既有计划引用的 `qeflow/input.toml`、`vaspflow/input.toml`、`wannier90flow/input.toml` 已不存在。示例采用现有 QE 测试模板和根 README、Overview、快速验证中的 TOML；不重建过时入口。

## 取舍

原阶段稿 FR-010 的位图直出与 Issue/Plan 的 SVG 主源相冲突。按 SVG 主源统一实施，保留像素尺寸与内容布局，不承诺旧后端逐像素一致。JPEG 直接从相同 RGBA 像素编码，省去无价值的 PNG 编解码往返。未扩展 VASP、替位 DOS、Wannier 或 EPW 专属绘图。

## 追踪

| 需求／标准 | 事实来源与职责 | 任务 | 验证 |
|---|---|---|---|
| FR-001–007；SC-003–005 | qf-config 的根 plot 配置及加载校验 | T001 | 配置单元测试及实际 CLI 非法输入 |
| FR-008–011；SC-009–014 | qf-plot 的内存 SVG、转换与错误隔离 | T002–T003 | 四格式解码、尺寸、DPI、文字和失败隔离 |
| FR-012–013；SC-001–002、006–008、015–016 | qf-phonon 三类图，qeflow 入口与续算清单 | T004–T006 | CLI 三图、实际 MgB₂ 数据副本重绘 |
| FR-014；SC-001–002、014–015 | qf-scflow 配置、三类图及兼容包装 | T007–T008 | 完整 scflow 黄金数值对照与额外格式 |
| FR-015–016 | 本次范围外 | T009 | 文档明确边界 |
