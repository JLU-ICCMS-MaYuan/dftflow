# 0045 配置与公开入口

以下为已有 TOML 的超导段修改示例，其他既有物理和程序设置继续保留：

```toml
[superconductivity]
task = "gap"
mode = "both"
gap_temperatures_k = [20.0, 30.0, 40.0]
initial_gap_mev = 10.0
mixing_beta = -0.3
```

- task 缺省 tc；gap 只算指定温度的非线性能隙。mode 支持 iso、aniso、both。
- 温度列表非空，保持用户顺序，按 0.01 K 精确表示、无重复，目标版本范围 0.01～999.99 K；逐点检查截断能否容纳 Matsubara 频率。
- gap 的单初值缺省 10 meV、必须有限正数；不要求旧双初值数组或 iso／aniso Tc 搜索表。其他 μ*、截断、迭代、混合、谱选择参数继续生效。
- beta 负值选择原生线性混合，正值选择 Broyden，0<abs(beta)<=1；不根据结果自动更改。
- tc 保持原有 gap_temperatures_k、initial_gaps_mev 和所选模式的搜索表语义。

公开命令保持：

```bash
epw_superconductivity -c "epwinput.toml" --stage superconductivity -s
```

每次最多提交一次原生调用；作业结束重复同一命令核验并推进，失败后显式 --resume。both 与三个温度在无失败情况下恰好六个原生调用；每次 nstemp=1，temps(1)=目标 K，tc_linear=false，只启用一个模式。

正常结束但未收敛的点记为执行完成，继续余下点，并明确标能隙不可作为有效数据。非零退出、未正常结束、数据来源损坏或预算不足阻止整体完成，保留已有成功点；身份冲突不能通过重试绕过。

结果包含逐点状态汇总、epw.in／out／err、退出与结束证据、可用的完整 Δ／Z。编号目录为 `8.epw-superconductivity/`，版本 3 的 manifest.json 保存固定点计划；尝试目录名含模式、两位小数温度与全局尝试编号。`results/points.csv` 区分执行与数据状态，有效值进入 gap_summary.csv／gap_states.csv。Tc 未请求时不产生 tc.csv 或 Tc 区间图。归集／绘图／检查沿用现有公开选项，不新增公共程序，详见 [数据模型](../data-model.md)。
