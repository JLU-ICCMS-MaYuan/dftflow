# 0026 快速验证

在隔离目录使用受控原生程序及替代 Slurm 命令执行公开入口进程测试，核对实际 sbatch 目标脚本、工作目录、一次科学调用、原始输入输出、重复提交保护和人工推进。受控程序不构成真实科学验收。

实际使用在 `input.toml` 所在目录执行 `epw_phonon -s`、`epw_wannier -s` 或 `epw_superconductivity -s`；作业结束后重复同一命令推进。失败修复后使用 `--resume -s`。`epw_phonon --all` 仅准备，不提交。

配置使用 `layout='numbered'`。每个目标显示 `workflow_complete: true` 后，再切换到下一个公开程序。旧非编号布局通过新的工作目录和 `--import-work` 显式导入；本轮不覆盖默认安装或旧冻结脚本。

回归入口：

```bash
bash scripts/build-native.sh test --locked -p epwflow
bash scripts/build-native.sh test --locked -p epwflow --test three_stage_execution direct_submit -- --nocapture
```

本机依赖离线来源和原生库按已有构建环境配置，不把机器路径写入共享配置。真实 QE／EPW 用例继续显式暂挂，不以这些替身测试声明真实科学结果通过。
