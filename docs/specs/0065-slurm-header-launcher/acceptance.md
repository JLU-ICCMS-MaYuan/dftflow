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

## 2026-11-17：显式程序路径、独立参数与 EPW 单 image

### 实现与需求对应

- FR-008／SC-007：新增统一 Programs 字段映射，外部只使用 `[programs]` 下的 `*_x`／`*_args`；内部保留 Program 接口。旧程序子表明确提示迁移，未知字段、错误参数类型和无路径的非空 args 拒绝。
- FR-009～010／SC-008：bands、DOS、projwfc、q2r、matdyn、postw90 只使用各自显式路径和 argv；移除同目录推导、postw90 回退及隐式 `-pd`。共享 qeflow 包装器传递独立 q2r／matdyn 参数，快照和交付清单记录显式程序来源。
- FR-011／SC-009：配置验证、阶段预检及底层 EPW 执行／超导尝试均拒绝多 image，覆盖 `-ni`、`-nimage`、`-nimages`、原生别名 `-npot`；未填或显式单 image 可用。PH 的 image 参数独立保留，pool 别名保留。
- FR-012／SC-010：迁移活动 MgB₂／SiP₂ 示例与隔离测试；原先自动添加的 pencil 参数改为在活动示例中显式声明。此次示例提交只修改程序配置，不改变结构、科学参数或其他任务的设置。

### 验证与制品

最终以当前已提交源码加本次独立补丁建立隔离源码和构建缓存，执行 `bash scripts/build-native.sh test --release --locked --offline -p epwflow --no-fail-fast -j 4`：34 个测试目标共 **404 通过、0 失败、16 项既有 ignored**；未筛除失败用例，ignored 不计通过。

公开连续进程测试把九个程序放到各自带空格的不同目录，实际执行 SCF／NSCF／电子／PH／Wannier／粗细网格替身；验证各自 args、空列表及 postw90 种子接口。共享声子测试实际运行新生成的包装器，核对 q2r／matdyn 的独立参数。替身结果不作为真实科学验收。

从同一已验证源码重新执行 release 四入口构建，使用实际 Cargo JSON 报告调用仓库发布器，覆盖固定 `bin/epwflow/` 下四个入口。四个安装文件与本次 release 制品逐字节一致；没有版本安装目录、旧命令备份或第二套公开入口。安装后三个计算 CLI 使用真实路径的隔离配置返回检查成功；旧程序子表迁移诊断、多 image 拒绝和四份安装哈希均通过。未提交真实科学作业，未改历史冻结配置、脚本或科学文件。

### 并发改动隔离

任务开始时工作树已包含大量其他未提交修改，执行期间另有任务改动并还原部分本次代码。用户已答复“有其他任务，我会暂停它”。最终测试和安装均来自“原已提交源码＋本次补丁”，其他目录整理、预算和超导修改没有打包或暂存。

早期在混合工作树发现的温度搜索校验、无原生求解拒绝和完整阶段链三个失败，已在任务开始时保存的源码副本复现；它们不作为本次最终结果。最终上述未筛除的完整回归全部通过。提交按任务开始基线分离，仅纳入本次变更；保留其他未提交修改。

完整日志、安装核对和隔离命令证据保存在忽略目录 `docs/local/0065-slurm-header-launcher/2026-11-17-explicit-programs/`。线上 Issue 和远程链接仍由用户管理，本次仅更新本地关联文档；不推送。
