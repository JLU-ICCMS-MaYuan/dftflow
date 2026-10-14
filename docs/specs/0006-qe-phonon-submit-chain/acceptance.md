# Issue 0006 验收记录

## 结论与边界

FR-001～006、SC-001～004 的本地实现、自动测试、真实计算与安装验证完成。线上 Issue 和双向远程链接仍由用户管理，没有创建、推送或关闭线上 Issue。小网格仅验证流程，不宣称材料参数收敛。

## 自动验证

- workspace release/offline：96 项通过（原 88 项加新增 8 项）。最终 CLI 聚焦回归 16 项通过。
- 5 项跨进程链路测试：原生 Slurm 协议、一次 SCF、依赖顺序、数据隔离、部分提交失败和续算、查询失败、无效作业号、单 q 失败阻断、两种模式及旧目录兼容、配置变更拒绝复用、重复完成项跳过。
- 新增非正交倒格转换测试及实际 outdir 严格合并测试；错误 q 坐标时不产生部分合并，patterns 逐 q 命名无嵌套复制。
- Bash 语法、git diff --check 和新增/更新手册链接检查通过。

测试日志只存 docs/local/0006-workspace-verified.log、0006-cli-final.log、0006-data-tests.log。

## 真实 QE 7.4.1 / sugon-amd

输入：项目 MgB₂ 原胞，PBE Mg/B v1.4 UPF，k=4×4×2、q=2×2×1、ecut=40/320 Ry。两个不可约 q 点。每个流程只有一份 SCF 输出，PWSCF 启动计数为 1；数字 q 目录没有 scf.in，仅复用独立保存数据执行 PH。

| 验收组 | index pre / q / post | coords pre / q / post | nosplit |
| --- | --- | --- | --- |
| 单节点 64 MPI 首轮 | 1250611 / 1250613,1250614 / 1250615 | 1250616 / 1250618,1250619 / 1250620 | 1250658 |
| 最终制品单节点 32 MPI 补验 | 1250681 / 1250682,1250683 / 1250684 | 1250689 / 1250690,1250691 / 1250692 | 1250705 |

上述作业均 COMPLETED、ExitCode=0:0。nosplit 1250658 从空目录一次完成全部步骤。最终制品补验使用 32 MPI、8 GiB、10 分钟，利用混合节点空闲配额；正式示例仍是单节点 64 MPI。

## 数值标准与结果

标准在实施前固定：对应动力学矩阵/谱线频率差 ≤0.1 cm⁻¹，总 DOS 积分相对差 ≤10⁻³。分别使用相同 MPI 配置的 nosplit 对照，没有放宽门槛。

| 配置与模式 | 动力学矩阵最大差 cm⁻¹ | 谱线最大差 cm⁻¹ | DOS 积分相对差 |
| --- | ---: | ---: | ---: |
| 64 MPI index | 0 | 0 | 0 |
| 64 MPI coords | 0.000073 | 0.000100 | 1.92761×10⁻⁶ |
| 32 MPI index | 0 | 0 | 0 |
| 32 MPI coords | 0.000563 | 0.000600 | 3.85670×10⁻⁶ |

所有模式的四份 CSV、独立声子谱/DOS PNG 和联合 JPG 存在且非空，图片经 PIL 解码核验。两种 split 有 chain_complete.json，调度/状态记录均可追溯。报告与输入哈希见 docs/local/0006-real-acceptance.json、0006-final-runtime-acceptance.json。

## 真实失败边界与修复

1. 登录节点 glibc 2.28、计算节点旧版导致首次启动失败（1250581/1250583），nosplit 1250584 的 QE 步骤完成但 Rust 后处理未启动。采用用户级配套 glibc 加载器打包，未修改系统或全局 LD_LIBRARY_PATH；原 nosplit 用 1250621 恢复后处理。源码/打包二进制分别记录哈希，保留原失败日志。
2. 三个流程同时提交触发 AssocMaxSubmitJobLimit（1250655/1250657），部分 q 点被依赖失败取消，作业号保留；未误执行 post。账户查询存在 MaxSubmitJobs=4/8 限制。手册明确 q 点数 + 2 的提交额度要求，按顺序验收；不增加未经确认的分批拓扑。
3. 整节点重复验收 1250666 预计排队两天，已取消；保留首轮成功结果，最终制品改在空闲 32 核配额补验。正式示例配置没有改成 32。
4. 32 与 64 MPI 的交叉比较曾得到谱线差 0.2884 cm⁻¹，DOS 积分相对差约 0.00431；该比较不是同 MPI 配置的有效对照。重新独立生成同配置 32 MPI nosplit 后通过，未混用参考或放宽标准。这也说明当前小网格/阈值不能代替物理收敛研究。

