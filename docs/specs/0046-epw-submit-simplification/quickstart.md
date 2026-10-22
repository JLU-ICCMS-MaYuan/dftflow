# 0046 隔离验证

在仓库根执行：

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p epwflow --test three_stage_execution
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p epwflow
```

隔离测试调用三个真实公开入口、生成并执行实际 Bash，科学程序和 Slurm 为受控替身。检查目标锁残留／中断不再造成锁错误、顺序提交不重复、脚本写法和实时 `.out`、非零退出、指定温度点与数据损坏拒绝。不会调用真实 sbatch 或改写示例计算目录。源码与默认安装分开报告。
