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

### 真实初始化与部署进展

- sugon-amd 短时单 MPI 初始化作业 **1250893** 已于 2026-10-14 17:17:02 启动、17:17:23 完成，sacct 状态 COMPLETED、退出码 0:0；此前预测 10 月 16 日启动仅为当时排队估计。
- 配置使用非默认 outdir，计算基于已有 MgB₂ SCF 快照的独立副本。QE 7.4.1 正常终止，无错误标记；dyn0 与原 QE 列表逐字节一致，ph_init_complete.json 的 q_count=2。
- 未产生新 SCF、完整 PH 或后处理输出；原始 SCF 快照逐文件 SHA-256 不变。该验收验证单步初始化入口，正式算例默认 64 MPI 配置保持原状。
- 用户随后明确要求「全部重新编译，我要新版的代码。」；本轮从全新 target 目录重编译五份主程序与独立工具并切换默认安装，交付证据见下节。

本轮证据位于忽略目录 docs/local/0006-dyn0-decouple/：tests-final.log、tool-detached-tests.log、entries.log、install-migration.log、source-manifest.json、delivery.json、original-scf-sha256.json 和 real-init/。线上 Issue 仍由用户管理。

### 全量重编译与安装完成（2026-10-14）

- 使用新的空 target 目录，Linux x86_64 配置、锁定依赖、offline/release 重新编译五份主程序，workspace 97 项测试全部通过；独立工具使用独立工程和另一个空 target，4 项 release 测试通过。未复用旧 target 编译产物。
- 五份主程序完成本机 glibc 加载器打包和来源 SHA-256 核对；安装器备份旧入口后切换至 linux-x86_64-20261014-180957。51 个主入口的隔离验证与实际安装核验均通过。
- 独立 qf-gen-dyn0 单独编译、打包、安装，不属于主 workspace 或主安装器；终端命令指向独立制品，输出与冻结 MgB₂ 基线逐字节一致。主 qeflow 不再接受 gen-dyn0 子命令。
- 真实初始化 1250893 已通过，T013/T014/T015 收尾完成；原有版本与计算目录保留。
- 本轮证据在 docs/local/rebuild-20261014-180957/：workspace-tests.log、workspace-build.log、tool-tests.log、tool-build.log、entrypoints.log、installed-entrypoints.json、install.log、tool-artifact.json、source-manifest.json、real-init-acceptance.json、delivery.json。docs/local/latest-rebuild.json 指向最新交付记录。

## US4 补充验收：默认 outdir 统一为 ./tmp

- FR-010：内置配置、PH 输入、初始化保存树检查、自动链快照与分 q 目录、独立合并及旧 coords 拆分回退均采用 ./tmp；显式配置仍覆盖默认值。新 MgB2 准备脚本同步写 ./tmp，历史运行目录保留。
- SC-007：`bash scripts/build-native.sh test --release --locked --offline --target-dir target/rebuild-20261014-180957 -p qeflow -p qf-config -p qf-input -p qf-phonon` 共 **32 项通过**。外部 QE 替身从真实生成输入解析 outdir 并检查保存文件，未再硬编码 out 路径。
- 新增 `default_tmp_runs_both_modes_and_standalone_merge`：省略 outdir 的 index/coords 完整模拟链均正常完成，SCF 各运行一次，根及各 q 的 tmp 保存树、PH patterns 合并与公开 qe_merge 入口通过。新增 `omitted_outdir_initializes_existing_tmp_save`：单独初始化从已有 tmp 读取数据，只执行 PH。原显式 ./out 和 custom-save 测试继续通过。
- `bash -n examples/MgB2/prepare.sh`、Overview/Spec 相对链接与 Git diff 空白检查通过。
- 五份主程序 release/locked/offline 构建并以新版本目录打包安装；隔离安装的 51 个入口检查通过，五份源/打包/已安装制品指纹核对通过。安装后实际调用 nosplit/index/coords/--dyn0 的仅准备模式，SCF/PH 输入均含 `outdir = './tmp'`；显式 ./out 初始化输入保留该值。独立 qf-gen-dyn0 安装目标保持原样。
- 本轮是默认目录和程序衔接验证，**没有重新提交真实 QE 数值作业**，不将模拟回归作为新的数值对照证据；既有数值验收仍以此前记录为准。

本机证据：docs/local/outdir-tmp-tests.log、outdir-tmp-build.log、outdir-tmp-package.log、outdir-tmp-entries.log、outdir-tmp-install.log、outdir-tmp-delivery.json 与 outdir-tmp-installed-check/。真实路径和生成输入不进入 Git。

## US5：nosplit 结构输入参数（2026-10-14）

