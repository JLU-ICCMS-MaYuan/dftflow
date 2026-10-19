# 当前入口：实际拟合外窗与编号运行目录

运行数据现位于 `../local_run/001-epw-validation/`，各次实验目录均为编号加目的简述；完整目录索引见数据根的 README.md。科学程序要求的 tmp、save 等内部目录保留原名。PDF 位于 `091-reference-literature/`，历史输入位于 `090-historical-input-scripts/`，迁移映射和逐文件哈希位于 `092-migration-evidence/`。本脚本目录没有运行数据或兼容数据链接。

当前图位于 `026-disentanglement-window-plots/results/figures/`：

- [电子能带](../local_run/001-epw-validation/026-disentanglement-window-plots/results/figures/electronic_bands_comparison.png)
- [电子 DOS](../local_run/001-epw-validation/026-disentanglement-window-plots/results/figures/electronic_dos_comparison.png)
- [四图总览](../local_run/001-epw-validation/026-disentanglement-window-plots/results/figures/comparison_overview.png)

同目录有 SVG、PDF。实际 Wannier Outer 外窗为 −4.90710～32.23753 eV，共同 E_F=7.497269413 eV，对应 E−E_F=−12.404369413～24.740260587 eV。输入没有显式 dis_win_min/max，采用程序日志报告的实际默认边界。宽窗内 QE 全部能带与五维 Wannier 子空间覆盖不同，图仅作诊断；投影继续暂缓，原声子 RMS 失败仍阻断下游。

```bash
bash test_auto/MgB2/epwflow/replot_window.sh
```

重绘入口只调用受测 Rust export-comparison／collect／plot／results check。新路径来源清单位于 026 根目录，旧清单原样归档；只读质量检查使用：

```bash
./target/linux-x86_64/x86_64-unknown-linux-gnu/debug/epwflow interpolation check \
  -w test_auto/MgB2/local_run/001-epw-validation/026-disentanglement-window-plots --json
```

预期 quality_failed、退出 1；这是已有质量失败，不是新绘图失败。5977 个原始文件（含 PDF 和被引用历史脚本）迁移前后 SHA-256 全部相同。新配置和工作目录使用上述编号数据根，不使用旧路径。

---

以下为此前测试记录，其中旧位置已由上述目录索引和迁移映射替代，历史路径不再作为操作入口。

# MgB₂ EPW 测试入口

脚本放在本目录，计算数据统一放在 `../local_run/epwflow/`，与已有 VASP 数据分开。

| 位置 | 内容 |
| --- | --- |
| `submit.sh` | 调用公开 `epwflow submit`，在主节点使用 |
| `replot.sh` | 从已有比较包收集并重绘，不运行 QE／EPW／Slurm |
| `verify_native_epw.py` | 旧 PBE 小网格原生冒烟检查器，不能用于新的 LDA 参数 |
| `historical-inputs/` | 本地忽略的历史脚本，仅保留已被来源清单引用的原始输入 |
| `../local_run/epwflow/0015-native/` | 输入、保存树、输出、图表、来源和运行证据，全部本地忽略 |

已删除 35 个临时、重复准备／绘图／验证脚本；Rust 回归测试及其他模块测试保留。11 个被质量来源清单引用的脚本实体移到 historical-inputs，原位置保留链接用于历史指纹核查。旧 `epwflow/local_run` 只作兼容链接，数据实体已迁出；不修改原始输入、日志或旧来源清单。迁移核对 5930 个非脚本文件的 SHA-256 全部一致，记录在新数据根目录 `directory-migration.json`。用户的 Mori PDF 保留原位。

## 直接命令

先使用平台构建脚本构建，明确指定受测入口；默认安装没有切换：

```bash
bash scripts/build-native.sh build -p epwflow --locked --offline
EPWFLOW_BIN="$PWD/target/linux-x86_64/x86_64-unknown-linux-gnu/debug/epwflow"
CONFIG="$PWD/test_auto/MgB2/local_run/epwflow/你的配置.toml"
WORK="$PWD/test_auto/MgB2/local_run/epwflow/你的运行目录"

"$EPWFLOW_BIN" check -c "$CONFIG" --json
"$EPWFLOW_BIN" prepare -c "$CONFIG"
"$EPWFLOW_BIN" submit -c "$CONFIG" --until qe-data
"$EPWFLOW_BIN" status -w "$WORK" --refresh --json
"$EPWFLOW_BIN" resume -w "$WORK" -s --until interpolation
```

最后两行按实际状态使用。`--until` 可选 `qe-data|interpolation|superconductivity|all`，默认 all；指定边界完成不表示完整流程成功。`run` 和 `resume -r` 需要已登记的 Slurm 分配，不能在登录节点直接执行科学计算。所有新配置的 work_dir 应放在上述数据根目录，仍使用同一共享预算。

等价脚本入口清楚调用上述命令：

```bash
bash test_auto/MgB2/epwflow/submit.sh "$CONFIG" --until qe-data
bash test_auto/MgB2/epwflow/replot.sh
```

配置模板见 [基础契约](../../../docs/specs/0008-epwflow-foundation/contracts/interface.md)，当前能力及限制见 [Overview](../../../docs/overview/epwflow/README.md)。自动入口的跨进程替身回归已覆盖停止边界和失败阻断；底层首次原生数据适配、矩阵元和两种超导执行仍有未完成项，不能据入口接受命令承诺全链成功。

## 当前真实结果

- 旧 PBE 三种 PH 来源完成了原生 EPW 5.9 的 Wannier、粗／细网格 EPC、α²F／λ 冒烟；不是公开自动链验收。原生直接求和 λ 分别为 nosplit 0.6509531、coords 0.6509533、index 0.6508502。严格同轴三来源比较尚未通过。
- Mori 讲义 LDA 输入完成 SCF、NSCF、6³ 粗 q PH／Wannier／粗网格；保留全部失败尝试，不混用旧 PBE 保存树。
- 当前声子质量门为最大 50 cm⁻¹、RMS 2 cm⁻¹。五点实测最大／RMS 为 49.448071／8.075133 cm⁻¹，RMS 仍失败；没有越过该门运行新细网格或 Eliashberg／Tc。元素／轨道投影暂缓。
- 主图紫点：Γ–M 段内 q=(1/12,0,0)，第八频率排序模，差 9.042613 cm⁻¹。路径外采样最大 q=(1/12,1/12,1/12)，第七排序模，差 49.448071 cm⁻¹，另列诊断区。

查看 [声子谱](../local_run/epwflow/0015-native/overlay-q6-purple50-01/results/figures/phonon_dispersion_comparison.png) 和 [四图总览](../local_run/epwflow/0015-native/overlay-q6-purple50-01/results/figures/comparison_overview.png)；同目录提供 SVG、PDF。这些链接只在保留本地运行数据的机器可用。

只读复查当前质量：

```bash
"$EPWFLOW_BIN" interpolation check -w test_auto/MgB2/local_run/epwflow/0015-native/quality-policy15-01 --json
```

预期返回 quality_failed、退出 1；这是科学质量未通过，不是命令崩溃。完整历史、参数和数值证据见 [0015 验收](../../../docs/specs/0015-epwflow-mgb2-real-validation/acceptance.md)。原预算仍为累计 8 节点小时、100 GiB；迁移和重绘不重置预算，不新增科学任务。
