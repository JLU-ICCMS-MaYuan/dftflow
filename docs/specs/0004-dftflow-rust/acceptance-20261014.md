# 0004 缺口收敛验收（2026-10-14）

本次按用户授权继续 0004，测试体系仍为 MgB2。旧记录中的任务勾选不覆盖本次审计发现的问题。

## 需求到证据

| 需求 / 标准 | 修改与验证 | 状态 |
|---|---|---|
| FR-001 / SC-001，T032 | SCF/NSCF 独立配置、默认 input.toml、-i/-c/-r/-s 及长选项、目录、输入、SCF 数据继承、实际执行与失败传播；`qeflow/tests/basic_cli.rs` 对照冻结 Python MgB2 输入并覆盖进程边界 | 通过 |
| FR-003 / SC-004，T033 | ndarray-linalg 0.17 + 系统静态 OpenBLAS；删除 Jacobi；2×2 已知谱和 65×65 稠密矩阵、Eliashberg 下限状态测试；三展宽 MgB2 合成 EPC 全链严格对照 | macOS 与 Linux musl 通过 |
| FR-001 / SC-003，T034 | Cargo metadata 恰好五个 bin；独立 gen_dyn0 源码与 target 删除；修正 vasp_plot_kcohp 分派；52 个安装入口逐一 --help | 通过 |
| FR-006/007 / SC-002，T035 | macOS OrbStack 中 Linux amd64 musl 构建、ELF/依赖检查、断网测试与打包 | 通过 |

## MgB2 真实 QE 双侧对照

使用相同 MgB2.vasp、4×4×2 k 网格、ecutwfc=40 Ry、ecutrho=320 Ry；两侧均调用本机同一 QE 与 MPI，Python 为 `python-reference-0004`（9396b3a）。

| 项目 | Python | Rust |
|---|---:|---:|
| SCF 总能 / Ry | -13.55142599 | -13.55142599 |
| SCF Fermi / eV | 7.6337 | 7.6337 |
| NSCF Fermi / eV | 7.6330 | 7.6330 |
| SCF / NSCF 完成标记 | 均 JOB DONE | 均 JOB DONE |

原 Mg 赝势 `mg_pbe_v1.4.uspp` 为旧二进制格式，本机 QE 无法读取；原件未修改。真实执行在独立目录使用同一 QE 子模块 `EPW/examples/mgb2/pp/` 的 `Mg.pz-n-vbc.UPF`、`B.pz-vbc.UPF`，两侧完全相同。这是软件迁移对照，不将此小网格 LDA 运行当作 MgB2 物理收敛验收。

本机原始日志与运行输入保留于 gitignored 的 `docs/local/0004-mgb2-real/`、`docs/local/0004-mgb2-python/`。仓库中纳入小型黄金输入与可重跑集成测试，不提交计算输出。

## scflow 数值与图形

- 合成 MgB2 EPC：2 q × 6 模式 × 3 展宽；不是实际 MgB2 EPC 物理结果。既有 Python golden 保持不变。
- 所有分展宽 CSV、summary 每一列、POSCAR 精确一致；JSON 全部数值采用 `1e-8 + 1e-6 × |ref|`，状态与非数值字段精确比较。仅临时输入根路径归一化，保留源文件名精确比较。
- 删除 Tc 的 5% 豁免与 DeltaC/Tc、Delta、Hc2 跳过规则，零频原始 gamma 与状态也不跳过。
- OpenBLAS 替换后原 Tc 差异消失；旧记录将其归因于 float32/float64 没有充分证据，不能继续作为豁免理由。
- 检查每次运行七份 PDF 的文件头与结束标记；CIF 空间群 35/Cmm2、POSCAR 与 Python 参考一致。文件格式检查不等于人工视觉逐页验收。

## 本机安装

安装器预检五个源程序、拒绝源目录等于目标目录、备份被替换入口，再创建可追溯版本目录的软链接。五个家族包含已有旧名 scflow/substitution，因此总安装入口数为 48 + 1 dyn0 + 3 新家族名 = 52。

本机安装位于 `~/.local/lib/dftflow/20261014-0004/bin`，入口位于 `~/.local/bin`；zsh 登录和交互配置末尾均优先该目录，原配置备份在版本目录，conda 原 Python 脚本未删除。已从新登录交互 shell 核对命令解析结果。

## 测试范围与外部边界