- FR-011/012、SC-008：`bash scripts/build-native.sh test --release --locked --offline -p qeflow --test nosplit_input`，2 项通过。测试真实 CLI 准备路径（无 QE/Slurm 执行）：四种参数拼写均覆盖旧工作目录 POSCAR，并核对生成 scf.in 的晶胞；未指定时覆盖工作目录 POSCAR、当前 POSCAR、CONTCAR 三种来源；非法显式文件报错且旧结构未变。
- 新构建 `qeflow qe-phonon-nosplit --help` 显示 `-i, --input <INPUT> [aliases: -p, --poscar]`。
- 已更新本地 Issue 草稿、Spec/Plan/Research/Tasks/需求质量门、README、nosplit 与三模式 Overview、AGENTS。线上 Issue 和远程链接仍由用户管理，未宣称关闭。
- 本次仅源码修复与本机原生 release 验证，未切换 PATH 中的安装版本；未运行真实 QE 计算，不将此项视为 nosplit 全链路修复。

附加回归：`basic_cli` 3 项、`phonon_chain` 6 项、`phonon_init` 5 项全部通过，本次共 16 项；本地运行日志 `docs/local/nosplit-input-regression-20261014.log`。编译仅有既有未使用项及依赖未来兼容警告，无错误。

### US5 本机命令更新

用户授权“更新新命令”后，qeflow 已由源码提交 b83c9ca 对应的 release 构建打包安装；其他四程序与旧版一致。51 入口帮助通过，安装别名在隔离目录实际使用 -i 生成输入且晶胞/结构一致。旧版本及入口备份保留；本轮未申请计算节点或提交 QE 作业。来源哈希、安装位置与帮助证据见本地 docs/local/nosplit-input-install-20261014.json；不将 squeue 查询问题记为已修复。

## US6：三种声子命令精简输出（2026-10-14）

- FR-013/014、SC-009：`bash scripts/build-native.sh test --release --locked --offline -p qeflow --test nosplit_input --test phonon_chain --test phonon_init --test basic_cli`，18 项通过（3+7+5+3）。本地日志 docs/local/0006-us6-tests.log。
- 真实 CLI 准备/提交输出验证覆盖 nosplit/index/coords：仅准备一条带工作目录状态，提交一条带作业号状态，无逐文件清单。Slurm 边界使用假 sbatch，未向真实队列提交任务；模拟拒绝提交时退出失败、保留错误且不报告提交成功。原结构参数、两模式完整链模拟与初始化失败边界仍通过。
- 完成状态统一措辞，未改计算、写文件、提交或续算的调用顺序。保留 nosplit 全链路限制及 squeue 无效作业号问题，未将文案更新记为这些问题修复。
- 用户明确授权更新本机入口；安装后的验证与制品来源见本地 docs/local/0006-us6-install.json。旧版及入口备份保留；其他四家族程序复用上一版本，未宣称重新编译。

安装验证通过：51 个默认入口帮助正常，三个已安装别名在隔离目录的准备及假 sbatch 提交输出均与约定单行文案一致。源码及打包制品分别记录哈希；安装时源码待本次提交，随后将本地清单 source_commit 对齐最终提交。

## 2026-10-15 追加验收

- 命令：`bash scripts/build-native.sh test --release --locked --offline -p qf-config -p qf-input -p qf-phonon -p qeflow`；最终 43 项通过，日志 `docs/local/phonon-additions/tests-verified.log`。旧 scflow 编译警告未在本次修改范围。
- 安装已验证 qeflow，沿用私有 glibc 加载器并保留旧版本；其他四个程序原样复制。51 默认入口帮助、三聚合入口模拟 sbatch 提交与快照脚本检查通过。安装版绘图与原生测试制品哈希一致，源码和制品哈希见 `docs/local/phonon-additions/install.json`。
- 不执行真实 QE、不提交真实 Slurm、不迁移现有运行目录；线上 Issue 仍由用户管理，远程双向链接不冒充完成。
- FR-015/016 / SC-010：index、coords 真实 CLI 生成提交脚本，在外部配置切换目录、外部结构被替换后依次执行 pre、两 q 脚本、post（QE/Slurm 为替身），原目录完整完成。随后删除外部配置、破坏工作 POSCAR，通过 input-init.toml 续算恢复结构；篡改快照或错误模式明确失败。
- nosplit 已安装模拟提交脚本的 collect/plot 命令固定使用 input-init.toml 与 selected.vasp-init；输入快照中固定工作目录与解析后的赝势目录，结构逐字节一致。自动查找结构记录实际来源名（含 CONTCAR），不统一改成 POSCAR-init。
- 有快照目录不覆盖快照；无快照旧目录需核对原绑定再显式准备。旧运行脚本固定的旧二进制路径不会因安装改写，本轮不恢复或重提旧 coords 取消作业。原 squeue 无效作业号问题仍不在范围内。
