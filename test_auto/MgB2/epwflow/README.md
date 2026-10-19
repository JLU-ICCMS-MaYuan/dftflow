# MgB₂ 三程序测试入口

公开 EPW 程序只有 `epw_phonon`、`epw_wannier`、`epw_superconductivity`。完整流程图、命令及限制见 [epwflow 手册](../../../epwflow/README.md)，需求与阶段契约见 [0008](../../../docs/specs/0008-epwflow-foundation/three-programs.md)。本目录只放测试脚本；数据统一在用户指定的 `../local_run/`，使用编号加目的命名，历史结果只读。

| 目录 | 本轮验证范围 |
| --- | --- |
| `001-epw-validation` | 历史科学尝试、原始日志与图表；按来源与原清单读取 |
| `002-three-stage-cli` | PBE 输入、qeflow 保留数据输入与三阶段脚本准备 |
| `003-shared-phonon` | 接入 qeflow 后，固定 KPATH／后处理程序身份的实际三脚本生成 |
| `004-three-stage-chain` | 三个私有运行库程序副本、总提交脚本生成与原预算只读核查 |
| `005-three-stage-recovery` | 粗／细检查点、失败归档、成功展宽复用与发布中断进程回归；使用测试替身 |
| `006-auto-quality-reference` | 显式纯能量选窗、独立参考与现有 EPW 输入兼容；使用测试替身 |
| `007-auto-comparison` | 自动比较／三格式图进程回归，以及 PBE 四脚本的生成检查 |
| `008-public-chain` | 三个公开程序连续执行、最终交付恢复与部分格式归属回归；使用测试替身 |

本轮没有提交新科学任务或切换默认安装。PBE 配置不能使用旧 LDA 质量参考；独立参考和第二阶段四类比较图的自动生成已接入；当前选择显式 `energy_only`，使用现有 EPW，不发送其不支持的组合选窗关键字。真实 PBE 全链仍须验收。隔离进程回归和生成脚本不代表真实 Tc 通过。

## 构建与使用

```bash
bash scripts/build-native.sh build -p epwflow --bins --locked --offline
# 以下 004 只复现历史脚本；新功能须按主手册准备新目录和当前程序。
# 复现 004 时使用其已打包的旧程序副本，不覆盖绑定。
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


## 粗／细恢复回归

005 保存测试替身回归日志；失败现场、成功展宽和来源身份由测试实际核验。持久日志在 local_run，临时 fixture 使用系统临时目录并串行运行；成功后按用例清理。它没有新的真实 PBE 参考或 Tc。可复现命令：

```bash
env -u TMPDIR bash scripts/build-native.sh test -p epwflow --lib \
  --test native_interpolation --test real_contracts --locked --offline \
  -- --test-threads=1 > "test_auto/MgB2/local_run/005-three-stage-recovery/recovery-check.log" 2>&1
```

共享声子链和原生尝试均使用固定输入；质量失败不会通过重试重新选窗，未知目录、变化来源或篡改证据拒绝恢复。旧的无原生检查点历史目录不自动收编，历史导入与新科学链仍另行验收。

## 三个公开程序连续执行与部分超导／最终交付恢复

008／009 使用隔离 QE／EPW／Slurm 进程替身，从三个公开程序及实际生成脚本执行到结果包；验证最终图目录冲突保留、终态所属锁回收、科学调用和原清单不变、复制后部分格式重绘与篡改拒绝。全部样本数值是 IO 契约数据，不是 MgB₂ Tc。原预算账本和历史科学来源不写入。

```bash
env -u TMPDIR \
  DFTFLOW_THREE_STAGE_TEST_ROOT="$PWD/test_auto/MgB2/local_run/009-partial-resume/cases-user" \
  bash "scripts/build-native.sh" test -p epwflow --test three_stage_execution \
  --locked --offline -- --test-threads=1 --nocapture
```

测试会生成独立命名案例并保留逐步日志；失败后的原始证据不算通过。已结算分配从活动数组移除，累计消耗必须保留；只读状态检查在显式 refresh 时结算。源代码、当前构建和默认安装分别核验。009 进一步验证 F4 已发布 partial 时的公开续算，保留成功模式和失败计数，拒绝变化来源、配置、模式、篡改产物和缺搜索文件；合法零尝试清单可核验。0008 T047 的定向回归 55 项通过，全部仍为进程替身证据，不是实际科学验收。

## PBE 三程序真实主路径

010-public-pbe 使用示例目录现有 PBE 输入和 energy_only，保留固定网格／五个声子参考／原共享预算。输入、程序和脚本已冻结；实际科学结果以[0015 记录](../../../docs/specs/0015-epwflow-mgb2-real-validation/public-pbe-validation.md)为准，进程替身不替代它。1295834 在科学程序前因 oneAPI 可选变量与 nounset 冲突失败，已经结算；修复的 41 个不同用例最终通过，真实 oneAPI 初始化段加载成功。后续只从同一工作流显式恢复，未知作业不重提，不自动扫描物理参数。

原生 Rust 构建若带 native libc 的 RUNPATH，在旧计算节点须依照仓库既有制品打包规则使用同目录加载器；本案仅变换测试副本的 ELF interpreter，原程序、库、系统和默认安装不变。随后发现启动账表零节点与控制器分配不同步，0008 T049 改为 scontrol 核对启动、sacct 终态结算，44 个不同回归用例最终通过，未决账表保留预留。上述三个失败作业累计只增加 5 节点秒，尚无新 QE 科学输出。

保留 EPW 数据时，q2r 的 XML 力常数输出为 `.fc.xml`，两类 matdyn 与自动比较均显式消费该路径；普通文本模式继续用 `.fc`。该衔接修复定向回归分批 47 个不同用例最终通过、1 项真实环境暂挂。PBE 已有 SCF／PH／q2r 原生完成证据，后处理及其余阶段仍按实际清单核验。
