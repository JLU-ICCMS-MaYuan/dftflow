# 0053：科学日志接口

无新增 CLI 或 TOML 配置。三个公开程序的新原生任务统一生成：

```bash
mpirun -np N xxx.x < xxx.in > xxx.out 2>&1
postw90.x seed > postw90.out 2>&1
```

原参数、引号、种子接口及 Slurm 头部保持原行为。日志实时写入，不保证不同 MPI rank 的输出排序。退出状态仍由原生程序和调度器共同核验，不把 `.out` 存在视作计算成功。

新任务只产生科学 `.out`，版本 1／2 的已有 `.err` 仍按原契约读取；不会自动迁移已有运行。构建不等于安装，新脚本需由新版本程序正常生成，冻结脚本不得直接改写。

## 显式 SCF 重跑

```bash
epw_phonon -c "epwinput.toml" --stage scf --resume -s
```

即使 SCF 已成功，每次调用也在确认旧任务已终止后归档旧阶段、创建新批次，并在 `1.qe-scf/scf.sh` 生成当前格式；无需用户删除阶段或 `completion.json`。旧脚本、日志和快照保存在 `epw-state/history/`，活动／未知任务仍拒绝重提。重跑结束后用不带 `--resume` 的同一命令 `--stage scf -s` 归集；再次携带 `--resume -s` 将再次重算。其他阶段的恢复契约不变。
