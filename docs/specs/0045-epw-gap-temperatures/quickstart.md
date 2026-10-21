# 0045 快速验证

构建、安装和真实运行的完成情况见 [验收记录](acceptance.md)。完整八步及目录结构见 [MgB₂ 运行说明](../../../examples/MgB2/sugon-amd/run/epwflow/README.md)。

在已有 TOML 中设置以下超导参数，并保留所需的物理参数、程序、预算和上游配置：

```toml
[superconductivity]
task = "gap"
mode = "both"
gap_temperatures_k = [20.0, 30.0, 40.0]
initial_gap_mev = 10.0
mixing_beta = -0.3
```

```bash
epw_phonon -c "epwinput.toml" --check --json
```

检查字段、路径和并行配置。按身份检查完成前七阶段后执行：

```bash
epw_superconductivity -c "epwinput.toml" --stage superconductivity -s
```

每次最多提交一个原生任务。等待终态，再重复同一命令，直至全部六点正常结束。`8.epw-superconductivity/manifest.json` 保存计划和尝试；各点子目录保存原生输入、日志、Δ／Z 和 `attempt.json`。`results/points.csv` 区分执行、收敛和数据状态，有效能隙进入 `gap_summary.csv`、`gap_states.csv` 及 `figures/`，不生成 Tc。

失败后修复原因，并显式续算：

```bash
epw_superconductivity -c "epwinput.toml" --stage superconductivity --resume -s
```

保留成功点及旧失败尝试，只重试未完成点。活动或状态未知的任务不重复提交；更改来源或科学参数会触发身份检查，不能通过修改清单强行复用。

完成后核验结果包：

```bash
epw_superconductivity --results-check --results "8.epw-superconductivity/results" --json
```

当前 PBE 示例已由公开八阶段实际跑通六点，正常结束、收敛和能隙数据均通过；可核对 workflow_complete、gap_run.execution_complete、gap_run.data_complete。原始作业及安装身份见验收记录，不把指定温度能隙解释成 Tc。
