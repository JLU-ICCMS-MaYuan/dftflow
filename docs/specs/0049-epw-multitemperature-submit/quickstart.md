# 快速验证

保持有效第 7 步数据，设置 task="gap"、mode="both"、mu_star=0.13 与 gap_temperatures_k=[20,30,40]，执行：

```bash
epw_superconductivity -c "epwinput.toml" --stage superconductivity -s
```

应看到两个独立作业号及 iso-mu0.13-20-40／aniso-mu0.13-20-40。每份 epw.in 包含 nstemp=3、三个 temps，每脚本一次 EPW 调用。作业结束后再次执行同命令收集结果，成功点不重算；故障修正后加 --resume -s 只补未完成点。

源码隔离验证：

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p epwflow --test three_stage_execution
```

该命令使用受控原生／Slurm 替身，不是实际科学运行或安装验收。
