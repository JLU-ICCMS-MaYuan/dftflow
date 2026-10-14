# qeflow

DFTFlow 的 QE 工作流程序，负责输入生成、计算提交、状态检查和后处理。源码位于本目录，展示算例统一位于根目录 [examples](../examples/README.md)。当前实现和验收范围见 [DFTFlow 总览](../docs/overview/dftflow-rust.md)。

## 构建与安装

从仓库根目录执行：

```bash
bash scripts/build-native.sh
```

平台依赖和命令安装方式见 [根目录 README](../README.md)。

## 展示算例

- [MgB₂ / sugon-amd：QE 7.4.1 声子计算](../examples/MgB2/sugon-amd/README.md)。
- [MgB₂ / inspur](../examples/MgB2/inspur/README.md)、[MgB₂ / chem](../examples/MgB2/chem/README.md)：等待机器配置。
- [Si 通用展示资产](../examples/Si/generic/README.md)：原有 POSCAR 和旧路径文件的用途及限制。
- [合成 α²F 参考材料](../examples/synthetic-a2F/generic/README.md)：早期演示数据，当前 CLI 不支持仅凭该曲线运行。

## 基础命令

```bash
qe_scf --help
qe_nscf --help
qe_phonon_nosplit --help
qe_process_phonon --help
```

SCF/NSCF 使用 `-c/--config` 读取配置，`-i/--input` 指定结构，`-r/--run` 执行，`-s/--submit` 提交；默认不执行实际计算。声子总流程的具体运行方式和当前限制以算例手册为准。

## 分 q 点全链路（Issue 0006）

qe_phonon_split_index / qe_phonon_split_coords 保留 -c/-i，默认仅准备，-s 一次安排整个 Slurm 依赖链，-r 在已有计算资源中顺序执行。只计算一次 SCF，各 q 独立复制保存数据；显式坐标为 QE Cartesian（2π/a₀）。qe_split_submit --chain 可用于前置阶段安排 q 点与 post。作业与状态记录见 [0006 契约](../docs/specs/0006-qe-phonon-submit-chain/contracts/cli.md)。

生成的 submit_phonon.sh 用 bash 执行；不要再按旧版分阶段手动提交 merge。旧 run_split.sh/索引目录的独立工具保留。

## 直接调用 dftflow 与单独初始化

`prepare.sh` 仅为算例生成结构、配置和提交脚本。准备好 `input.toml`、`POSCAR` 和 `KPATH.in` 后，可直接调用实际命令：

```bash
# 按索引或坐标分 q：二选一，在各自独立目录执行
qe_phonon_split_index -c input.toml -i POSCAR -s
qe_phonon_split_coords -c input.toml -i POSCAR -s

# 不分 q：准备输入；本例通过准备脚本生成的 nosplit_job.sh 完成全流程
qe_phonon_nosplit -c input.toml
sbatch nosplit_job.sh
```

分 q 命令不加 `-s`/`-r` 时只准备，加 `-s` 提交完整依赖链。`-c` 指定配置，`-i` 指定结构。不分 q 的直接 `-r`/`-s` 全流程尚有已知缺口（漏跑/重复后处理与路径处理）；本例保留显式 nosplit_job.sh 路径，修复不纳入本次工具拆分。

只想取得真实 QE 的 q 点列表，已有完整 SCF 保存数据时执行：

```bash
qe_ph_nosplit -c input.toml --dyn0       # 只生成 ph_init.in 和 run_ph_init.sh
qe_ph_nosplit -c input.toml --dyn0 -s    # 提交仅初始化作业
# 已获得计算资源时才使用：
qe_ph_nosplit -c input.toml --dyn0 -r
```

注意单步命令是 **qe_ph_nosplit**，完整流程命令是 **qe_phonon_nosplit**。
`[qe_phonon].work_dir` 相对配置文件目录解析；SCF 保存树位于该工作目录的 `[qe_phonon.scf].outdir`，也可指定绝对 outdir。prefix 依次取 `[qe_phonon].prefix`、SCF prefix、结构化学式。需要 XML、电荷密度及波函数；此命令不补跑 SCF。

初始化设置 `ldisp=true, only_init=true, lqdir=true`，清除 q 点/表示范围限制，等待 QE 正常退出后检查 `ph_init.out` 和 `prefix.dyn0`。成功生成 `ph_init_complete.json` 只代表初始化完成；不计算完整声子、不提交分 q 点及后处理。已有 dyn0 在实际重新初始化前归档为 `prefix.dyn0.before-init-<时间戳>`，失败不复用旧列表。

自动分 q 流程已包含初始化，无需先手动执行该命令。自写 dyn0 生成器已拆为独立工具，主程序和自动流程不再调用它。