## 技能、安装与制品来源

使用 skill-installer 安装 Big-Project-Skill 固定提交 2ed8ea610627f16c53e5fe49184144ec759014ea 的六个技能；20 个文件与上游归档逐字节一致。本 Feature 使用 issue-manager、spec-runner 和 overview-maintainer；用户指定本地 Issue 及完整实施授权优先于默认线上门。

最终版本目录名为 linux-x86_64-0006-20261014，五份二进制通过 scripts/linux/package-native-runtime.sh 打包，52 个兼容入口安装至当前用户全局命令目录。登录节点 52 项检查通过，计算节点作业 1250719 的清洁 PATH 检查也全部通过。源构建五份 SHA256SUMS 与打包来源清单核对通过；patchelf 仅修改加载器/RPATH。

源码清单、技能安装文件哈希、二进制哈希和安装证据保存在 docs/local/0006-source-manifest.json、0006-skill-install.json、0006-install.log、0006-package-final.log、0006-node-entries-1250719.out。提交后来源信息补记到本地交付记录。

## 文档与非目标

根/qeflow README、三个机器手册、Overview、AGENTS 与本 Feature 标准产物已同步。inspur/chem 未冒充完成真实验收；原始运行文件保留归档，运行目录和本机信息均不跟踪。

## US3 补充验收：独立 dyn0 工具与初始化入口

### 已通过

- 主 workspace release/locked/offline：97 项测试通过。原生成器 2 项单元测试和 1 项内嵌别名测试移出，新增 4 项跨进程初始化测试；两种分 q 链路及非正交转换回归通过。
- 独立工具：4 项测试通过，目录复制至仓库外后 offline/locked 测试和 release 构建再次通过；MgB₂ dyn0 与 q_points.json 和拆分前入口产物逐字节一致。无效网格、乘积溢出及缺失 XML 返回失败。
- 初始化：默认仅准备、-r 执行、-s 原生提交且生成脚本实际执行均覆盖；不同启动目录、非默认 outdir、SCF prefix 解析、缺失电荷/波函数、QE 日志报错、dyn0 不完整和本次未生成 dyn0 均已验证。只运行 ph.x，不启动 pw.x、q2r.x、matdyn.x；仅写初始化成功标记。
- 新版五二进制打包完成；隔离安装的 51 个入口检查通过；旧 dyn0 链接备份移除、用户独立程序与独立链接保留的迁移回归通过。
- 主 workspace 元数据不包含独立工具，主源码不包含生成器模块或 gen-dyn0 子命令；构建来源五份哈希与打包清单一致。

### 真实初始化与部署待办

- 已在 sugon-amd 提交短时单 MPI 初始化作业 **1250893**，配置非默认 outdir，使用已有 MgB₂ SCF 快照的独立副本；不改原计算。仅验证初始化，正式算例默认 64 MPI 保持原配置。
- 调度状态为 PENDING / Priority，预计 2026-10-16 15:58:35 启动；尚无真实初始化输出，不能宣称本轮真实验收通过。
- 新制品暂存版本目录，默认安装尚未切换；按原方案等待真实初始化成功。不能把上一轮 0006 的实际验收当作本次新入口验收。
- 后续验收：sacct 核对 COMPLETED/0:0，ph_init.out 含正常终止且不含错误，dyn0 与原 QE 列表一致，ph_init_complete.json 的 q_count=2，无新 SCF/完整 PH/后处理；原始 SCF 快照哈希不变。通过后切换全局入口并验证 51 个入口与源码/制品来源，更新任务状态。

本轮证据位于忽略目录 docs/local/0006-dyn0-decouple/：tests-final.log、tool-detached-tests.log、entries.log、install-migration.log、source-manifest.json、delivery.json、original-scf-sha256.json 和 real-init/。线上 Issue 仍由用户管理。
