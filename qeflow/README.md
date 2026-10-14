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
