# 配置与命令契约

> 2026-11-17 最新约定：预算账本功能已按用户要求彻底撤销。本文件涉及账本、campaign 身份、预算预留／结算及全局两个作业限制的旧条款均已失效；当前计算只依赖工作流记录、Slurm 作业身份及科学核验。历史消耗与验收记录不改写。详见 [0058 最新需求](../../0058-epw-remove-resource-limits/spec.md)。

```toml
[superconductivity]
task = "gap"
mode = "both"
mu_star = 0.13
gap_temperatures_k = [20.0, 30.0, 40.0]
```

mu_star 可省略，默认 0.13；显式值优先。其余原配置仍必需。温度正数、0.01 K 精度、互不重复且最多 50 点，按配置顺序传递。

```bash
epw_superconductivity -c "epwinput.toml" --stage superconductivity -s
epw_superconductivity -c "epwinput.toml" --status --json
epw_superconductivity -c "epwinput.toml" --stage superconductivity --resume -s
```

前置可复用时一次提交两个独立作业，输出方法、目录和作业号；单方法只提交一个。脚本直接运行科学程序，使用 --chdir，无额外 cd 或 shell 严格选项。

再次调用收集已结束的批次，不需逐温度推进；只读 --status 显示两方法调度状态。未完成批次恢复需 --resume；未知回执不重提，第二次提交失败保留第一次作业。每方法使用 resources 配置并单独预留共享预算，不扩大上限。
