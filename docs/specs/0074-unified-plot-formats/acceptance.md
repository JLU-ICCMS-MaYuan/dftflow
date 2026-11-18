# 验收记录：统一绘图格式配置

## 验收范围

按 #74 核心范围交付公共格式配置、公共转换、QE 声子三类图、scflow 三类图。VASP、替位 DOS、Wannier、EPW 专属图未接入，scflow 必需 PDF 是明确兼容例外。既有配置层九项新增测试已接续验证，另增加真实加载测试。

## 真实 MgB₂ 只重绘

读取既有 `test_auto/MgB2/local_run/0072-stage-auto-plotting-20261117-03/run/4.qe-phonon-index/` 的原生频率、DOS、路径标签，与仓库 MgB₂ 结构一起复制到独立测试目录。未执行 QE、EPW 或 Slurm 科学任务。

证据目录：`test_auto/MgB2/local_run/0074-plot-formats-20261118/`（忽略目录）。`verify.py` 为可复现验证脚本，`evidence.json` 保存原始数据哈希、退出码、图片清单和 CSV 对照；不检查任何电荷密度或波函数。

| 配置 | 三类图生成格式 | 结果 |
|---|---|---|
| 不配置 `[plot]` | SVG | 三图成功 |
| `formats=['png']` | PNG | 三图成功，无额外格式 |
| `formats=['svg','png','pdf','jpg']` | 四格式 | 十二图成功 |

共 18 张请求图片。SVG 通过 XML 解析；PNG/JPEG 完整解码并核对画布尺寸；PNG 为 11812 px/m（约 300 DPI）；三份 PDF 通过 pdfinfo 解析和 pdftoppm 栅格化。已实看 PNG 联合图及 PDF 栅格图，文字、路径标签、频率轴与 DOS 图例正常。三种配置生成的四份 CSV 各自哈希一致，四份直接来源文件前后哈希不变。

## 自动化覆盖

- `crates/qf-config/src/lib.rs`：默认、大小写、别名、去重、空／非法格式和实际配置加载。
- `crates/qf-plot/tests/formats.rs`：四格式单独输出与组合、只绘制一次、合法图像、PNG DPI／尺寸／实际文字、JPEG 白底透明合成、单项失败继续、全部失败、空列表、重复和绘制失败不遗留文件。
- `qeflow/tests/phonon_plot.rs`：实际 CLI 三图、默认／只 PNG／四格式、非法配置、平滑及原始 CSV 不变、路径错误拒绝。
- `scflow/tests/scflow_parity.rs`：完整生产 CLI 覆盖系统图、文章图、汇总图的默认 PDF+SVG 和配置 PNG/JPG/PDF；黄金 CSV、result.json、结构与数值标准保持原样。
- `crates/qf-scflow/src/plots.rs`：额外格式失败仍保留必需 PDF 和旧返回接口；必需 PDF 失败即使 SVG 成功也报错。

## 回归中修正的既有测试参考

- MgB₂ `rust-reference/scf.in` 与 `nscf.in` 的 Mg 赝势文件名仍是 `mg_pbe_v1.4.uspp`，与仓库实际 `mg_pbe_v1.4.UPF` 不一致。仅同步参考文件名，未改科学参数或赝势数据。
- 分 q 自动链测试仍期待旧提示文字和多余空格；更新为现有生产消息，保留失败状态和错误原因校验。
- 声子重绘测试不再用位图文件大小下限衡量 SVG，改验实际 SVG 内容；保留 CSV 数值和路径异常检查。

## 全量回归与构建

隔离源码排除其他任务未提交改动，使用独立缓存。最终验收和构建基于最新已提交的 #73（9daa9b7f）叠加本任务变更，保留最新分支功能；其他未提交绘图、超导和示例改动未编入本次制品。

| 验证 | 实际结果 | 本地日志 |
|---|---|---|
| qf-config、qf-plot、qf-phonon、qf-scflow、qeflow、scflow 全部测试 | 79 通过，0 失败，0 忽略 | targeted.log |
| #73 双分支回归 | 11 通过 | epw-0073.log |
| #72 自动绘图回归 | 7 通过 | epw-0072.log |
| EPW → qeflow nosplit 共享声子链 | 1 通过 | shared-phonon.log |
| 除 EPW 外的全工作区回归 | 162 通过、1 失败 | workspace-excluding-epw.log |
| 不含 #74 的纯基线复现 | 6 项 EPW 失败均复现；1 项 Wannier 失败也复现 | baseline-failures.json、baseline-*.log |
| 原配额失败用例在 SSD 上使用 release 补测 | 1 通过，4.56 秒 | generation-resource-recheck-release.log |
| release 全工作区构建与固定入口覆盖 | 成功，9 个产品 SHA-256 均与受测制品相符，全部 --help 正常 | release.log |
| 已安装兼容入口重绘 | 三场景再次生成 18 图，CSV 及来源文件不变 | 真实目录 installed/evidence.json |

本地日志目录：`docs/local/0074-unified-plot-formats/`（忽略目录）。99 项直接相关及资源补测全部通过；162 项扩展通过中包含部分上述专项，不简单累加。

### 既有失败与全量边界

完整 `--workspace --no-fail-fast` 在初始隔离源码中执行，已观察到 EPW 失败，随后以 SIGTERM／143 结束，不算全量通过。另有一项脚本生成测试遇到 `/tmp` 配额不足（`Disk quota exceeded`），属于本轮验证环境错误，不能当成代码基线缺陷；改用 SSD 临时目录和 release 程序后，该用例补测通过。release 构建曾中止，改用 SSD 临时文件和日志、降低并发后重建成功，固定命令只在构建成功后替换。

中止前观察到的代码失败，全部匹配既有 #73 基线进程记录中的同名失败，或下列单独基线复现；`comparison.json` 保存逐项对照。尚未执行的 EPW 用例不能宣称通过。

单独复现使用纯 9daa9b7f，源码不含 PlotSection／#74，独立缓存和实际运行日志证明以下失败均早于本次格式改动：

- foundation 的提交未知状态记录查找失败。
- native_interpolation 的旧 coarse_wannier 角色查找失败。
- slurm_log_filenames 的三项旧默认头／路径断言失败。
- superconductivity 的原生单温度尝试断言失败。
- Wannier windows 仍期待旧错误文字 `Disabled`，纯基线也在同一断言失败。

这些失败没有计入通过，也没有修改相关科学实现或放宽标准。完整仓库零失败尚未达到；#74 的配置、输出、CLI、失败隔离、真实图像和共享声子调用已经通过专项验证。

## 成功标准映射

| 标准 | 验证证据 |
|---|---|
| SC-001、002、005–008、015、016 | qeflow/scflow 真实 CLI 默认与配置格式；MgB₂ 三种配置 |
| SC-003、004 | 配置加载单元测试和声子 CLI 空／非法输入 |
| SC-009、010、011、013 | 公共转换测试及真实 XML、PNG 解码／尺寸／DPI，内置字体墨迹与 PNG 实看 |
| SC-012 | 真实三份 PDF 的 pdfinfo、pdftoppm 及联合图实看 |
| SC-014 | 公共输出的失败隔离和 scflow 必需 PDF 例外测试 |

## 文档、提交与远端边界

文档更新根 README、CLAUDE.md、实际 QE 示例及受影响 Overview；原计划不存在的模块根 input.toml 未重建。公共转换单点维护，模块保留既有绘制逻辑，遵循 KISS/DRY。

本次不推送、不切换分支、不操作远端 Issue；新增 Spec 尚未推送，双向远程链接保持未完成，不能据本地实现直接关闭 #74。
