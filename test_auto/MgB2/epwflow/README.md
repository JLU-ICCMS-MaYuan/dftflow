# MgB₂ 三程序测试入口

公开 EPW 程序只有 `epw_phonon`、`epw_wannier`、`epw_superconductivity`。完整流程图、命令及限制见 [epwflow 手册](../../../epwflow/README.md)，需求与阶段契约见 [0008](../../../docs/specs/0008-epwflow-foundation/three-programs.md)。本目录只放测试脚本；数据统一在用户指定的 `../local_run/`，使用编号加目的命名，历史结果只读。

| 目录 | 本轮验证范围 |
| --- | --- |
| `001-epw-validation` | 历史科学尝试、原始日志与图表；按来源与原清单读取 |
| `002-three-stage-cli` | PBE 输入、qeflow 保留数据输入与三阶段脚本准备 |
| `003-shared-phonon` | 接入 qeflow 后，固定 KPATH／后处理程序身份的实际三脚本生成 |
| `004-three-stage-chain` | 三个私有运行库程序副本、总提交脚本生成与原预算只读核查 |

本轮没有提交新科学任务或切换默认安装。PBE 配置不能使用旧 LDA 质量参考；PBE 独立参考自动生成和第二阶段四类比较图自动生成仍待收敛，现有 EPW 的组合选窗关键字兼容仍阻断第二阶段。隔离进程回归和生成脚本不代表真实 Tc 通过。

## 构建与使用

```bash
bash scripts/build-native.sh build -p epwflow --bins --locked --offline
# 复用本轮固定目录时使用其已打包的程序副本。
EPW_BIN_DIR="$PWD/test_auto/MgB2/local_run/004-three-stage-chain/bin"
CONFIG="$PWD/test_auto/MgB2/local_run/004-three-stage-chain/inputepw.toml"
WORK="$PWD/test_auto/MgB2/local_run/004-three-stage-chain/epwflow"

"$EPW_BIN_DIR/epw_phonon" --check -c "$CONFIG" --json
# 新运行目录使用同一套程序生成；已固定目录须保持原程序身份。
"$EPW_BIN_DIR/epw_phonon" --all -c "$CONFIG"
# 真正需要提交时，才执行生成的总脚本。
bash "$WORK/.epwflow/generated/submit-all.sh"

# 也可逐阶段提交；后两阶段要求前置阶段已有有效完成证据。
"$EPW_BIN_DIR/epw_phonon" -c "$CONFIG" -s
"$EPW_BIN_DIR/epw_wannier" -c "$CONFIG" -s
"$EPW_BIN_DIR/epw_superconductivity" -c "$CONFIG" -s
"$EPW_BIN_DIR/epw_phonon" --status -w "$WORK" --refresh --json
```

以上总提交与逐阶段提交是两种入口用法，不要对同一尝试重复提交。默认安装未更新时，应明确使用受测程序路径。004 中已生成的脚本绑定该目录的 `bin/` 副本；新的构建请创建新的工作目录，不能覆盖旧绑定。

`submit.sh` 是调用上述三个程序的测试辅助脚本，例如 `EPW_BIN_DIR=... bash test_auto/MgB2/epwflow/submit.sh phonon "$CONFIG"`。失败后使用对应阶段的 `--resume -w "$WORK" -s`，活动或未知状态不自动重提。

## 只读检查和重绘

`CALC` 取工作流记录的实际 calculation_dir。独立重绘只消费已有原生数据，不调用 QE／EPW／Slurm。

```bash
"$EPW_BIN_DIR/epw_wannier" --check -w "$CALC" --json
EPW_WANNIER_BIN="$EPW_BIN_DIR/epw_wannier" \
  bash test_auto/MgB2/epwflow/replot_window.sh "$CALC"
```

`replot_window.sh` 在缺比较清单时从 calculation_dir/comparison-sources.json 导出；已有清单时只收集和重绘。`replot.sh` 只消费已有比较清单。两者使用 `epw_wannier` 的选项，不再调用旧 epwflow 多层子命令。图中 QE 实线先绘、EPW 虚线后绘，四张独立图加总览支持 PNG／SVG／PDF；实际外窗和冻结窗边界保留。元素／轨道投影继续暂缓，缺项明确标注。

当前声子门为最大／RMS 50／20 cm⁻¹；旧报告保留当时阈值，不能把旧的 2 cm⁻¹ 门写成当前事实。真实数值、原生求解问题及完整验收缺口见 [0015 验收](../../../docs/specs/0015-epwflow-mgb2-real-validation/acceptance.md) 与 [原生源码诊断](../../../docs/specs/0015-epwflow-mgb2-real-validation/native-solver-source-audit.md)。原共享预算仍为 8 节点小时／100 GiB，不按目录或阶段重置。
