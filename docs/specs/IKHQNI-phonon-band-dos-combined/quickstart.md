# 快速验收

准备符合 [输入契约](contracts/plot.md) 的独立绘图工作区，安装已有 numpy/matplotlib。
JSON 配置可用于未安装 TOML 解析器的 Python 3.10 环境：

```json
{"qe_phonon": {"work_dir": "/absolute/path/to/plot-workspace", "prefix": "Si8P16"}}
```

将示例绝对路径替换为实际工作区，保存到该目录的 `input.json` 后执行：

```bash
PYTHONPATH="/home/mayuan/code/dftflow" python3 -m qeflow.phonon.process_phonon \
  -c "/absolute/path/to/plot-workspace/input.json" --plot
```

预期：退出码 0；旧 PNG/CSV 继续存在；新增 `phonon_band_dos.jpg`。
打开联合图检查：左谱右 DOS、共享 y 轴、保留全部虚频、无重复右侧 y 轴刻度。
Pillow 打开图片应报告 JPEG；输入频率和 DOS 哈希在执行前后不变。

旧 `phonoband/phonodos` 布局只通过输入副本和标签文件适配，不改原始文件，不使用可能被 DOS 覆盖的根目录频率文件。
缺数据应明确失败；不要加 `-r`、`-s` 或 `--collect` 来替代纯绘图。
