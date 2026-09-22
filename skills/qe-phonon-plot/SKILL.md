---
name: qe-phonon-plot
description: 使用 dftflow 现有 qe_process_phonon 从 QE 归档绘制声子谱、元素态密度和共享频率轴联合 JPG；适用于已完成计算结果的绘图及旧目录适配，不启动新计算。
---

# QE 声子归档绘图

## 入口和输出

优先复用 `qe_process_phonon -c <配置绝对路径> --plot`，不要另建绘图 CLI。
未安装命令入口时可运行 `python -m qeflow.phonon.process_phonon`，确保环境能导入本仓库。
输出原有两张 PNG、四个 CSV，以及工作区根目录 `phonon_band_dos.jpg`（左谱右 DOS，共享频率轴）。

## 数据核验

1. 确认色散频率是路径采样而非 DOS 网格：核对 `nbnd/nks`、matdyn 路径点权重和原始 q 坐标。
2. 旧计算常把 DOS 输出写回根目录同名 `.freq`；优先检查 `phonoband/` 保存的路径数据，不仅按文件名猜测。
3. DOS 原子列按实际 QE 原子顺序对应结构，不能根据化学式中的元素排序猜测。无分波输入时不要把占位零值描述为有效元素投影。
4. 读取来源和绘图输入的路径、哈希、模式数、频率范围应记录在调用方结果目录。

## 旧目录适配

在新的独立工作区准备副本，不重命名或覆盖用户原始归档：

- `POSCAR`：与 QE 输入一致的计算结构。
- `phband/<prefix>.phband.freq`：正确路径频率。
- `phband/<prefix>_band.labelinfo.dat`：标签、1-based 索引、累计距离、三个路径坐标。
- `phdos/<prefix>.phdos.dat`：频率、TDOS、可选原子分量。

标签取自实际 matdyn 输入，并核对所有路径顶点和总点数；不要自动换成另一条标准路径。
可复用 `qeflow.input_deck.kpath.write_kpath_labels`；其距离单位为 Å⁻¹。若直接使用 QE `.freq.gp` 距离，应注明其 2π/alat 单位。
JSON 配置的 `qe_phonon.work_dir` 设为该工作区绝对路径，`prefix` 与文件名一致；JSON 不依赖 TOML 解析包。

## 验收与边界

- 保留所有负频，不裁剪或取绝对值，不从图片自动宣称稳定性或 Tc。
- 核对 CSV 与原频率及 DOS；联合 JPG 应可解码且两面板轴范围一致。
- 只需绘图时不使用 `-r`、`-s` 或 `--collect`。
- 历史完成状态不保证 JPG 存在；直接检查新产物。
- 重绘会覆盖生成文件；优先使用新输出工作区。缺失数据应报告，不画零曲线代替。

详细输入/输出：[qeflow README](../../qeflow/README.md#8-qe_process_phonon)。
