# qe_phonon_nosplit：不分 q 的声子流程入口

[返回 qeflow 索引](README.md)

## 功能与入口

使用一个 ph.x 任务计算网格中的不可约 q 点。默认准备输入；`-r` 依次运行缺失的 SCF、PH、q2r 和两次 matdyn，结束检查后归档绘图；`-s` 提交聚合脚本。真实目标集群的完整科学验收仍需单独核对。

等价入口：`qeflow qe-phonon-nosplit`。

## 使用方法

```bash
qe_phonon_nosplit -c input.toml -i POSCAR
# 仅当已按算例生成并检查此脚本时，才提交它
sbatch nosplit_job.sh
```

`-c/--config` 默认 input.toml；`-i/--input` 指定结构文件，支持 `-p/--poscar` 别名，与两个 split 入口一致。显式文件优先于已有工作目录 POSCAR；文件不存在时报错。不传结构参数时优先复用工作目录 POSCAR，否则依次查找当前目录 POSCAR、CONTCAR。本地执行与提交脚本均使用这一份实现。

仅准备时输出一条准备状态与工作目录；`-s` 提交成功时输出一条 `已提交声子流程作业 <作业号>`，不打印脚本和输入文件清单。错误继续显示，完整流程的既有限制见下文。

## 输入、输出与衔接

默认 `[qe_phonon.scf].outdir` 为 `./tmp`，SCF 与 PH 初始化/分 q/合并共用该设置；显式设置其他目录会覆盖默认值。已有运行目录若显式使用 `./out`，继续按原配置读取，不自动搬迁或改写历史输入。

默认工作目录 `qe_phonon_nosplit/`，可用 `[qe_phonon].work_dir` 覆盖；准备 SCF、PH 与后处理输入。`nosplit_job.sh` 是算例准备脚本生成的文件，不是该 CLI 必然生成的文件。

## 使用边界

已修复 `-r` 漏执行 q2r／matdyn 和 `-s` 重复后处理；本地执行会核对每一步日志，q2r 失败时不运行 matdyn／绘图。进程替身回归验证了完整执行顺序，不等同于真实目标机器的科学通过。

配置 `[qe_phonon].retain_epw_data=true` 可为 EPW 生成 XML 动力学矩阵、逐 q 响应势并在完成后归档到 `save/`；清单带 SHA-256，原 SCF／PH scratch 保留。默认关闭，首版仅支持 nosplit，分 q／单独初始化明确拒绝。缺失或有歧义的 dvscf、归档冲突和内容变化不能发布或幂等复用。该清单证明归档角色与文件身份；EPW 的物理来源、模式和扰动基底仍由消费端严格校验。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/lib.rs)；[薄二进制入口](../../../qeflow/src/bin/qeflow.rs)；[原生 PH 归档](../../../crates/qf-phonon/src/epw.rs)。默认目录的回归与安装验证见 [0006 US4 验收](../../specs/0006-qe-phonon-submit-chain/acceptance.md)。

参见 [三种模式与实际验收边界](phonon-workflow.md)；[0006 验收](../../specs/0006-qe-phonon-submit-chain/acceptance.md)。
