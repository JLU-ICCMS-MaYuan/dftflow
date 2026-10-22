# 快速验证：QE 分阶段即时出图

## 前置条件

使用仓库 Rust／本机原生依赖构建环境，不切换默认安装。测试使用隔离工作区和科学程序替身，不提交真实 Slurm 作业。

## 自动回归

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --locked -p epwflow --test three_stage_execution qe_stage_plots -- --nocapture
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --locked -p epwflow --test comparison_plots
```

预期：无 PH／Wannier 产物时电子图已生成；声子图包含真实元素汇总；缺原子投影被拒绝；删除图及失败恢复不增加成功科学程序调用。声子比较 SVG 只显示 total 面板，无元素投影或缺失占位；电子叠图回归保持通过。

## 实际用户路径

```bash
epw_phonon -c input.toml --stage electronic -s
# 原生作业结束后重复同一命令，直至本阶段核验完成。
epw_phonon -c input.toml --stage phonon -s
# 原生作业结束后继续重复，最后查看相应阶段 plots/。
epw_wannier -c input.toml --stage wannier -s
```

这些命令只是使用说明，本轮不对用户当前工作区执行真实计算。已有科学阶段可复用时，执行原有公开推进命令补图；恢复已失败操作沿用 --resume。实际目录和图名见[接口契约](contracts/plots.md)。

## 真实数据出图检查

从已有只读验证来源复制必要绘图输入到独立临时目录，调用生产绘图函数。出图前后对直接原始输入做 SHA256，检查 CSV／元素汇总和图像；不调用 QE／EPW／sbatch，不覆盖历史图。证据与局限记录在 acceptance.md。


显式检查已有真实数据时，使用新的空输出目录（须不存在）：

```bash
DFTFLOW_QE_PLOT_SOURCE="/已有编号工作区" \
DFTFLOW_QE_PLOT_OUTPUT="/新的绘图证据目录" \
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --locked -p epwflow --test qe_stage_plots_real -- --ignored --nocapture
```

不要在其他进程回归仍使用同一产物目录时重新构建程序；测试核验正在执行的程序来源，构建替换可能导致 `(deleted)` 路径失败。
