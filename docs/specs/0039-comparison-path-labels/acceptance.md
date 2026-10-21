# 验收记录

公开绘图回归 9/9 通过（串行与 4 线程各一次）；新增用例核对 Γ、M | L、原竖线坐标及科学数据哈希。首次新用例因 SVG 换行未 trim 而断言失败，修正测试；同次并行进程出现一次 SIGBUS，随后完整串行与并行复跑均通过，未确认该信号根因，保留原日志，不把首次失败计为通过。

规范构建、安装 3 个公开程序并以实际配置检查；另外 6 个模块安装制品保持。已用安装的 epw_wannier --plot --results 离线重绘本例 15 张 PNG／SVG／PDF，未提交科学作业。实际 SVG 的 Γ、A | L、M | H 正确，PNG 总览已实看，PDF 转图与源／表保护核验见后续最终记录。质量仍 failed；新 0040 明确另行改变数值阻断策略，不由本绘图缺陷放宽质量。

证据：docs/local/0027-mgb2-epwflow-example-debug/tests-0039-final.log、tests-0039-parallel-repeat.log、build-0039.log、install-0039-verification.log、build-install-0039-manifest.json、plot-0039.log。旧 15 图已保存 before-0039-figures/。

最终核验：280 份比较科学源／表文件哈希不变；525 份上游、494 份独立参考、78 份拟合、4 份后处理保持；103 份源文件与安装记录一致。实际 PNG 总览及 PDF 转图已实看，三类 SVG 的 Γ、A | L、M | H 已解析核对，15 图清单有效。证据为 verification-0039.json、protection-0039.log、overview-0039-pdf.png。
