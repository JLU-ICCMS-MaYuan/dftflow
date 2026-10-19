# 实际拟合外窗重绘与运行目录整理验收

本轮落实用户要求：展示 `dis_win_min`～`dis_win_max` 内的电子 DOS 和能带；将 `test_auto/MgB2/epwflow/local_run` 的运行数据迁入 `test_auto/MgB2/local_run`，运行目录采用编号加目的简述，`epwflow/` 仅保留测试脚本及入口说明。沿用 0012、0014、0015，不新增 Issue。

## 实际窗口与科学含义

本次 LDA 输入没有显式设置 `dis_win_min/max`，因此读取带 SHA-256 的原生 Wannier `.wout` 中实际采用的唯一 `Outer` 外窗，不能将此前诊断范围 ±0.5 eV 当作实际拟合窗。

| 参数 | 实测值 |
| --- | --- |
| 绝对能量外窗 | −4.90710～32.23753 eV |
| 共同 E_F | 7.497269413025949 eV |
| 图中 E−E_F 范围 | −12.404369413025949～24.740260586974053 eV |
| DOS 显示轴 | 在上述完整范围重新累积 501 点，沿用两侧共同高斯核与物理权重 |

电子能带和 DOS 均显示完整外窗，标题同时标明绝对窗口与相对 E_F 的坐标。QE 实线先绘，EPW 虚线后绘；四张独立图和 2×2 总览保留三格式。五维 Wannier 子空间与 QE 全部能带覆盖不同，当前宽窗比较记录 2527 个未匹配参考态，不能据此声称整个外窗拟合通过。电子显示窗口扩大不改变独立质量检查的原窗口、容差或采样。

## 目录与来源保护

数据根为 `test_auto/MgB2/local_run/001-epw-validation/`，该目录 README 列出全部编号和用途。示例：`019-lda-q6-phonon-wannier/` 为实际 6³ 粗网格来源，`023-lda-five-q-quality-check/` 为五点检查，`026-disentanglement-window-plots/` 为本轮重绘。科学程序要求的 `tmp`、`save` 等内部名称保留。

历史输入、参考 PDF、迁移证据分别归入 `090-historical-input-scripts/`、`091-reference-literature/`、`092-migration-evidence/`，缓存归入 093／094。脚本目录内旧数据链接已移除。迁移前后核对 **5977 个实体文件、7626611464 字节，全部 SHA-256 一致**；11 个历史脚本链接更新为指向新实体的相对链接。

迁移未改写原科学输入、日志和历史 JSON 内的旧绝对路径。旧清单作为历史记录保留；本轮生成新路径的 `026-disentanglement-window-plots/comparison-sources.json`、`quality-reference.json` 和 `source-relocation.json`，核对原指纹并保留旧清单来源。实际 `.wout` 所属 `epwdata.fmt` 与比较所用版本哈希一致。

逐文件迁移记录为 `092-migration-evidence/migration-before.json`，汇总为 `directory-migration.json`，路径映射为 `relocation-map.json`。这些机器相关原始证据及真实计算文件均保持本地忽略，不纳入 Git。

## 验证结果

| 检查 | 结果 |
| --- | --- |
| 当前工作区 epwflow 完整回归 | `bash scripts/build-native.sh test -p epwflow --locked --offline`：87 通过，0 失败，0 暂挂；其中其他已有改动不归为本轮实现 |
| 最后窗口定向复测 | `comparison_plots` 6 项通过：实际窗口、Fortran 指数、非法／重复窗口、非零 E_F 换算、覆盖不足、来源篡改和旧包兼容 |
| 静态检查 | 本轮四个 Rust 文件 rustfmt 检查、脚本 `bash -n` 与 `git diff --check` 通过 |
| 真实重绘 | 新入口 `replot_window.sh` 成功；生成 15 个 PNG／SVG／PDF，结果检查 `figures_verified=true` |
| 三格式实看 | 查看能带和总览 PNG，另将实际 PDF、SVG 渲染后查看，窗口、图例与实虚线可辨 |
| 可搬移性 | 复制结果包后在空 PATH 下用受测二进制重绘 SVG 并检查成功，包内比较数据哈希不变 |
| 原始数据保护 | 重新累积 DOS 和绘图未改变原始比较来源及迁移历史文件；QE／EPW／sbatch 调用为零 |

本轮图像位于 `026-disentanglement-window-plots/results/figures/`；独立重绘包与 `window-verification.json` 位于同目录运行实体的 `verification/`。可用命令及图片链接见[测试入口](../../../test_auto/MgB2/epwflow/README.md)。

## 验收边界

本轮验收实际窗口显示与目录整理，未新增科学作业、改变物理参数或覆盖默认安装。五点声子最大／RMS 仍为 49.448071／8.075133 cm⁻¹；在 50／2 门下 RMS 失败，质量检查仍返回 `quality_failed`、退出 1。图包保留 `quality_status=failed`、`workflow_complete=false`、`comparison_complete=false`。

元素／轨道 PDOS 和元素声子 DOS 继续按用户决定暂缓，不以总量曲线替代严格投影。完整三来源、原生矩阵元／Eliashberg／Tc 等未完成项保持原状态，共享预算不重置。
