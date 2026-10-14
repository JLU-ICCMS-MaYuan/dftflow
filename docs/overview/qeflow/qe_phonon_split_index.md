# qe_phonon_split_index：按索引拆分声子全流程

[返回 qeflow 索引](README.md)

## 功能与入口

一次 SCF → QE only_init → 各 q 点 PH → 合并 → q2r → matdyn 声子谱与 DOS → 收集与绘图。`ldisp=.true.`，用 `start_q=last_q=i` 计算 dyn0 中第 i 个不可约 q 点，i 从 1 开始。

等价入口：`qeflow qe-phonon-split-index`。

## 使用方法

```bash
qe_phonon_split_index -c input.toml -i POSCAR
# 一次提交完整 Slurm 链
qe_phonon_split_index -c input.toml -i POSCAR -s
# 或在已有计算资源内顺序完成
qe_phonon_split_index -c input.toml -i POSCAR -r
```

`-c/--config` 默认 `input.toml`；`-i/--input` 指定结构（也支持 `-p/--poscar`）。不带执行选项时生成输入与脚本；`-r` 在当前资源中运行，`-s` 提交作业，二选一。 两条执行命令是替代用法，不要同时执行。

## 输入、输出与衔接

默认 `[qe_phonon.scf].outdir` 为 `./tmp`，SCF 与 PH 初始化/分 q/合并共用该设置；显式设置其他目录会覆盖默认值。已有运行目录若显式使用 `./out`，继续按原配置读取，不自动搬迁或改写历史输入。

默认工作目录 `qe_phonon_split_index/`，可由 `[qe_phonon].work_dir` 覆盖；新链路相对路径以配置文件目录为基准。生成 `submit_phonon.sh`、`q_index_map.json`、`scf_snapshot/`、数字 q 目录和作业记录；成功输出见 [声子工作流](phonon-workflow.md)。

## 使用边界

每个 q 复制独立可写 SCF 数据，不逐 q 重算 SCF。-s 使用 Slurm afterok 依赖；失败会阻断下游。原输入不变时可重复入口续算；修改结构/配置/模式须使用新目录，已有输入绑定会拒绝混用。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[完整链路](../../../qeflow/src/phonon_chain.rs)；[链路覆盖](../../../qeflow/tests/phonon_chain.rs)。默认目录的回归与安装验证见 [0006 US4 验收](../../specs/0006-qe-phonon-submit-chain/acceptance.md)。

参见 [0006 验收记录](../../specs/0006-qe-phonon-submit-chain/acceptance.md)。
