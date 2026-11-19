# 分支执行契约

公开命令不变：`epw_phonon --stage electronic -s`、`--resume -s` 和 `--plot`。

内部脚本使用 `--electronic-branch qe_eband|qe_eledos --allocation-id ID --config VERSION`；不是额外公开程序。必须处于正在运行的 Slurm 分配，提交 ID、作业号和固定配置版本均须匹配；重复执行、过期脚本及错配分配拒绝。

- 能带脚本：qe_eband/qe_eband.sh；原生顺序 PW → 即时范围诊断 → bands。
- DOS 脚本：qe_eledos/qe_eledos.sh；原生顺序 dos → projwfc，同一作业号。
- 收尾脚本：阶段根 electronic.sh；提交时 afterany 依赖本轮需要执行的分支。两边科学产物合法后调用既有绘图；不完整时报告失败并保留成功结果。
- 默认准备不提交；生成的收尾脚本也可在单一分配内顺序执行尚未提交的分支。
- 所有原生进程沿用各自 launcher/args 和用户 slurm.header，不补资源参数。

新增分支段见 data-model.md；不创建第二套状态文件、旧命令备份或新编译入口。