- 本轮 macOS Rust 全工作区 **88 项通过**；全工作区 Rust 测试、MgB2 独立命令回归与原 Python 对照均记录各自结果，不能将 Mock 提交当作真实 Slurm 作业。
- 本轮聚焦 Python MgB2 SCF/NSCF API、三种晶格模式及错误路径：9 项通过。原晶格参数字符串断言受 cell2ibrav.x 文本末位影响，已改为同一固定数值容差，不更改期望值。
- 真实 Slurm 调度、其他计算节点已安装的 QE/VASP、可选 MatterSim 环境不由本机容器验证代替；线上 Gitee 由用户管理，未自动关闭。


## 启动与内存观察

同一 Mac 上对构建后首次 `--help` 记录 `/usr/bin/time -l`：Rust qeflow WALL 0.70 s、最大 RSS 8,634,368 bytes；Python 参考 qe_scf WALL 0.13 s、最大 RSS 36,487,168 bytes。未清除系统文件缓存，单次测量可能受 macOS 首次加载检查影响；这是观察值，不证明提速，也不作为验收门槛。RSS 不等于整个系统物理内存占用。

- 已安装基础 CLI pytest：SCF、NSCF、eband、eledos 四项通过；Rust 额外覆盖 VASPKIT 标题、能带三步与 DOS 两步子进程执行及 SCF 数据继承。


## 进一步补齐

- 四个基础 CLI 的已安装命令 pytest 全通过；能带路径使用真实 VASPKIT 生成格式，回归覆盖四列非数值标题。
- 独立 NSCF/eband 坐标输出 8 位小数、SCF/phonon 输出 17 位，晶格为 Python `.12g` 的有效数字规则。该差异由真实 MgB2 计算对照发现并修复；完整数据重验结果另记。


## 真实电子结构数据最终对照

修正输入精度后重跑 Rust 的 SCF→NSCF→eband→eledos，与冻结 Python 参考同参数计算比较：能带 GNU 数值表、能带总投影/分原子轨道投影、DOS 总表及分原子轨道投影共 **16 张数值表**，形状全部相同，所有解析数值的最大绝对误差为 **0.0**。原始比较记录为本地 `docs/local/0004-mgb2-real/electronic-comparison.json`。


## Linux 静态交付最终结果

- 构建主机：当前 macOS，通过 OrbStack 的 Linux x86_64 容器；不是远程 Linux 节点。
- 固定 Rust 镜像摘要见 Dockerfile，实际 rustc **1.99.0 (b940084d7 2026-09-28)**；OpenBLAS **0.3.30**、spglib **2.6.0** 源码均固定版本并校验 SHA-256。
- 五个程序全部为 x86-64 ELF、static-pie、stripped；`scripts/linux/verify-static.sh` 验证无 PT_INTERP 和 DT_NEEDED，不依赖目标机安装同版本 OpenBLAS/spglib 共享库。
- **Linux release 88 项通过；断网容器 --offline 88 项通过；52 个入口通过。** 已 strip 的 scflow 单独运行 MgB2 合成 EPC，summary 与冻结 Python golden 逐字节相同。
- 交付包：`dist/dftflow-linux-x86_64-musl.tar.gz`；五个 ELF 约 40 MiB，压缩包约 16.2 MiB。内含逐文件 SHA256SUMS、BUILD-INFO.txt 和安装器。
- 此验证证明目标架构的构建、静态链接及离线运行；真实计算节点上的 QE/VASP、赝势、Slurm 权限与可选 MatterSim 环境仍由部署条件决定。绘图需要目标环境可用的字体数据，构建验收镜像使用 DejaVu；静态库不等于内嵌 QE/VASP 或字体资源。

## 本轮纳管输入哈希

```text
b29069dfd034db0ecd61423f52170d6fc343a714fbd9f05fa3b9895fba4b0b02  test_auto/MgB2/MgB2.vasp
0cc2355b6cd2864eab90e710737882c5d65fcea21cc2bdc50b5dda0a2fd66f5f  test_auto/MgB2/input_qe.toml
df2bf0fa82d6a3279712c52df9cc347295a89cca3f73b1154a6b174a2c1123c3  test_auto/MgB2/rust-reference/KPATH.in
2a3c18717aef16a7755dabfe01c3ca5be2831a68aab06c7934b40d619753906c  test_auto/MgB2/rust-reference/nscf.in
ddfe40da759f9e990d668517d596e0ae7bae12022dfc9a3cf284b292b472db8a  test_auto/MgB2/rust-reference/scf.in
7f1438082129857815c2d3147ca8b171f8aac333ea589c135cedba7b0d179bc4  test_auto/MgB2/rust-reference/grid.xml
```
