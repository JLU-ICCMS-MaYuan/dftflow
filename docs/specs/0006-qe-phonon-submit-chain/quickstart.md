# 快速验收

在仓库根目录执行 `bash examples/MgB2/prepare.sh sugon-amd index`（coords/nosplit 同理）。准备不运行计算；检查生成的 input.toml 和提交入口，按输出路径进入运行目录后 `bash submit_phonon.sh`。用 squeue/sacct 及各阶段日志确认依赖链。输出 phband/phdos 的 CSV、PNG 和联合 JPG，SCF 日志只一份且无逐 q pw 执行。

在相同 QE/赝势/结构/网格上跑两模式与 nosplit：对应频率误差 ≤0.1 cm⁻¹，DOS 积分相对误差 ≤10⁻³。记录版本、作业号、输入哈希和制品哈希。重复启动应复用完成结果或已有活动作业。

## 单独取得 q 点列表

已有 SCF 数据时直接使用 `qe_ph_nosplit -c input.toml --dyn0 -s`。省略 `-s` 仅准备；`-r` 用于已有计算资源。完整分 q 流程仍直接运行 `qe_phonon_split_index -c input.toml -i POSCAR -s` 或 `qe_phonon_split_coords -c input.toml -i POSCAR -s`，内部自动初始化。离线小工具见 [独立 dyn0 手册](../../../tools/qf-gen-dyn0/README.md)。
