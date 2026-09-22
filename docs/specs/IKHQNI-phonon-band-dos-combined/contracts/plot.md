# 绘图输出契约

入口仍为 `qe_process_phonon -c <配置绝对路径> --plot`，或对应 Python 模块。

输入保持：

- `POSCAR`、`qe_phonon.prefix/work_dir` 配置。
- `phband/<prefix>.phband.freq`。
- `phband/<prefix>_band.labelinfo.dat`。
- `phdos/<prefix>.phdos.dat`。

新增输出：工作区根目录 `phonon_band_dos.jpg`，JPEG，300 dpi，白底。
左面板高对称路径 / 频率；右面板 DOS / 同一频率；仅左面板显示频率单位和刻度。
已有六个 PNG/CSV 输出与 `workflow_status.json` 的判定规则保持不变。

CLI 无新参数、无新依赖、无外部服务。缺文件或保存失败继续抛出异常并使命令失败；不自动调用 QE 补计算。
与已有绘图相同，重复执行可覆盖工作区中的生成文件。调用者应使用新的独立工作区避免覆盖人工编辑结果。
