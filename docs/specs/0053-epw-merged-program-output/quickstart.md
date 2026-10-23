# 0053：验证与使用

## 隔离回归

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --locked -p epwflow --test three_stage_execution
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --locked -p epwflow --test slurm_log_filenames --test superconductivity
bash scripts/build-native.sh build --release --locked -p epwflow --bins
```

测试实际执行 Bash 与受控科学程序，覆盖实时输出、退出非零、旧日志、分配内链、两方法收尾和恢复；不提交真实集群任务。

## 使用

运行新编译并按既有方式安装的 CLI，在新任务中查看对应 `.out`；不再依赖科学程序 `.err`。Slurm 自身日志沿用原设置。已有任务继续按记录版本续算，不编辑绑定脚本或删除旧 `.err`。本次具体编译及安装边界见 [验收](acceptance.md)。

## 保留现场并重新计算 SCF

确认已使用本次验证后的新版本，在 MgB₂ 工作区执行：

```bash
epw_phonon -c "epwinput.toml" --stage scf --resume -s
```

无需删除 `1.qe-scf` 或 `completion.json`。即使已有成功结果，该命令也归档旧阶段到 `epw-state/history/`，在原路径生成新的 `scf.sh` 并提交新作业；科学命令使用 `> scf.out 2>&1`。活动或状态未知时拒绝重提。

作业结束后用普通提交命令归集和核验；重复普通命令复用结果：

```bash
epw_phonon -c "epwinput.toml" --stage scf -s
```

再次携带 `--resume` 会再次重算。该强制行为仅适用于显式 SCF 恢复提交，不使其他阶段的 SCF 依赖重复计算。
