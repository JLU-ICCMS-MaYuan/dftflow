# 配置、命令与目录契约

```toml
[superconductivity]
task = "gap"
mode = "both"
mu_star = 0.13
gap_temperatures_k = [20.0, 30.0, 40.0]
```

mu_star 可省略，默认 0.13；显式值优先。其余原配置仍必需。温度正数、0.01 K 精度、互不重复且最多 50 点，按配置顺序传递。目录首尾采用配置列表首末项，无单位后缀或多余零。

```bash
epw_superconductivity -c "epwinput.toml" --stage superconductivity -s
epw_superconductivity -c "epwinput.toml" --status --json
epw_superconductivity -c "epwinput.toml" --stage superconductivity --resume -s
```

前置可复用时一次提交两个独立作业，输出方法、目录和作业号；单方法只提交一个。目录为 8.epw-superconductivity/iso-mu0.13-20-40 和 aniso-mu0.13-20-40，各有 epw.in、epw.sh、epw.out／err、slurm-epw.log 及带温度的原生输出。脚本直接运行科学程序，使用 --chdir，无额外 cd 或 shell 严格选项。

再次调用收集已结束的批次，不需逐温度推进；只读 --status 显示两方法调度状态。未完成批次恢复需 --resume；未知回执不重提，第二次提交失败保留第一次作业。每方法使用 resources 配置并单独预留共享预算，不扩大上限。
