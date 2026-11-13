# 0065 验收记录

## 结论与边界

2026-11-13 完成统一 Slurm header／launcher 与覆盖构建需求。生产代码已移除 Resources 模型和重复 MPI／节点参数；Slurm 资源指令来自用户 header，公开提交不再覆盖。固定公开命令已经重编译覆盖；没有编译版本记录、版本安装目录或入口备份。未提交新的科学作业，不声称 MgB₂ 的数值效果已经验收。

## 需求与验证

| 需求／标准 | 实现与证据 |
| --- | --- |
| FR-001／SC-001 | Config／Document 复用 qf-config 的 SlurmSection；旧 resources 测试通过，错误包含文件与迁移提示 |
| FR-002／SC-002 | script_header 覆盖编号、原生与旧生成入口；公开提交测试保留自定义 partition、nodes、ntasks、CPU、输出文件，提交 argv 不覆盖；无 header 不增加资源指令 |
| FR-003／SC-003 | mpi_count 从 launcher 只读取得数量；移除 SLURM_NTASKS 相等检查与公开 --nodes／--mpi；恢复测试使用较大的分配，MPI／pool 调整后上游结果继续复用 |
| FR-004／SC-004 | 账本接口仅登记 label／job，不依赖虚构资源；实际消费仍来自 Slurm accounting；未知／活动作业、防重复和失败恢复回归通过 |
| FR-005～006／SC-005 | 审计 QE、VASP、Wannier、SC、替位及共享执行组件，其他模块已经沿用 header；活动夹具、SiP₂ 和 Ubuntu 输入迁移，当前文档同步 |
| FR-007／SC-006 | 删除 published 编译记录和可选入口备份；实际 Cargo 发布专项与安装兼容回归通过；全工作区 release 覆盖 9 个固定命令，逐字节核对缓存制品与公开文件一致 |

## 回归结果

- EPW 分批执行去重后 **359 通过、15 既有 ignored、无未解决失败**。ignored 不算通过，不新增真实科学执行。
- 全工作区其他模块串行回归 **140 通过、0 失败**。
- `scripts/tests/build-publish.sh` 通过：实际 Cargo、fresh、模块隔离、非交付、编译／启动失败、目标冲突、活动进程／作业、独立工具与带空格路径；断言没有 published 记录。
- `scripts/tests/install-compat.sh` 通过：旧别名直接移除且不留备份，独立工具与链接保留；即使设置旧备份开关也不产生备份。
- 修改的 Shell 脚本语法、TOML、本地链接及 git diff --check 通过。

初次 EPW 总回归在旧“--check 不生成 run”断言处停止；该断言与现有保存配置来源行为不符，已改为检查不生成计算阶段／提交脚本。分批补跑三程序、完整阶段恢复、能窗与超导回归。分配数量大于启动数时，恢复测试替身原先要求两者相等，已改为允许分配更多资源并单项复测通过。没有将失败命令冒充单次完整通过。

其他模块初次 4 线程执行时，qf-input 两测试复用进程临时目录并删除对方文件；不改无关生产代码，全套改为单线程复测通过。

可复查的本机完整日志留在忽略目录 `docs/local/0065-slurm-header-launcher/`，不纳入 Git。

## 实际构建与入口

执行 `bash scripts/build-native.sh build --release --workspace --locked -j 6` 成功，覆盖 wannier90、scflow、vaspflow、substitution、qeflow、epw_clear、epw_phonon、epw_wannier、epw_superconductivity。测试结束后再次执行同一构建命令，确保最终公开入口来自 build 制品，9 个文件与本次 release 逐字节一致。安装器只保留固定模块路径和兼容链接，没有第二套版本安装。不删除构建缓存或科学来源快照。

## Ubuntu MgB₂

正式三个 EPW 入口均对 `examples/MgB2/ubuntu/input.toml` 返回 configured／检查成功，workflow_complete=false。实际生成 `run/1.qe-scf/scf.sh` 并检查用户资源指令、launcher 与 ~/bin 环境脚本，无 sbatch 或科学调用。

首次输入的 gap 模式缺少 tc_linear=false、gap_edge=0.01 eV、必要 EPW pool；另外 campaign 身份和工作区内账本位置不符合既有契约。已补齐输入，以 run 为计算子目录、campaign 为同级账本目录，仍全部位于 ubuntu 内。没有改变网格、截断能或温度，也没有放行未完成科学阶段。

## 文档与协作

Issue／Spec 双向本地链接、原始需求与实施授权完整保留；当前根说明、模块手册、Overview、Ubuntu 手册和 AGENTS 同步。线上 Gitee 由用户管理，当前 origin 不匹配，未操作线上 Issue，未声称远程链接完成，未推送。
