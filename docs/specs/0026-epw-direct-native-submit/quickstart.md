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

## 当前机器安装与旧脚本

本次已覆盖仓库 `bin/epwflow/` 的三个入口。终端加载 `source scripts/env.sh` 后使用这些入口。按模块打包可使用：

```bash
bash scripts/linux/package-native-runtime.sh --module epwflow SOURCE_BIN PACKAGE_BIN NATIVE_PREFIX
bash scripts/install-bin.sh --module epwflow PACKAGE_BIN
```

现有示例的 `5.epw-wannier/submit.sh` 直接运行 `epw.x`，同目录 `submit-wannier90.sh`、`submit-postw90.sh` 分别用于独立后处理调试。PH 的 q2r/matdyn 与 QE 的 bands 各有单调用脚本。手工提交这些脚本可能重写对应原生产物，不自动更新正式工作流状态；正式推进仍使用公开命令 `-s`，每次生成并绑定独立任务脚本。旧阶段总入口、封存现场和测试总入口已退役，详细映射及原文见 [部署记录](deployment.md)。
