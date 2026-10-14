# DFTFlow AGENTS.md

## 一、基础语言规范
- 所有问答必须使用中文，避免歧义。
- Markdown文档需按“层级清晰、分类规整、重点突出”原则排版。

## 二、需求处理流程
1. **需求优化**：明确需求边界、补充场景约束、修正歧义。
2. **确认与执行**：向用户同步“优化后需求+执行方案”，确认后启动执行。
3. **记录更新**：需求与方案按分类记录到 AGENTS.md，确保可追溯。

## 三、Git 提交规范
- 提交时机：完成需求处理后立即提交。
- 提交格式：`git commit -m "[YYYY-MM-DD HH:MM:SS] <需求类型>：<修改内容>"`。

## 四、编码与 PR 规范
1. **语言**：强制 TypeScript 且启用严格模式。
2. **命名规则**：组件用 PascalCase，工具函数用 camelCase。
3. **工具优先级**：使用 fd.git、rg、sg 搜索工具。

## 五、确认与执行
- **不需要确认**：纯讨论、方案编写、代码片段。
- **需要确认**：改动文件、执行命令、提交更改。

## 需求处理记录

### [2026-10-14] Issue 0009：重写声子谱路径映射与联合绘图
- **需求与授权**：用户要求新建 Issue/Spec，参考 Python qeflow/VASPKit 重写声子谱；“认可你的方案开始”确认完整 FR-001～006、SC-001～004，含安装与隔离重绘。0009 为本地 Bug 编号，线上仍由用户管理，沿用 Rust。
- **实现**：共享逐点路径/频率模型，修复用端点表行号索引全部采样点导致横坐标回零；高对称点作横轴刻度，跳段拆线/合并标签，保留虚频；联合图左谱右 DOS 真正共享频率轴，分波按结构计数聚合。--plot 同步更新 CSV，不运行 QE。
- **验证**：37 项 release 测试；真实 MgB₂ 183×9 频率与固定 Python 参考差为零，距离仅 CSV 舍入；matdyn 输入逐字节一致，PRIMCELL/计算晶胞基底核对通过；已查看新旧/Python/合成虚频图。51 已安装入口及默认绘图命令通过，图/CSV 与原生构建一致。
- **保护与证据**：不修改原计算目录、旧图或提交计算；旧版/入口备份保留。数值、图像、输入哈希和安装制品来源仅存 docs/local/0009-phonon-plot；追踪 docs/specs/0009-phonon-band-plot-rewrite/acceptance.md。

### [2026-10-14] Issue 0006：统一声子入口精简输出
- **需求与授权**：用户要求 nosplit/index/coords 输出一致精简，并使用 Issue/Spec；“继续”确认完整草案及安装授权，复用 0006。
- **实施**：准备仅报告状态与工作目录，提交仅报告状态与作业号，完成统一措辞；去掉逐文件清单、重复提示，保留错误和活动作业信息，不改计算/调度逻辑。
- **验证**：18 项 release 回归通过，三模式真实 CLI 覆盖准备/提交/失败；Slurm 边界替代，无真实计算提交。安装来源与入口验证保存在 docs/local/0006-us6-install.json，旧版保留。

### [2026-10-14] 安装 nosplit 结构参数新版命令
- **授权**：用户要求“更新新命令”，将 b83c9ca 的 nosplit 参数修复安装为默认入口。
- **执行与验证**：使用已测试的 qeflow release 制品和既有加载器打包，其他四程序原样复用；保留旧版与入口备份。51 入口帮助、已安装 nosplit -i 的隔离输入准备通过，来源哈希和机器路径仅存 docs/local/nosplit-input-install-20261014.json。
- **边界**：未提交计算；squeue 无效作业号问题尚未修复。本记录更新此前“未安装”的当时状态。

### [2026-10-14] Issue 0006：nosplit 结构输入参数修复
- **需求与授权**：用户要求使用 Issue/Spec 修复 nosplit 缺少 -i；已确认完整补充草案及复用 0006，不新建 Issue。沿用 Rust 实现。
- **实施**：添加 -i/--input 与 -p/--poscar，传入现有结构上下文；显式结构优先，缺失时报错，省略参数保留工作目录 POSCAR/当前 POSCAR/CONTCAR 查找行为。
- **验证与边界**：真实 CLI 两项回归覆盖四种拼写、结构写入、旧结构覆盖、自动查找和缺失失败；文档与验收见 0006 US5。未切换本机安装、不提交 QE 作业，不修复其他 nosplit 全流程缺口。

### [2026-10-14] epwflow 总体规划确认与本地 Epic 草稿
- **需求与授权**：用户要求参考 EPWpy 新增 Rust epwflow，使用 big-project-planer 规划并通过 Issue/Spec 实现；七项访谈后明确“确认开始”。沿用 Rust/Bash 语言例外。
- **已确认范围**：结构/赝势/配置起步、校验后复用、nosplit/index/coords、各向同性与各向异性常规模型、固定物理参数、受限 Tc 搜索、数据和图表、真实 MgB₂ 小网格对照；不包含材料收敛研究、自动 Wannier 调参、SOC/磁性/全带宽/显式库仑矩阵。
- **预算**：真实试算每作业最多 1 节点/64 MPI/2 小时，同时最多 2 个；本次累计 8 节点小时、数据 100 GiB，参考和失败重试计入。授权仅在上述上限内自动试算，达到上限保留现场并停止追加。
- **追踪与状态**：已确认总体规划 `docs/plan/epwflow-superconductivity.md`；用户已确认本地 Epic `docs/specs/0007-epwflow-superconductivity/issue-draft.md`，进入子 Feature 审查。F1 本地 0008 草稿位于 `docs/specs/0008-epwflow-foundation/issue-draft.md`，待该版确认；五个子 Feature 的详细 Spec/实现尚未开始。编号为本地需求编号，线上 Gitee Issue 仍由用户管理。保留原有未提交文件，不提交其他工作。


### [2026-10-14] 修复 MgB2 根运行目录的提交环境
- **需求与授权**：用户要求修复本地 input.toml 并测试能否启动，完整计算由用户亲自提交。
- **实施**：补齐 Slurm 单节点 64 进程资源、MPI/oneAPI 环境与线程设置，补齐 KPATH；新工作目录隔离原失败现场及配置绑定，保留显式 outdir。机器路径、输入备份与测试日志仅存忽略目录。
- **验证**：实际 split_index 无提交准备成功、生成脚本语法通过；64 进程及单进程限时资源申请均因无可用资源超时退出，无遗留队列任务。登录节点加载环境后能定位 mpirun，pw/ph/q2r/matdyn 均进入 QE 7.4.1 输入读取阶段，空输入预期报错退出；计算节点启动尚未验证，不记为成功；未提交完整声子链。


### [2026-10-14] Issue 0006：声子默认 outdir 统一为 ./tmp
- **需求与授权**：用户要求“我希望默认的 outdir为'./tmp'你帮我修改并且更新 spec，issue 和 overview”；沿用 0006 Rust/Bash 实现。
- **方案**：默认配置及 PH 初始化、分 q、合并的缺省路径统一为 ./tmp，新算例配置同步；显式目录保持优先，已有 ./out 计算文件不改写或迁移。
- **追踪与验证**：0006 FR-010 / SC-007 / T016～T018；32 项定向测试通过，安装后四类入口默认 tmp/显式 out 校验通过；五主程序已重新编译安装，51 个入口隔离核验及制品哈希通过，证据见 acceptance.md。线上 Issue 仍由用户管理，仅维护本地草稿。

### [2026-10-14] 全量重新编译并切换本机最新版
- **需求与授权**：用户明确「全部重新编译，我要新版的代码。」；重新编译五份 Rust 主程序及独立 dyn0 工具，切换默认命令。
- **执行**：使用全新的主项目/独立工具 target 目录，release/locked/offline 构建；五主程序通过兼容加载器打包，51 个入口备份后切换。独立 qf-gen-dyn0 单独打包安装，不恢复主项目集成。
- **验证**：workspace 97 项、独立工具 4 项、51 主入口及独立工具基线输出通过；源码、源二进制和打包制品哈希核对。此前真实初始化 1250893 已于当日正常结束并通过产物检查。
- **追踪**：0006 T013/T014/T015 完成；机器路径、构建日志和安装记录仅 docs/local/latest-rebuild.json 指向的本地证据目录。保留旧版本和原计算目录，线上 Issue 仍由用户管理。

### [2026-10-14] Issue 0006：dyn0 独立工具与 QE 初始化入口
- **需求与确认**：用户澄清保留自写 dyn0 小程序，但解除主项目集成；qe_ph_nosplit --dyn0 改为调用 QE 初始化并支持提交。用户确认完整方案，要求使用 Issue/Spec 执行。沿用 Rust/Bash 语言例外。
- **实现**：tools/qf-gen-dyn0 独立 Cargo 工程与锁文件，迁入算法/XML 对称解析和测试；主程序移除 gen-dyn0/别名分派，保留坐标读取与旧目录数据兼容。单步与自动链路共用 only_init 正常初始化、保存数据及输出检查；-s 提交、-r 执行、默认准备。
- **自动验收**：workspace 97 项、独立工具仓库外 4 项、隔离安装 51 入口、旧链接与独立程序迁移检查通过；迁移前后工具输出逐字节一致。已打包新制品并校验来源。
- **后续验收**：1250893 已于当日 17:17 完成，COMPLETED/0:0；dyn0 与原 QE 列表逐字节一致，无完整 PH/后处理，原始 SCF 快照哈希不变。后续全量重编译与安装见本次补充记录；真实配置和证据仅 docs/local。

### [2026-10-14] 忽略 examples 目录新增内容
- **需求与授权**：用户要求将 examples 目录加入 .gitignore 并提交 Git。
- **实施**：将仅忽略 examples 下 run 目录的规则扩展为根目录 `/examples/`；保留本地文件及已跟踪的 13 个手册、脚本和输入文件，不取消跟踪。
- **验证**：确认新增算例和运行输出命中忽略规则，现有 13 个文件仍被跟踪；仅提交忽略规则与本记录。

### [2026-10-14] 全局技能安装与 Issue 0006 声子一次提交全链路
- **需求与授权**：用户完整批准 0006 方案，随后要求安装 Big-Project-Skill 并用其实现；已确认一次 SCF、多关联作业、index/coords 两模式和真实集群验收。
- **技能**：通过 skill-installer 将上游 2ed8ea6 的六个技能安装至当前用户全局目录，20 个文件与固定上游提交一致；使用 issue-manager、spec-runner 和 overview-maintainer。用户指定本地 Issue 草稿及既有确认优先，不操作线上 Gitee。
- **实施**：SCF 完整快照与 PH only_init；各 q 独立复制并仅 PH；原生 Slurm afterok/作业记录/失败恢复；修正 coords 单位、实际 outdir 合并与后处理路径；三模式 examples 统一 Bash 提交入口。沿用 Rust/Bash 语言例外。
- **部署**：计算节点旧 glibc 与登录节点不兼容，用户级私有运行库和加载器打包解决，不改系统；五份程序与 52 入口更新。
- **验证**：workspace 96 项、最终 CLI 16 项、登录和计算节点 52 入口通过；64 MPI 两模式及最终制品 32 MPI 同配置 nosplit 对照通过，每流程 SCF 一次；频率/DOS 标准固定。实际队列额度失败、取消与部署失败如实留档，不冒充通过。
- **边界**：小网格非物理收敛；账户需有 q 点数 + 2 的提交额度；inspur/chem 模板待实际配置。线上 Issue、远程链接由用户自管；真实路径、作业日志与哈希仅存 docs/local。追踪 docs/specs/0006-qe-phonon-submit-chain/acceptance.md。

### [2026-10-14] 展示算例集中到根目录 examples
- **需求与授权**：用户要求将其他目录用于展示的 examples 集中到根目录 examples。
- **实施**：迁移 qeflow/examples 的四份资产；Si 结构与旧路径放 examples/Si/generic，合成 α²F 和配置放 examples/synthetic-a2F/generic。更新根目录索引及 qeflow 手册，统一运行输出忽略规则。
- **边界与验证**：不迁移自动测试夹具、冻结参考或第三方代码；历史记录保留当时路径。核对资产哈希、文档链接；尝试合成曲线入口发现当前 CLI 要求 QE EPC 文件，已明确标注为参考材料，旧 Si 路径格式限制也已说明；不提交 QE 作业。

### [2026-10-14] MgB2 按机器组织 QE 声子测试示例
- **需求与授权**：用户要求使用 qe-7.4.1-amd-oneAPI2024，并把测试与手册放在 examples/MgB2/sugon-amd；inspur、chem 沿用相同目录规则。
- **方案与实施**：共享 Bash 准备脚本生成机器目录下 run/ 的输入与显式 Slurm 顺序执行脚本；机器路径仅保存在 docs/local 的环境文件。sugon-amd 使用本机 AMD 配置；其他机器保留待填写模板，不臆造配置。沿用项目 Bash 计算脚本语言例外。
- **验证与边界**：核对 QE 7.4.1 四个计算程序；准备输入、检查脚本和 TOML，确保运行文件不跟踪；未提交实际计算，小网格仅验证流程。

### [2026-10-14] 本机安装 dftflow 与平台构建配置分离
- **需求与授权**：用户要求安装 dftflow，并确认“编译需要区分在 linux x86_64上的编译文件和 macOS_arm64的编译文件，这样就不用每次编译都修改同一份编译文件了。”
- **方案**：默认 Cargo 配置保持平台无关；Linux x86_64 glibc 与 macOS arm64 分别使用 `.cargo/linux-x86_64.toml`、`.cargo/macos-arm64.toml` 和独立产物目录；`scripts/build-native.sh` 自动按本机平台分派，既有 Linux musl 静态交付配置独立保留。沿用现有 Rust 项目和 Bash 构建脚本。
- **安装边界**：用户目录准备 Rust 和原生依赖，五个 Rust 程序复制到稳定版本目录，安装全部 52 个入口；调整 Bash PATH 以优先使用 Rust 入口，原终端配置先备份。机器路径、运行日志和安装证据仅放 `docs/local/`。
- **完成与验证**：Linux x86_64 glibc 五程序已安装至用户稳定版本目录并创建 52 个入口；Rust 88 项 release 离线测试、全部入口帮助检查通过。已安装 scflow 在清洁环境中通过三展宽 CSV/JSON 数值、CIF/POSCAR 和 7 份 PDF 黄金对照；新登录 shell 确认 QE/VASP/scflow 命令指向 Rust 入口。
- **依赖与验收边界**：用户级 OpenBLAS 0.3.30（静态、GENERIC、单线程）和 spglib 2.6.0（静态、PIC）已构建；本机库路径只存本地环境文件。两平台配置解析、Cargo 产物目录和脚本分派检查通过；本轮实际编译验收为 Linux glibc，macOS 与既有 musl 交付不冒充本轮实测。证据见 `docs/local/dftflow-install-acceptance-20261014.json`。
- **状态**：本机安装与平台配置分离完成；源码计算逻辑未改动，未执行第一性原理计算。

### [2026-10-14] 移除临时 Python 参考工作树
- **需求**：用户明确不再需要 dftflow-py 工作树，要求删除。
- **执行与验证**：确认无未提交改动，忽略文件均为 Python/pytest 缓存后，通过 git worktree remove 移除目录及注册信息；主工作树、python/master 分支和 python-reference-0004 标签保留。

### [2026-10-14] 分支改名与 Rust 默认主分支切换
- **需求**：用户要求“我希望把 master 分支改名为 python，然后把 rust 改名为 master，并且把改名后的 rust 设置为默认主分支。”
- **方案**：先将原 master（`7a172fe`）保全到本地及远程 python，再将原 rust 改名为 master，通过普通快进推送更新远程 master，最后移除旧远程 rust 分支名；保留本地备份引用及冻结标签。
- **文档与构建**：README 明确 master/python/冻结参考标签的用途；musl 工作流的 push 与 pull_request 分支过滤同步为 master。历史记录中的旧分支名不改写。
- **验收**：核对 python 仍指向原 master、原 rust 提交包含于新 master；Gitee 默认 HEAD 指向 master，两个本地分支分别跟踪对应远程；切换证据仅存 `docs/local/branch-rename-20261014-125405.json`。分支切换不改变计算逻辑，不重跑数值计算。

### [2026-10-14] 规范整理根目录 .gitignore
- **需求**：用户要求“重新规范的整齐的书写 gitignore”。
- **实施**：按构建依赖、Python 缓存、Rust 产物、测试输出、本机资料、参考手册、临时文件七类整理，统一中文注释和分隔线，例外规则紧随对应忽略项。
- **验证**：50 条规则集合不变，412 个路径整理前后匹配结果一致；保留测试输入和脚本白名单、未来 .codex/ 目录放行。仅排版与分类调整，不删除本地文件或改写历史。

### [2026-10-14] Issue 0005 保留本地文件并补齐忽略规则
- **需求**：用户明确“暂时不需要删除这些文件，但是保证不用 git 跟踪”，并引用缓存、临时文件忽略规则、空 .codex 及历史运行输出清单。
- **实施**：补齐用户指定的忽略规则；空 .codex 仅取消跟踪且保留本地，未来同名目录仍可纳管。105 个历史运行输出维持不跟踪，不继续改写历史。
- **验证**：105 个输出及 .codex 共 106 个本地文件 SHA-256 不变；全部未跟踪且被忽略，8 个输入/脚本/赝势继续跟踪；13 个忽略场景与 .codex/config.toml 放行检查通过。追踪见 0005 T011 与补充验收。

### [2026-10-14] Issue 0005 单文件历史清理与 rust 推送
- **需求与授权**：用户明确允许清理 rust 分支历史中的 `test_auto/MgB2/local_run/vasp_cohp_uniform/WAVECAR`，保留本地文件并重新推送，作为此前“不改写历史”约定的单文件例外。
- **实施**：先保存原分支本地备份引用，仅重写涉及目标文件及其后继的 8 个提交；其他运行输出历史、分支、标签及子模块不变。
- **验证**：新旧提交逐一对照仅目标文件有差异；当前代码树、提交总数、本地 WAVECAR SHA-256 和其他 refs 不变；rust 可达历史中目标路径和 blob 均已移除。
- **结果**：清理后 rust 已普通推送到 Gitee 并跟踪 origin/rust，备份及映射仅保留本地。追踪见 0005 Spec 的 T010 与 acceptance.md；线上 Issue 仍由用户自管。

### [2026-10-14] Issue 0005 补充收尾：VASP Linux 编译配置备份入库
- **需求与授权**：用户要求“你来帮我处理一下这个分支没有提交的代码。以 issue0005 完成”；确认唯一未提交项是 VASP Linux 编译配置备份后，补充“也一起提交到对应的 vasp 的那个仓库的远程把”。
- **方案与例外**：将 `makefile.include.linux-aocl.bak` 原样纳入 VASP 私有仓库，作为此前 `.bak` 不提交与机器路径本地化规则的单文件例外；父仓库仅更新子模块指针和 0005 文档。
- **结果与证据**：VASP 提交 `5e8bc01` 已推送私有 Gitee master，`ls-remote` 验证完整提交号一致；提交前后备份 blob 均为 `5b7876d8ccd3f68d49cfa433a8b1f0b6019e99ed`，文件未改写。
- **边界**：无计算逻辑变更，不重编译或运行 VASP；父仓库在 qeflow-rust 本地提交，线上 Issue 由用户自管。追踪见 `docs/specs/0005-git-hygiene-manual-testing/` 的补充需求、T009 和验收记录。

### [2026-10-14] 0004 实质缺口收敛与 macOS 主机上的 Linux 交付
- **需求与授权**：用户要求补齐 QE 基础命令、OpenBLAS 接入和严格数值验收、命令环境切换、五二进制收敛；继续 0004 Issue/Spec 和 MgB2 测试体系，在 macOS 完成 Linux 交付。沿用本项目 Rust 与 pytest，不采用通用 TypeScript 规则。
- **方案**：复刻冻结 Python 参考（9396b3a）的基础 QE 行为；ndarray-linalg + 系统静态 OpenBLAS 替代 Jacobi；恢复 Tc 和全部派生量容差；版本目录安装并备份旧入口；macOS Docker/OrbStack 运行 Linux amd64 构建环境，编译独立目标依赖并验证静态 ELF 和离线运行。
- **实现**：SCF/NSCF 正确配置段、目录、k 网格、数据继承、运行/提交及错误传播；补齐 eband KPATH 生成和标题跳过、SCF 关键文件复制、DOS 全树复制；修正 vasp_plot_kcohp 旧入口；Cargo 仅五个 binary target，dyn0 保留别名。机器登录信息和实际运行输出仍只存 docs/local。
- **验收**：macOS Rust 88 项、Python MgB2 API 9 项、已安装基础 CLI 4 项、52 个安装入口通过。Python/Rust 真实 MgB2 SCF 总能与 SCF/NSCF Fermi 能一致；scflow 合成三展宽的 CSV/JSON/全部 Tc 派生量、零频记录、CIF/POSCAR 与七份 PDF 检查通过。Linux 结果按 acceptance-20261014.md 回填，不沿用旧“全部完成”自述。
- **追踪**：`docs/specs/0004-dftflow-rust/{tasks.md,acceptance-20261014.md}`；本地 Issue 草稿关联 Gitee IKKGLP，线上仍由用户自管。
- **状态**：本轮缺口实现和验收完成：Linux 五个静态 ELF 已产出并 strip，88 项 release 测试、88 项断网重跑、52 个入口及 stripped scflow 数值对照通过；交付包在 `dist/`。真实远程计算节点与可选 MatterSim 不冒充本轮验收。


### [2026-10-13] Git 卫生治理与手动计算操作文档（Issue 0005）
- **需求**：处理所有未提交/误跟踪的文件：编译临时文件不入 Git；测试输出不保留；远程机器登录与软件配置（含路径）只记本地；测试输入、命令与操作步骤完整写入文档供手动计算查对。经澄清确认：local_run 166MB 运行输出 `git rm --cached` 保留本地；vasp.6.4.3 推用户私有 Gitee；q-e 推自己的 GitLab fork `madegan/q-e`（先同步上游）；input_*.toml 用占位符提交、真实路径存 gitignored 的 `docs/local/paths.toml` 由 conftest 自动注入；以 0005 新 Spec（issue-draft/spec/tasks/acceptance 全套）管理。
- **结果**：
    1. `.gitignore` 补 `target/`、`.pytest_cache/`、`docs/local/`、`*.bak` 与 local_run 黑白名单（放行 toml/sh/vasp/potcar_lib）；106 个 local_run 运行输出移出跟踪、磁盘保留，`git ls-files test_auto/MgB2/local_run` 仅剩 8 个输入/脚本类文件；`validate_scflow_isif4.py` 显式放行纳管。
    2. 机器路径占位符化：`input_qe/vasp/w90.toml`、local_run/{toml,sh}、`qeflow/examples/input.toml` 全部改 `@KEY@`（8 键）；新建 `docs/local/paths.toml`（不入 Git）与 `test_auto/MgB2/_local_paths.py` 注入模块；`conftest.py` 拷贝时自动注入，残留占位符即 `pytest.exit` 报错；`test_vaspflow.py` 三处断言改读路径表，合成测试串中性化，`test_qe_phonon.py` pyproject 路径改相对定位（SC-002 范围细化的最小测试调整）。
    3. 文档：新建 `docs/manual-testing.md`（占位符机制表/测试资产清单/pytest 命令序列/各 flow 手动计算步骤）与 `docs/local/remote-machines.md` 留档模板；`docs/qe-environment.md` 通用化；`docs/overview/qe-install-m5.md` 移入 `docs/local/` 并在总览索引留说明。
    4. 子模块：vasp.6.4.3 提交 `7a82340`（makefile.include macOS arm64 构建 + BUILD_MACOS.md，排除 .bak）推送私有 Gitee 成功（`6e06f68..7a82340`，父仓库指针已同步）；q-e 已添加 upstream `QEF/q-e` 并 fetch（upstream/master = `ee7f72e8f`，fork master `770a0b2d1` 为其祖先可 fast-forward）。
    5. 验证：pytest 收集 248 项、冒烟（vaspflow 非 run 类）48 通过，替换后输入含真实路径；6 个非 run 失败为 0005 之前已存在的环境性失败（mattersim 未安装、numpy 2.5 移除别名、rv4 ENCUT 预期 800 vs 输入 520），与本次改动无关；SC-002 全库机器路径检索仅剩冻结 golden 基线豁免项。
- **留档/补齐（2026-10-14）**：q-e 推送已完成——经用户提供的 glab token 认证，fork `madegan/q-e` master fast-forward 至官方上游（`770a0b2d1..ee7f72e8f`），`macos-memusage-fix`（含提交 `51fdfbd13`）推送成功并 ls-remote 验证；临时 PAT 用后已从钥匙串清除（并提醒用户轮换）。嵌套子模块 `external/mbd` 指针变动与 `.DS_Store` 按约定不提交、仅本地留档。
- **追踪**：Spec `docs/specs/0005-git-hygiene-manual-testing/`；手册 `docs/manual-testing.md`；关联 Issue 0004（[IKKGLP](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKKGLP)）；线上 Gitee Issue 由用户自建（issue-draft.md 已留草稿）。
- **状态**：全部完成并已提交（acceptance.md 六项 SC 全部通过）。

### [2026-10-13] 彻底完成 0004 全量迁移（P0–P5 收尾）
- **需求**：用户要求"彻底完成 0004 的所有需求"；经确认按全量 P5 收尾执行（补齐缺失 CLI、删被替换 Python、改名、清 pyproject、建 48 个旧名入口），musl 静态/离线验证如实留档为待 Linux 节点项；`qeflow-python-backup/` 随 P5 删除；48 个旧名入口采用多调用二进制 + 安装期链接。
- **结果**：
    1. 补齐 qeflow 8 个缺失命令（process_eband/process_eledos/ph_nosplit/q2r/matdyn_phband/matdyn_phdos/split_submit/check）并升级 `qe_phonon_nosplit` 为完整聚合工作流；`qf-gen-dyn0` 并入 `qeflow gen-dyn0` 并保留别名，支持 `--xml/--nq1-3/--prefix/--out-dir` 契约与 `q_points.json` 产出。
    2. 补齐 vaspflow 4 个命令（idos_computed/plot_cohp→process_cohp 对齐 Python 输出契约/plot_kcohp 全新实现/charge_diff）；`--interaction/-low/-up/--emin/--emax` 参数对齐。
    3. 新建 `crates/df-substitution`（约 3000 行）真实实现 7 个 substitution 命令：selection/structure/status(OUTCAR 启发式)/workflow/prepare(potcar+qe_pseudo 交互选号)/plot_eledos/check_phonon；split 布局改为 Python 的 `NN/` 目录 + 逐 q `run_split.sh`。
    4. 新建 `crates/qf-compare` 契约容差对照器；新增集成测试（`qeflow/tests/postprocess_golden.rs`、`df-substitution/tests`），`cargo test --workspace` 75 项全绿。
    5. 真实数据逐字节对照：qe_process_eband 三表（MgB2 真实 projwfc）、qe_process_eledos 四表、substitution structure/continue 全部产物（仅差根路径）、vasp_charge_diff。
    6. P5 切换：删除 Python `qeflow`(备份)/`scflow`/`vaspflow`/`wannier90flow`/`substitutionflow`/`public`；`wannier90flow-rust`→`wannier90flow`、`substitutionflow-rust`→`substitutionflow`；`pyproject.toml` 仅保留 `mattersim_opt`；`mattersimflow` 经 `mattersimflow/_public_compat.py` 解除 `public` 依赖（CLI 行为不变）。
    7. `scripts/install-compat.sh` 交付 48 个旧名硬链接入口（argv[0] 分派，含 `qf-gen-dyn0` 别名）；参考基线冻结为提交 `9396b3a`（标签 `python-reference-0004`）；`.gitignore` 补 `target/` 并清除误入索引的 23043 个构建产物。
- **留档**：`docs/specs/0004-dftflow-rust/{tasks.md,acceptance.md}` 已逐项勾选/标注；`docs/overview/dftflow-rust.md` 重写为最终实现总览（含待 Linux 验证命令）；README 重写。
- **scflow 完整链路补齐（同日续）**：发现此前 scflow 仅覆盖单点 a2F 路径，随即移植完整链路（dyn*.elph/elph.in_lambda 解析、q 权重三级回退、direct/interp 双通道含 Python float16 stairs 与 wlog/w2 数组别名行为、scipy simpson Cartwright 修正与 reflect 边界精确复刻、分展宽 CSV/result.json/spglib 对称化 CIF/三类 PDF via svg2pdf、summary CSV/PDF）；新增 spglib/svg2pdf 依赖（brew 安装 spglib，仓库级 `.cargo/config.toml` 提供 bindgen 头文件路径）；合成 EPC fixture 与 Python 参考逐字节对照通过并固化为 `scflow/tests/scflow_parity.rs`（修复了笛卡尔→分数坐标转换的行列约定错误与原子量 B=10.811）；修正 `.gitignore` 后重建的 `Cargo.toml` serde_json preserve_order、pytest 基线运行 212 通过/34 环境性失败/2 跳过。
- **待 Linux 节点项**（不伪造通过证据）：T004 静态 OpenBLAS 切换与 spglib-sys musl 静态链接、T005 PDF 链接验证、T030 musl strip+离线运行、计算节点 `run` 类 pytest 与 PATH。
- **边界**：线上 Gitee Issue（IKKGLP）不操作，由用户自管；分支名保持 `qeflow-rust`。
- **追踪**：[Gitee IKKGLP（编号 0004）](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKKGLP)；Spec `docs/specs/0004-dftflow-rust/`；Overview `docs/overview/dftflow-rust.md`。
- **状态**：代码迁移与本地验收完成并已提交；musl 静态交付验证待 Linux 节点。

### [2026-10-09] 将 qeflow 用 Rust 全量重写为单一静态二进制（Issue 0004）
- **需求**：用 Rust 重写现有 Python 版 `qeflow`，产出单一 musl 静态二进制，仅作编排层 spawn 外部 QE/VASP，避免 Python 启动/导入开销；在 `qeflow-rust` 分支开发。
- **方案**：以 FireWorks 状态机 + 队列适配器 + Workflow DAG 为主骨架，吸收 jobflow 类型化数据流，文件系统状态文件替代 daemon/数据库；cargo workspace 拆为 `qf-config`/`qf-crystal`/`qf-input`/`qf-io`/`qf-state`/`qf-runner`/`qf-phonon`/`qf-scflow`/`qf-cli`；复刻 `input.toml` 递归覆盖、状态机续算与 Slurm 两段提交；scflow 数学用 `ndarray`+静态 OpenBLAS 自实现，绘图用 `plotters`。
- **结果**：已完成六维度框架调研与架构大纲、cargo workspace 骨架、`qf-config`/`qf-io`/`qf-state`/`qf-runner` 与 `qe-scf→qe-nscf` + `scflow` 垂直切片，musl 静态二进制构建路径搭通；建立 Spec `docs/specs/0004-dftflow-rust/`。
- **追踪**：[Gitee IKKGLP（编号 0004）](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKKGLP)；Spec 位于 `docs/specs/0004-dftflow-rust/`；Overview 见 `docs/overview/dftflow-rust.md`。
- **状态**：进行中（plan building）；全量迁移（输入 deck / phonon split-merge / scflow 绘图 / CLI）待做，尚未替代现有 Python `qeflow`。

### [2026-10-10] 扩大 Rust 重写范围至全量 Python 包（Issue 0004 范围变更）
- **需求**：在 qeflow 单一重写基础上，把项目内剩余 Python 包（vaspflow、wannier90flow、substitutionflow、scflow、public）也完整迁移为 Rust；mattersimflow 除外。
- **决策**：① 最终仓库目录仍叫 `dftflow`（不叫 dftflow-rust），过渡期每个 flow 用 `xxx-rust/`，删 Python 后改名 `xxx/`；② 绘图用 Rust `plotters` 重绘，内容/布局等价（非像素级），CSV/DAT 数值严格 1:1 对齐，不再依赖 Python 调库；③ 保留 `test_auto/` pytest 黄金基线作对照，Rust 侧补集成测试；④ 科学库替代：scflow 对称化 CIF 用 `spglib`，COHPCAR.lobster 用 Rust 手写解析，mattersimflow 生成的 ML 脚本仍由目标机 Python 运行（登记为 Python 例外）；⑤ Gitee 线上 Issue 由用户自管，Spec 内用 Markdown 双向链接回退记录 Epic→子 Issue 拆分（P0–P5）。
- **范围排除**：`mattersimflow` 不迁移为 Rust，保留其原有 Python 实现、不删、不改。
- **追踪**：Gitee [IKKGLP（编号 0004）](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKKGLP)；总体计划 `docs/plan/dftflow-rust-full-migration.md`；详细 Spec `docs/specs/0004-dftflow-rust/`。
- **状态**：当时的 plan building 记录；目录切换时机、数值容差与 MatterSim 边界已由下条「复核并修正」记录更新，尚未进入分阶段实现。

### [2026-10-10] 复核并修正 Rust 全量迁移文档（Issue 0004）
- **需求**：客观审查并完善总体计划、0004 Spec 全套与本地 Issue 草稿；本次只更新文档并提交，不执行 Rust 迁移或操作线上 Gitee Issue。
- **已查明问题**：P0 将 `qeflow-rust/` 改成已存在的 `qeflow/` 会冲突；Python `substitutionflow` 仍导入 `qeflow.common`，Wannier/substitution/MatterSim 仍导入 `public`；`pyproject.toml` 实为 48 个待迁移 CLI 加 1 个保留的 `mattersim_opt`；`test_auto/MgB2/` 的 11 个 pytest 脚本被 `.gitignore` 忽略，原文引用的 `validate_scflow_isif4.py` 不存在；现有 Rust `qf-gen-dyn0` 会形成第六个可执行文件。
- **用户澄清**：0004 按 Epic 加 P0–P5 子 Issue 管理，线上创建与编号由用户负责；允许仅为移除 `public` 依赖对 `mattersimflow` 作等价调整，保留 Python CLI 及 substitution 可选 ML 链路；浮点按预定容差验收；`scflow` 仍须在弱机完整 Rust 化，Eliashberg 本征值选择静态 OpenBLAS；性能只记录，不设通过门槛。
- **修订方案**：P0 冻结可跟踪的 pytest 基线并验证 musl 静态 OpenBLAS/spglib、PNG/JPG/PDF 与 CIF；P0–P4 让 Python 与 Rust 并存，P5 反向依赖清零后集中删除被替换 Python、改名并交付五个家族二进制与 48 个旧命令兼容入口；`pyproject.toml` 最终保留 `mattersim_opt` 的最小 Python 安装内容。原始用户需求逐字记录不改写，当前 0004 Spec 仅为跨阶段过渡资料，P1–P4 待真实子 Issue 编号后建立正式 Feature Spec。
- **追踪**：总体计划 `docs/plan/dftflow-rust-full-migration.md`；跨阶段 Spec 与子 Issue 草稿 `docs/specs/0004-dftflow-rust/`。线上 Issue 与双向链接尚待用户建立，不记为完成。
- **状态**：仅文档修订；Rust 源码、pytest 文件纳管、目录迁移和线上 Issue 均待用户在另一台机器启动后实施。

### [2026-10-12] 修正 0004 命名以对齐全量迁移范围（Issue 0004）
- **需求**：用户指出 0004 的 issue/spec 名 `qeflow-rust` 与实际要做的事不符——目标是把整个 dftflow 变成 Rust，而非单独 qeflow；要求更名。
- **变更**：`docs/specs/0004-qeflow-rust/` → `docs/specs/0004-dftflow-rust/`；`docs/qeflow-rust/` → `docs/dftflow-rust/`；`docs/overview/qeflow-rust.md` → `docs/overview/dftflow-rust.md`；AGENTS/plan/overview/spec 内全部路径引用同步更新；`issue-draft.md` 头部补记命名变更说明。历史条目中的 `qeflow-rust/` 旧目录与 `qeflow-rust` 分支名为当时事实，保持不改写。
- **状态**：命名修正完成；线上 Gitee Issue 标题未改动，由用户自行决定是否同步。

### [2026-10-12] phase0-restructure：musl CI 修正与 Issue 0004 状态落档
- **需求**：重排 Rust workspace（包名改 dftflow、多二进制骨架、引入 spglib/plotters 与 musl CI），并用 `big-project-issue-manager` 更新 Issue 0004。
- **澄清**：用户确认「包名改 dftflow」为**仅文档统称**——不改任何包名、目录名与分支名，交付物家族在文档与沟通层面统称 dftflow；本次只补 musl.yml 与 Issue 状态。
- **变更**：`.github/workflows/musl.yml` strip 列表由 qeflow/vaspflow 补齐为五二进制（+scflow/wannier90/substitution），`ls -lh` 清单同步；`issue-draft.md` 新增「原始需求（用户原话）」小节（逐字留档）、Epic 段补交付物统称说明、P0 段补现状（T001–T003 完成、T004/T005 待 Linux 节点）。
- **边界**：线上 Gitee（IKKGLP）不操作，由用户自管；分支名保持 `qeflow-rust`。
- **状态**：P0 剩余 T004/T005 待 Linux x86_64 节点验证 musl 静态样例与 spglib/PDF 可行性。

### [2026-09-23] ISIF4 的 scflow 完整真实数据验收（Issue 0001）
- **需求**：直接用 SiP2-19p2/vasp_opt_ISIF4/Tc-236 完成 scflow 全部展宽的真实数据验收；沿用 Python。
- **方案**：原生 QE 文件发现、精确零频且 λ=0 的零贡献规则与原始 γ 记录、实例状态隔离、Tc float64 运算及 Eliashberg 下限状态；在独立目录运行 μ*=0.10/0.16。
- **结果**：20 份 CLI 结果、49 个输入哈希、独立公式与全部输出校验通过，24 项回归通过；μ*=0.16 有 6 个 Eliashberg 点触及搜索下限，不能视为有效物理 Tc。
- **追踪**：现有 Spec 的 SC-005/SC-006；报告位于数据项目 `docs/scflow-isif4-acceptance-2026-09-23/`。其他相问题保留，Issue 不关闭。


### [2026-09-23] SiP2 的 scflow / A2F / QE 真实归档对照（Issue 0001）
- **需求**：对 SiP2 各相的已有 QE EPC 输出使用当前 scflow、原版 A2F 与 QE 后处理代码比较 Tc。
- **方案**：冻结程序版本，在独立报告目录复制并哈希验证输入，统一 μ*=0.10；保留归档 μ* 对照，区分原样执行与零频/q 权重诊断。
- **结果**：10 组 × 10 个展宽；成功的 102 组核心配对数值相同。暴露零频、结构字段、q 点舍入和 Tc 下限等问题，SC-004 部分失败，Issue 保持开放。
- **记录**：规格验收见 `docs/specs/0001-scflow-qe-superconductivity/acceptance.md`；数据项目报告位于 `docs/scflow-comparison-2026-09-23/`。
- **边界**：本次只提交已完成的对照报告/验收文档；没有修改程序源码、原始计算数据或全局环境，没有重跑第一性原理计算。


### [2026-09-23] 将 A2F QE 超导后处理迁移为 scflow（Issue 0001）
- **需求**：把 `external/a2f/src` 的全部计算代码复制到 dftflow 内部，命名为 `scflow`，通过 CLI 读取 `input.toml` 和 QE EPC 输出并计算超导转变温度。
- **方案**：新增 `scflow/` 包、`scflow -c/--config` 入口、`[scflow]` 配置段和 `scflow` 可选依赖组；保留 A2F 完整输出，运行时不导入 `external/a2f`。
- **兼容性**：适配当前 SciPy 的 `simpson`、pymatgen CIF 文本写出和 NumPy 标量 JSON 序列化；不改物理公式和 QE 输入输出。
- **追踪**：[Gitee IKHZT0（编号 0001）](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKHZT0)；Spec 位于 `docs/specs/0001-scflow-qe-superconductivity/`；真实 QE 归档路径待补。
- **状态**：公开 `La2H20` QE EPC 归档已通过 scflow CLI 回归；真实材料 Tc 验收未执行。

### [2026-09-22] matdyn 声子谱晶体坐标声明（Issue 0002）
- **需求与确认**：用户同意在现有 Python 声子谱输入生成器中明确声明晶体坐标，增加非立方晶胞回归并更新说明；本修复沿用 Python，不采用通用 TypeScript 规则。
- **方案**：仅在 `qeflow/input_deck/matdyn.py` 增加 `q_in_cryst_coord = .true.`；不新增 CLI/配置开关，不改绘图层、DOS 输入、计算数据或工作流结构。
- **追踪**：[Gitee IKHSA0（编号 0002）](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKHSA0)；规格 `docs/specs/0002-matdyn-crystal-coordinates/`。
- **验收边界**：非立方路径与原有声子测试本地验证；历史结果核查、ISIF2/ISIF3 实际后处理和外部 QE 环境另行验收；不处理未完成的 SC-07。

### [2026-09-22] 声子谱与态密度联合 JPG（Issue 0003）
- **需求**：沿用 `qe_process_phonon`，同时绘图时增加左声子谱、右总 DOS / 元素分波、共享频率轴的 JPG；保留旧 PNG/CSV。
- **方案**：仅扩展 `qeflow/phonon/phonon_plot.py`，提取轴绘制 helper；联合范围覆盖全部频率并保留虚频，不改变计算、读取及旧状态判定。
- **语言例外**：用户已明确确认本功能沿用现有 Python，不适用本文通用 TypeScript 规则。
- **追踪**：[Gitee IKHQNI（编号 0003）](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKHQNI)；规格位于 `docs/specs/0003-phonon-band-dos-combined/`。Gitee 类型标签使用合法的 `type-feature`。
- **状态**：实现与本地测试完成；真实归档验收和交付记录见该 Feature 的验收文档。

### [2026-09-21] 全局安装 Big-Project-Skill 技能集合
- **需求**：将 `JLU-ICCMS-MaYuan/Big-Project-Skill` 安装到当前用户的全局技能目录，供 DFTFlow 和其他项目使用。
- **方案**：
    1. 从上游提交 `3377d2735d38984b8a27dd317a5665c4f5f07c32` 安装到 `~/.codex/skills/`。
    2. 新增 `big-project-planer`、`big-project-contractor`、`big-project-spec-runner`、`big-project-translator`，更新 `big-project-issue-manager` 与 `big-project-overview-maintainer`。
    3. 更新前将两个同名技能备份到 `~/.codex/skill-backups/big-project-skill-20260921-223323/`，保留已有 `big-project-spec`。
    4. 校验全部 20 个技能文件与上游内容一致、技能入口及其相对链接完整；安装文件位于项目仓库之外，本仓库只记录安装结果。
- **状态**：已完成；安装范围为当前用户跨项目使用，不涉及其他系统用户。

### [2026-06-03] vasp_process_eledos d 轨道分量命名收敛为 dx2-y2
- **需求**：
    1. `vasp_process_eledos -e "Yb:s,p,d"` 处理 VASPKit 生成的 `PDOS_Yb.dat` 时报 `缺少轨道分量列: dx2`。
    2. 真实 `PDOS_*.dat` 表头中第五个 d 分量为 `dx2-y2`，脚本不需要兼容 `dx2` 等旧写法。
    3. `vasp_process_eledos`、复用轨道解析的 `vasp_process_eband` 和文档中的精细轨道命名应统一只保留 `dx2-y2`。
- **方案**：
    1. 将 `D_COMPONENTS` 与 `ORBITAL_GROUPS` 的第五个 d 分量从 `dx2` 改为 `dx2-y2`。
    2. 调整 PDOS 表头归一化逻辑，保留连字符，使 `dx2-y2` 表头可被直接识别。
    3. 同步 `vasp_process_eband` 的 PBAND 分量键、自动化测试和 `vaspflow` README。
- **状态**：已完成。

### [2026-06-03] vasp_opt rv4 四步 INCAR 模板更新
- **需求**：
    1. `vasp_opt` 的 `rv4` 四步优化参数按新的 `INCAR_1` 到 `INCAR_4` 模板写出。
    2. 第 3/4 步固定使用 `NCORE = 1`，避免高精度优化阶段触发 VASP Davidson 并行数值不稳定。
    3. 模板中的 `PSTRESS = 500.0` 不固定采纳，继续沿用配置或 `-p` 压强覆盖逻辑动态写入。
- **方案**：
    1. 更新 `vaspflow/templates.py` 中 `opt_stage_templates` 的四步默认参数，新增 `stage4`。
    2. 修改 `vaspflow/opt/vasp_opt.py`，让 `rv4` 的 `INCAR_4` 也从 `stage4` 生成，`rv1/rvf` 继续使用 `[vasp_opt]` 参数。
    3. 更新 `vasp_opt` 自动化测试和 `vaspflow` README，锁定新四步模板、动态 `PSTRESS` 与第 3/4 步 `NCORE = 1`。
- **状态**：已完成。

### [2026-06-03] vasp_opt rv4 使用 VASP 6.4.3 MPI 完整命令
- **需求**：
    1. `vasp_opt` rv4 分步脚本中的 VASP 运行命令应为 `mpirun -np 48 /work/home/mayuan/software/vasp.6.4.3/bin/vasp_std > vasp.log_$i 2>&1`。
    2. 保持 `CONTCAR_i`、`OUTCAR_i` 和 `POSCAR` 分步归档语义不变。
- **方案**：
    1. 确认 `vaspflow` 的 `[vasp].executable_path` 语义是完整执行命令，`vasp_opt` 会原样写入 rv4 脚本。
    2. 更新测试配置中的 `[vasp].executable_path` 为 VASP 6.4.3 MPI 完整命令。
    3. 更新 `vasp_opt` rv4 与多压强总脚本断言，锁定完整 MPI 命令。
- **状态**：已完成。

### [2026-06-02] vasp_opt rv4 分步日志与归档强化
- **需求**：
    1. 排查 `substitutionflow` 链路中 `vasp_opt` 优化后看不到 `CONTCAR_n`、`OUTCAR_n` 的原因。
    2. `vasp_opt` 的 rv4 分步优化应按 `INCAR_n -> INCAR`、运行 VASP、归档 `CONTCAR/OUTCAR`、再用 `CONTCAR` 更新 `POSCAR` 的顺序循环。
    3. 每一轮 VASP 运行需要重定向输出到 `vasp.log_n`，便于定位具体优化阶段的问题。
- **方案**：
    1. 确认 `substitutionflow` 通过 `vasp_opt -r` 间接调用 `vasp_opt`，最终仍复用 `vaspflow/opt/vasp_opt.py` 的 rv4 命令序列。
    2. 修改 rv4 命令序列为 `vasp_std > vasp.log_$i 2>&1` 风格，并对 `INCAR/CONTCAR_n/OUTCAR_n/POSCAR` 拷贝使用 `cp -f` 明确覆盖。
    3. 补充 `vasp_opt` 测试断言，锁定 `fouropt.sh` 中的分步日志和归档命令。
- **状态**：已完成。

### [2026-05-30] vasp_eledos KPOINTS 按 kmesh 比例校准
- **需求**：
    1. `vasp_eledos` 仍优先使用 `[vasp_eledos].kmesh` 调用 `vaspkit` 生成 `KPOINTS`。
    2. 生成后自动读取 `vasp_scf/KPOINTS`，按 `vasp_scf.kmesh / vasp_eledos.kmesh` 推导整数网格倍数，检查 DOS 网格是否逐轴满足该倍数。
    3. 若 `vaspkit` 生成的网格不满足整数倍关系，例如 `12 12 6 -> 24 24 13`，自动修正为可整除网格，例如 `24 24 12`。
    4. 保持 `wavecar_path` 复制和 `ISTART = 1` 语义不变。
- **方案**：
    1. 新增 `vaspflow/kpoints.py`，集中解析、写出 automatic mesh KPOINTS，并按 kmesh 密度比推导整数倍。
    2. 修改 `vaspflow/eledos/vasp_eledos.py`，在 `vaspkit 102 -> 2 -> kmesh` 后校准 `vasp_eledos/KPOINTS` 到 SCF 网格整数倍。
    3. 对非整数 kmesh 密度比采用向上取整，保证最终 DOS 网格逐轴都是 SCF 网格的整数倍。
    4. 补充自动化测试覆盖 `24 24 13 -> 24 24 12` 修正和非整数密度比向上取整。
- **状态**：已完成。

### [2026-05-29] 新增 wannier90 与 VASP 6.4.3 外部子模块
- **需求**：
    1. 将 `external/wannier90` 作为 dftflow 的 Git submodule 管理。
    2. 将本地 `external/vasp.6.4.3` 推送到私有 Gitee 仓库，并作为 dftflow 的 Git submodule 管理。
    3. 避免把 VASP 编译产物作为源码提交到私有仓库。
- **方案**：
    1. 在 `external/vasp.6.4.3` 初始化独立 Git 仓库，新增子仓库 `.gitignore` 忽略 `build/`、`bin/`、`bin-omp/` 与常见编译产物，提交源码并推送到 `git@gitee.com:mayuan_JLUPHY/vasp.6.4.3.git`。
    2. 在父仓库 `.gitmodules` 中登记 `external/vasp.6.4.3`，指向该私有 Gitee 仓库。
    3. 在父仓库 `.gitmodules` 中登记 `external/wannier90`，指向官方 `git@github.com:wannier-developers/wannier90.git`，并记录 master 当前提交。
    4. 原本损坏的 `external/wannier90` 工作树已保留为本地备份目录 `external/wannier90.local-backup-20260529-1650`。
- **状态**：已完成。

### [2026-05-29] 新增 vaspflow 架构 ADR 记录
- **需求**：
    1. 通读 `vaspflow` 源码，梳理 VASP 运行型 CLI、纯后处理 CLI 和公共基础设施边界。
    2. 新增 vaspflow ADR 文档，沉淀当前模块职责、依赖方向和后续扩展约束。
    3. 保持 ADR 顶层索引与 vaspflow README 可互相定位。
- **方案**：
    1. 使用 `explore` skill 的本地 fallback 流程巡检 `vaspflow/`、`public/`、`pyproject.toml` 与相关测试。
    2. 新增 `docs/adr/vaspflow/README.md` 与 `docs/adr/vaspflow/0001-architecture-boundaries.md`，记录 setup、postprocess、templates、job/config/executable helpers 的职责边界。
    3. 更新 `docs/adr/README.md` 与 `vaspflow/README.md`，补充 vaspflow ADR 入口。
- **状态**：已完成。

### [2026-05-29] vaspkit 调用输出改为静默模式
- **需求**：
    1. qeflow 和 vaspflow 中通过 shell 子进程调用 `vaspkit` 时，不再把 `vaspkit` 菜单、banner 或运行输出打印到屏幕。
    2. 保持现有 `vaspkit` 菜单输入序列、生成文件和工作流提示不变。
    3. 失败时仍保留明确错误状态，不因静默输出改变原有生成逻辑。
- **方案**：
    1. 修改 `qeflow/input_deck/kpath.py`，让 `vaspkit 3 -> 303` 调用将 stdout/stderr 重定向到 `subprocess.DEVNULL`。
    2. 修改 vaspflow 中 SCF/ELEDOS/EBAND/ELF/LOCPOT/FERMI/CHARGE/COHP 相关 `vaspkit` 调用，统一屏蔽 stdout/stderr。
    3. 对纯后处理的 `vasp_process_eband` 与 `vasp_process_eledos` 保留退出码检查，失败时只报告退出码，不回显 `vaspkit` 原始输出。
- **状态**：已完成。

### [2026-05-28] 新增 qeflow 架构 ADR 记录
- **需求**：
    1. 新增 `docs/adr/`，用于记录架构决策，并按 `qeflow`、`vaspflow` 等子系统分区。
    2. 将 qeflow 当前模块边界、依赖方向和后续扩展约束沉淀为可追溯文档。
    3. 保持 README 与 ADR 入口可互相定位。
- **方案**：
    1. 新增 `docs/adr/README.md`，定义 ADR 编号、状态、子系统分区和使用规则。
    2. 新增 `docs/adr/0000-template.md`，作为后续架构决策记录模板。
    3. 新增 `docs/adr/qeflow/README.md` 与 `docs/adr/qeflow/0001-architecture-boundaries.md`，记录 `setup/crystal/input_deck/io_reader/phonon/check` 的职责边界和依赖方向。
    4. 更新 `qeflow/README.md`，补充 qeflow 分区 ADR 入口。
- **状态**：已完成。

### [2026-05-28] 去除 qeflow 内部 QE 模块目录前缀
- **需求**：
    1. 将 `qeflow/qe_check` 改名为 `qeflow/check`。
    2. 将 `qeflow/qe_input_deck` 改名为 `qeflow/input_deck`。
    3. 将 `qeflow/qe_io_reader` 改名为 `qeflow/io_reader`。
    4. 将 `qeflow/qe_setup` 改名为 `qeflow/setup`。
    5. 保持用户命令名兼容，例如 `qe_check` console script 不改名。
- **方案**：
    1. 直接移动四个目录，并全仓库更新 Python import、测试 monkeypatch、`pyproject.toml` console script 模块路径。
    2. 更新 qeflow README 与 Mermaid 流程图索引，流程图文件名同步收敛为 `setup-flow.mmd` 与 `input-deck-flow.mmd`。
    3. 运行聚焦 qeflow 测试，确认目录重命名不影响现有 CLI 行为。
- **状态**：已完成。

### [2026-05-27] 删除 qeflow qe_input 并重建 QE 输入边界
- **需求**：
    1. `qeflow/qe_input` 名称和职责过于混杂，需要彻底删除，不保留兼容 facade。
    2. 新边界需要清晰区分 QE Calculation Setup 生命周期、QE 原生 input deck 写出、晶体结构模型与声子 workspace。
    3. `scf.in` 中三种晶格写法相关的晶格参数推导应进入 `qeflow/crystal`，QE 语法写出留在 input deck writer。
- **方案**：
    1. 新增 `qeflow/setup/`，迁移 `calculation_setup.py` 与 `cli_adapter.py`，仅负责计算目录生命周期编排。
    2. 新增 `qeflow/input_deck/`，按 `pw.py`、`ph.py`、`q2r.py`、`matdyn.py`、`bands.py`、`dos.py`、`lattice.py`、`scripts.py` 等拆分 QE 输入 deck 写出职责。
    3. 新增 `qeflow/crystal/`，承载 POSCAR 解析、化学式、kmesh 与 `cell2ibrav` 晶格参数推导；`input_deck/lattice.py` 只负责 QE 语法序列化。
    4. 将旧 `qe_input/fileio.py` 拆为 `qeflow/phonon/workspace.py` 与 `qeflow/io_utils.py`，同步更新生产代码、测试、README、CONTEXT 和 Mermaid 流程图。
- **状态**：已完成。

### [2026-05-27] 删除 qeflow 旧 facade 并新增 QE IO Reader
- **需求**：
    1. 删除 qeflow 中旧版本遗留的 Python facade 和旧导入接口，内部全部使用解耦后的最新模块。
    2. 保留现有 CLI 命令名兼容，不破坏已有脚本入口。
    3. 新增 `io_reader` 模块，集中读取 QE 原生输入输出文件与 QE 后处理生成文件，包括 `pw.x` 的 `scf.out/nscf.out/eband.out`。
- **方案**：
    1. 新增 `qeflow/io_reader/`，按 `common.py`、`pw.py`、`ph.py`、`bands.py`、`dos.py`、`matdyn.py` 拆分读取职责。
    2. 删除 `qeflow/phonon/_shared.py`、`qeflow/qe_input/builders.py` 与 `qeflow/check/outputs.py` 旧导入层，测试和生产代码改为直接依赖真实模块或 reader。
    3. 将 `dyn0`、`qe_process_eband`、`qe_process_dos`、`phonon_tables`、`qe_check.checks` 的读取/解析路径收敛到 `io_reader`，并删除 `qeflow/phonon/dyn0.py` 旧读取入口。
    4. 保留 `qe_phonon_nosplit`、`qe_ph_nosplit` 等 CLI 命令名兼容，更新 qeflow README 中旧 facade 描述。
- **状态**：已完成。

### [2026-05-27] 深化 qeflow 架构并解耦 phonon 公共模块
- **需求**：
    1. 按架构巡检候选 1-5 继续重构 qeflow，改善函数名称、模块名称、复用边界和耦合情况。
    2. 重点迁移并解耦 `qeflow/phonon` 中公共的 `_shared.py`、后处理、merge、split 和输入写出逻辑。
    3. 保持现有 CLI 名称、参数和 `workflow_status.json` 行为兼容。
- **方案**：
    1. 将 `_shared.py` 收敛为兼容 facade，新增 `context.py`、`dyn0.py`、`q_index_map.py`、`runner.py`、`root_steps.py`、`script_plan.py` 承载真实职责。
    2. 将 `process_phonon.py` 拆为 `postprocess_workflow.py`、`archive.py`、`phonon_tables.py`、`phonon_plot.py`，保留旧函数 re-export 和 CLI。
    3. 将 `qe_merge.py` 的 split merge 策略迁移到 `split_merge.py`，CLI Adapter 只负责输出与状态刷新。
    4. 新增 `qeflow/qe_input/phonon_input.py` 作为 **QE Phonon Input Deck** Module，`builders.py` 仅保留兼容导出。
    5. 新增 `qeflow/qe_input/cli_adapter.py`，统一 `qe_scf/qe_nscf/qe_eband/qe_eledos` 的 CLI Adapter。
- **状态**：已完成。

### [2026-05-27] 重构 QE Calculation Setup
- **需求**：
    1. 先从 qeflow 架构候选 1 开始，收敛 `qe_scf/qe_nscf/qe_eband/qe_eledos` 中重复的 setup 生命周期。
    2. 新 Module 使用对象式 Interface，不保留旧 `QESetup/QENSCFSetup/QEEBandSetup/QEEleDosSetup` 类名。
    3. CLI 命令名和 `-i/-c/-r/-s` 行为保持不变。
    4. 第一阶段不纳入 phonon SCF、不拆 `templates.py`、不重构 split workflow 状态机。
- **方案**：
    1. 在 `CONTEXT.md` 中新增 **QE Calculation Setup** 领域词，作为命名锚点。
    2. 新增 `qeflow/qe_input/structure.py`，集中维护 POSCAR 解析、化学式和 kmesh 派生。
    3. 新增 `qeflow/qe_input/calculation_setup.py`，用 `QECalculationSetup` + `QECalculationSpec` 注册表统一 SCF/NSCF/EBAND/ELEDOS 的配置加载、结构准备、依赖准备、输入写出、运行脚本和执行命令。
    4. 将 `qe_scf.py`、`qe_nscf.py`、`qe_eband.py`、`qe_eledos.py` 收敛为薄 CLI Adapter，测试改为直接验证 `create_qe_calculation_setup()`。
    5. 把 KPATH 解析从 `qe_eband.py` 移到 `qeflow/qe_input/kpath.py`，避免公共工具反向依赖 CLI 模块。
- **状态**：已完成。

### [2026-05-27] 为 QE 输入增加自动 ibrav 晶格写出模式
- **需求**：
    1. `qe_scf`、`qe_nscf`、`qe_eband` 和 `qe_phonon` 的 `pw.x` 输入需要支持三种晶格写法：自动 `ibrav + A/B/C/cos*`、自动 `ibrav + celldm(n)`、旧版 `ibrav = 0 + CELL_PARAMETERS {angstrom}`。
    2. 默认使用 `ibrav_angstrom`，并通过 `[qe].lattice_mode` 选择模式。
    3. 不允许用户手动指定 `ibrav`、`A/B/C`、`cosAB/cosAC/cosBC` 或 `celldm(1..6)`，避免和自动判断结果冲突。
    4. 自动判断优先调用 QE 官方 `cell2ibrav.x`；不可用时使用 Python fallback，便于和 QE 结果对照。
- **方案**：
    1. 新增 `qeflow/qe_input/lattice.py`，集中实现 lattice mode 校验、`cell2ibrav.x` 调用、QE `latgen.f90` 风格 Python fallback 和 QE 输入写出 helper。
    2. 修改 `qeflow/templates.py`，在 `[qe]` 模板增加 `lattice_mode = "ibrav_angstrom"`，并移除各 `SYSTEM` 模板中的默认 `ibrav = 0`。
    3. 修改 `qe_scf/qe_nscf/qe_eband/qe_phonon` 输入生成路径，统一复用公共晶格写出逻辑；`cell_parameters` 模式保持旧输出。
    4. 更新 README、qeflow README、Mermaid 流程图与自动化测试，覆盖三种模式、手动参数报错、phonon 默认输出和 fallback 对照测试。
- **状态**：已完成。

### [2026-05-27] 新增 substitution_check_phonon_stability 周期表式声子稳定性检查
- **需求**：
    1. 新增 CLI `substitution_check_phonon_stability`，用于批量检查 substitutionflow 声子计算体系是否稳定。
    2. 展示方式复用周期表式布局，按 XA 目录和 pressure 分别输出 `phonon_stability_summary_<pressure>.png`。
    3. 只输出 PNG，不生成 `.dat` 或其它汇总文件。
    4. 稳定性状态分为 `stable / unstable / unfinished`；读取 `phdos/phonon_dos_elements.csv` 的频率网格，最小频率换算为 THz 后若 `< -1.0 THz` 判为 `unstable`，否则判为 `stable`；缺失或无法解析判为 `unfinished`。
- **方案**：
    1. 新增 `substitutionflow/check_phonon_stability.py`，复用 `substitution_plot_eledos` 的 XA 目录发现、pressure 发现和周期表布局逻辑。
    2. 通过 `substitutionflow.status.get_case_phonon_workdir()` 定位每个 case 的 QE 声子工作目录，兼容 `phonon_mode` 默认目录与显式 `[qe_phonon].work_dir`。
    3. 在图中用绿色、红色、灰色分别展示 `stable / unstable / unfinished`，并显示元素符号、状态和最小频率 THz 数值。
    4. 注册 `pyproject.toml` console script，更新 README、Mermaid 流程图和自动化测试。
- **状态**：已完成。

### [2026-05-27] 为 substitution_check_phonon_stability 增加并行读取
- **需求**：
    1. `substitution_check_phonon_stability` 在检查大量 case 的声子稳定性时需要提高效率。
    2. 并行范围应覆盖 case 结果读取与稳定性判定，不应并行调用 matplotlib 绘图。
- **方案**：
    1. 在 `substitutionflow/check_phonon_stability.py` 中新增 `-j/--jobs` 参数，默认按 CPU 数自动选择线程数。
    2. 使用 `ThreadPoolExecutor` 并行读取同一 XA 目录、同一 pressure 下各 case 的 `phonon_dos_elements.csv`，并保持输出格子顺序稳定。
    3. 绘图继续按 XA 目录与 pressure 顺序串行执行，避免 matplotlib 全局状态并发风险。
    4. 更新 README 与自动化测试，覆盖显式 `-j` 参数和并行读取路径。
- **状态**：已完成。

### [2026-05-20] 修复 qe_merge 当前目录解析并收敛 split 脚本编号
- **需求**：
    1. `qe_merge -c <config>` 在 `qe_phonon_split_index/coords` 当前目录内直接执行时，必须读取当前目录下的 `q_index_map.json`，不能误落到默认 `qe_phonon/` 目录。
    2. `qe_phonon_split_index/coords -s` 只保留两个顶层提交脚本：`submit_pre_and_split_stage.sh` 与 `submit_merge_and_process_phonon.sh`。
    3. 两个脚本的进度编号统一扩展为全局 `1/11` 到 `11/11`，pre 段覆盖 `1/11` 到 `3/11`，post 段覆盖 `4/11` 到 `11/11`。
    4. post 回收阶段直接展开 `qe_merge -> q2r -> matdyn_phband -> collect phband -> matdyn_phdos -> collect phdos -> plot`，不再依赖中间的 `submit_process_phonon.sh`。
- **方案**：
    1. 修改 `qeflow/qe_input/fileio.py` 与 `qeflow/phonon/qe_merge.py`，让 merge 在当前工作目录存在 `q_index_map.json/workflow_status.json` 时直接读取当前目录，并且以只读上下文运行。
    2. 修改 `qeflow/phonon/_shared.py`，让脚本生成器支持显式 `start_index` 与 `total_steps`，便于固定 11 步编号。
    3. 修改 `qeflow/phonon/qe_phonon_split_assign_index.py` 与 `qeflow/phonon/qe_phonon_split_assign_coords.py`，只生成 pre/post 两个脚本，并把 post 脚本展开为完整回收链。
    4. 修改 `qeflow/phonon/process_phonon.py`，让 `run_process_phonon_steps()` 只负责输入与执行，不再额外生成 `submit_process_phonon.sh`。
    5. 更新 `README.md`、`qeflow/README.md`、Mermaid 流程图与 `test_auto/MgB2/test_qe_phonon.py`，同步新脚本边界、编号和回收语义。
- **状态**：已完成。

### [2026-05-20] 收敛 qe_phonon -s 为 Slurm 内完整执行链
- **需求**：
    1. `qe_phonon_nosplit -r/-s` 与 `qe_phonon_split_index/coords -r/-s` 都要完整推进到 `plot`，不再把 Python 绘图留作人工续算点。
    2. split `-s` 第一次提交后，应在 Slurm 脚本内顺序执行 `scf -> dyn0 -> split q 点提交`，把 dyn0 后的 Python 物化和提交逻辑也放入脚本。
    3. split q 点提交继续保持逐目录 `cd 01 && sbatch run_split.sh`、`cd 02 && sbatch run_split.sh` 的并行排队语义，只提交 `error/unstarted`，跳过 `finished`。
    4. q 点全部完成后的回收阶段不再在登录节点直接执行 merge，而是提交一个 Slurm 脚本顺序执行 `qe_merge -> q2r -> matdyn_phband -> collect phband -> matdyn_phdos -> collect phdos -> plot`。
    5. 每个 Slurm 脚本步骤执行前输出当前步骤，便于从 `slurm-*.out` 判断进度。
- **方案**：
    1. 新增内部编排 CLI `qe_split_submit`，通过 `--mode index|coords` 复用 split 目录物化逻辑，并逐个进入 q 点目录提交 `run_split.sh`。
    2. 修改 `qe_process_phonon -s` 生成的 `submit_process_phonon.sh`，把 `collect phband/phdos` 与 `plot` 都写入脚本，并支持 `--collect phband|phdos`。
    3. 修改 `qe_phonon_nosplit` 的 `-r/-s`，使本地执行和 Slurm 提交都覆盖 `q2r/matdyn/collect/plot` 全链路。
    4. 修改 `qe_phonon_split_index` 与 `qe_phonon_split_coords`，将 `-s` 收敛为 `submit_pre_and_split_stage.sh` 与 `submit_merge_and_process_phonon.sh` 两段脚本，merge 通过 `qe_merge` CLI 在 Slurm 内执行。
    5. 更新 README、Mermaid 流程图和 `test_auto/MgB2/test_qe_phonon.py`，覆盖新脚本顺序、CLI 注册、自动绘图和 split 提交语义。
- **状态**：已完成。

### [2026-05-20] 新增 qe_process_phonon 统一声子后处理编排
- **需求**：
    1. 新增独立 CLI `qe_process_phonon`，统一编排 QE 声子后处理 `q2r / matdyn_phband / matdyn_phdos / plot`。
    2. `-r` 本地顺序执行 `q2r -> phband -> collect phband -> phdos -> collect phdos -> plot`；`-s` 只提交 `q2r/matdyn/collect` 脚本，不在 shell 中调用 Python 绘图。
    3. 计算完成后立即按步骤归档，`phband` 结果进入 `phband/`，`phdos` 结果进入 `phdos/`，避免后一步 matdyn 输出覆盖前一步可绘图数据。
    4. 绘图直接由 Python 实现，输出声子谱与声子 DOS 的 PNG 和 CSV；声子 DOS CSV 同时保存原子级与元素级数据，绘图只绘制 `TDOS + 元素级曲线`。
    5. 状态继续复用 `workflow_status.json`，不新增独立状态 JSON；`matdyn_phband/matdyn_phdos` 要求计算输出和归档文件同时存在才算 `finished`，新增 `phonon_plot` 标记绘图状态。
- **方案**：
    1. 新增 `qeflow/phonon/process_phonon.py`，复用现有 `qe_q2r`、`qe_matdyn_phband`、`qe_matdyn_phdos` 的输入与脚本生成逻辑，并集中维护归档、绘图和 CSV 写出逻辑。
    2. 修改 `qe_matdyn_phband` 与 `qe_matdyn_phdos`，使单步 `-r/-s` 在 matdyn 完成后复用统一归档逻辑。
    3. 修改 `qe_phonon_nosplit`、`qe_phonon_split_index`、`qe_phonon_split_coords`，后处理阶段统一调用或提交 `submit_process_phonon.sh`。
    4. 修改 `qeflow/check/checks.py` 与 `qeflow/phonon/_shared.py`，把归档文件和 `phonon_plot` 纳入 `workflow_status.json` 刷新与打印。
    5. 注册 `qe_process_phonon` console script，更新 README 与 Mermaid 流程图，并补充自动化测试覆盖脚本顺序、归档、CSV/PNG 和状态刷新。
- **状态**：已完成。

### [2026-05-19] 收敛 qe_phonon split merge 为程序内直接执行
- **需求**：
    1. `qe_phonon_split_index -s` 与 `qe_phonon_split_coords -s` 在 split q 点全部完成后，`merge` 步骤直接在程序内部执行，不再生成或提交 `run_merge.sh` / `submit_merge_stage.sh`。
    2. `merge` 成功后，同一次 `-s` 调用继续生成并提交 `submit_post_stage.sh`；`merge` 失败时返回非零并停止后续提交。
    3. `qe_merge` 独立 CLI 删除 `-r/-s` 执行方式差异，调用即直接执行 merge，保留可选 `-c/-i` 上下文参数。
    4. 不再暴露旧的 `qe_ph_split_assign_index` 与 `qe_ph_split_assign_coords` console scripts，split q 点目录物化逻辑改为内部复用。
    5. `qe_ph_nosplit.py` 重命名为 `ph_step.py`，但用户命令名 `qe_ph_nosplit` 保持不变。
- **方案**：
    1. 新增 `qeflow/phonon/split_prepare.py`，集中维护 `prepare_split_assign_index_step()` 与 `prepare_split_assign_coords_step()`，总工作流从该内部模块复用 split q 点物化逻辑。
    2. 修改 `qeflow/phonon/qe_merge.py`，删除 `prepare_merge_step()`、`run_merge.sh` 写出与 `submit_script()` 分支，将 `run_merge_step()` 收敛为直接执行并按成功/失败返回状态码。
    3. 将 `qeflow/phonon/qe_ph_nosplit.py` 重命名为 `qeflow/phonon/ph_step.py`，并同步更新 `pyproject.toml` 与内部 import。
    4. 修改 `qeflow/phonon/qe_phonon_split_assign_index.py` 与 `qeflow/phonon/qe_phonon_split_assign_coords.py`，在 `-s` 路径中直接执行 merge，成功后立即提交 post 阶段，失败则返回 `1`。
    5. 从 `pyproject.toml`、README 与 Mermaid 流程图中移除旧 split CLI 入口，并更新 `test_auto/MgB2/test_qe_phonon.py` 覆盖新语义。
- **状态**：已完成。

### [2026-05-13] 为 substitution_plot_eledos 增加数值统计与进度条
- **需求**：
    1. `substitution_plot_eledos` 在生成 `eledos_summary_<pressure>.png` 的同级目录，同步输出费米能级处 DOS 数值统计 `eledos_summary_<pressure>.dat`。
    2. `.dat` 使用空格分隔，列包含体系化学配比名称、XA/XB/H/TDOS 的费米能级处 DOS 和 `status`。
    3. 统计过程中显示类似 `substitution_continue` 的紧凑进度条，便于跟踪处理到的 XA 目录和压强点。
- **方案**：
    1. 修改 `substitutionflow/plot_eledos.py`，复用现有 `*_at_ef.csv` 读取结果写出 `eledos_summary_<pressure>.dat`，缺失 CSV 或缺列统一记为 `error`。
    2. 在 `substitution_plot_eledos` 主流程中按全部 XA 目录与 pressure 总数显示 `处理 eledos: [..] current/total` 进度条。
    3. 更新 `test_auto/MgB2/test_substitutionflow.py`，覆盖 `.png/.dat` 同步生成、缺失 case 保留、数值写出和进度条输出。
    4. 更新 `substitutionflow/substitutionflow-cli-flow.mmd`，同步新的 `png + dat` 输出语义。
- **状态**：已完成。

### [2026-05-13] 新增 qeflow CLI Mermaid 流程图索引
- **需求**：
    1. 为 qeflow 总体 CLI 和各子模块补充 Mermaid 流程图，便于从 README 快速定位各命令链路。
    2. README 需要列出总览和分模块流程图入口。
- **方案**：
    1. 新增 `qeflow/qeflow-cli-flow.mmd` 作为 qeflow 总览图。
    2. 新增 `scf/nscf/eband/eledos/phonon/check/qe_input` 子模块流程图，分别描述各 CLI 输入、生成文件、运行/提交路径和输出。
    3. 更新 `qeflow/README.md`，在开头补充 Mermaid 流程图索引。
- **状态**：已完成。

### [2026-04-29] 新增 Codex 本地历史会话管理 skill
- **需求**：
    1. 设计并实现一个新的自定义 skill，用于管理 Codex 本地历史会话。
    2. 功能覆盖对话归档、对话摘要、标题建议/重命名流程、删除前确认与记录。
- **方案**：
    1. 在 `~/.codex/skills/codex-history-manager/` 下新增 `SKILL.md`、`agents/openai.yaml` 与 `scripts/manage_history.py`，将 skill 触发范围限定为 `~/.codex` 下的本地 JSONL 会话历史。
    2. 用统一 `argparse` 子命令实现 `list / summarize / suggest-title / archive / rename / delete` 六类操作；标题更新只改 `session_index.jsonl` 的 `thread_name`。
    3. 删除采用“先归档、后硬删除、必须显式 --confirm”的安全流程；归档和审计记录默认写入 `~/.codex/archived_sessions/`，且不修改 SQLite 内部库。
    4. 通过当前真实 `~/.codex` 的只读验证，以及临时 `codex_home` 的归档/重命名/删除闭环测试确认脚本行为。
- **状态**：已完成。

### [2026-04-26] 废弃 qe_phonon 工作目录 input.toml 副本并为 substitution_continue 新增 -c
- **需求**：
    1. `substitutionflow + qeflow phonon` 的唯一配置真源固定为 `case/input.toml`，`qe_phonon_*` 工作目录不再生成、修复或依赖 `input.toml` 副本。
    2. `substitution_continue` 新增 `-c/--config`，用于把新的根配置批量灌入已有 case 的 `input.toml`，并让后续续算/提交显式使用该真源配置。
    3. `substitution_continue -rws` 收敛为“按需要重写 case 真源配置并重写状态”；`qe_phonon_*` 残留目录只修复 `POSCAR`，不再修复 `input.toml`。
    4. `substitutionflow` 内部调用 `qe_phonon_*` 时必须显式透传 case 真源配置路径，旧目录中遗留的 `qe_phonon_*/input.toml` 一律忽略。
- **方案**：
    1. 修改 `substitutionflow/cli_continue.py` 与 `substitutionflow/workflow.py`，新增 `-c` 参数透传，并补充按已有 case 批量重写 `case/input.toml` 的公共逻辑，供 `continue_workflow()` 与 `rewrite_workflow_states()` 复用。
    2. 修改 `qeflow/qe_input/fileio.py`、`qeflow/phonon/_shared.py`、`qeflow/phonon/qe_merge.py` 与 `qeflow/phonon/workflow.py`，移除 `qe_phonon_*` 工作目录中的 `input.toml` 复制假设，统一通过显式 `config_file` 工作。
    3. 收敛 `_repair_qe_phonon_inputs()` 为仅修复 `POSCAR`，并把 `substitutionflow` 生成的 `qe_phonon_*` 命令改为显式传入 case 真源配置绝对路径。
    4. 更新 `README.md`、`qeflow/README.md`、`substitutionflow/substitutionflow-cli-flow.mmd` 与 `test_auto/MgB2/test_substitutionflow.py`、`test_auto/MgB2/test_qe_phonon.py`，同步新语义与回归测试。
- **状态**：已完成。

### [2026-04-25] 收紧 substitution_continue 的 refresh 边界到默认摘要模式
- **需求**：
    1. `substitution_continue -r` 与 `substitution_continue -s` 语义统一，二者都只从每个 pressure 当前状态文件中的第一个非 `finished` 步骤开始推进。
    2. `-r/-s` 不再在入口自动执行 `refresh_status()`；真实刷新只保留给默认无参数模式。
    3. 默认执行 `substitution_continue -o <dir>` 时，仍需先真实刷新再输出摘要，作为手工校准状态的专用入口。
- **方案**：
    1. 修改 `substitutionflow/workflow.py` 中的 `continue_workflow()`，将无条件入口刷新改为仅在 `run_calc=False` 且 `submit=False` 时执行。
    2. 保持 `-r` 执行过程中的步后刷新不变，只删除运行前的预刷新；`-s` 继续基于现有状态文件收集待提交单元，不再预刷新。
    3. 更新 `substitutionflow/cli_continue.py`、`README.md` 和 `test_auto/MgB2/test_substitutionflow.py`，把帮助文本、文档和回归测试同步收敛到新的 refresh 边界。
- **状态**：已完成。

### [2026-04-25] 删除 substitution_prepare_qe_pseudo 的 case pp_dir 自动回写
- **需求**：
    1. `substitution_prepare_qe_pseudo` 不再批量修改各 case `input.toml` 的 `pp_dir`。
    2. 根配置固定允许直接写 `pp_dir = "qe_pseudo_lib"`，`substitution_structure` 对该值原样继承。
    3. `substitution_prepare_qe_pseudo` 的职责收敛为“推导元素 + 选择候选 + 写入共享 `qe_pseudo_lib/` 与 `qe_pseudo_lib.log`”，行为与 `substitution_prepare_potcar` 对齐。
    4. QE 相关 CLI 需要支持在批量工作流中从相对 `pp_dir` 稳定定位共享赝势目录，避免依赖自动回写绝对路径。
- **方案**：
    1. 修改 `substitutionflow/prepare_qe_pseudo.py`，删除 case `input.toml` 回写逻辑，仅保留 QE 赝势库准备与记录文件维护。
    2. 修改 `qeflow/common.py` 与 QE 各 CLI/config loader，把相对 `pp_dir` 规范化为可用目录路径，优先当前目录，再从配置文件所在目录向上逐级搜索，支持批量根目录共享 `qe_pseudo_lib/`。
    3. 更新 `test_auto/MgB2/test_substitutionflow.py`，把 QE 测试前提收敛为根配置直接写 `pp_dir = "qe_pseudo_lib"`，删除对自动回写与 `[pseudo_map]` 的旧断言。
    4. 更新 `README.md`、`qeflow/README.md` 与 `substitutionflow/substitutionflow-cli-flow.mmd`，删除自动回写描述，明确新的 `pp_dir` 约定。
- **状态**：已完成。

### [2026-04-25] Mermaid 流程图统一放入各自模块目录
- **需求**：
    1. `substitutionflow` 的流程图 `substitutionflow-cli-flow.mmd` 不再放在仓库根 `docs/` 目录。
    2. 以后新增 `.mmd` 流程图时，统一放在对应模块的 `*flow/` 目录下，避免文档位置分散。
- **方案**：
    1. 将 `docs/substitutionflow-cli-flow.mmd` 迁移到 `substitutionflow/substitutionflow-cli-flow.mmd`。
    2. 同步修正仓库内对该文件路径的文字记录，后续其它模块流程图遵循同一目录约定。
- **状态**：已完成。

### [2026-04-24] 取消 substitutionflow 的 running 状态并收敛为三态
- **需求**：
    1. `substitutionflow` 中不再保留 `running` 状态，状态集合统一收敛为 `finished / error / unstarted`。
    2. 删除基于 Slurm `squeue/%Z` 与本地活动工作目录的运行探测逻辑，不再因为“运行中”而特殊跳过续算或提交。
    3. 纯 setup-only 目录仍应判为 `unstarted`，但存在明确未完成运行残留的目录应判为 `error`，以免破坏正常续算语义。
    4. `qe_phonon` 旧历史状态或 task 中若仍出现 `running`，刷新后必须向三态模型收敛，并统一按 `error` 处理。
- **方案**：
    1. 修改 `substitutionflow/status.py`，删除 `running` 相关常量、探测函数和活动目录参数，所有步骤状态检测改为纯基于目录内容与输出文件判定。
    2. 修改 `substitutionflow/workflow.py`，删除 `running` 跳过与过滤逻辑，使运行/提交流程只按 `finished / error / unstarted` 三态推进。
    3. 更新 `README.md`，明确 `substitutionflow` 不再维护 `running` 状态，续算与提交也不再依赖 Slurm 或本地进程探测。
    4. 更新 `test_auto/MgB2/test_substitutionflow.py`，删除 `running` 相关测试假设，并补充旧 `running` 历史值映射为 `error`、setup-only 保持 `unstarted` 等回归场景。
- **状态**：已完成。

### [2026-04-24] qeflow 废弃 pseudo_map 并统一从 pp_dir 自动匹配赝势
- **需求**：
    1. `qeflow` 中所有会写 QE `ATOMIC_SPECIES` 的 CLI 必须废弃 `[pseudo_map]`，统一仅依赖 `pp_dir` 自动解析赝势文件名。
    2. 匹配规则固定为扫描 `pp_dir` 直下文件，并按文件名开头连续字母串与元素名做不区分大小写比较。
    3. `qe_scf`、`qe_nscf`、`qe_eband` 以及 phonon builder 路径都要收敛到同一套公共解析逻辑，避免重复实现和行为分叉。
    4. 模板、README 和自动化测试必须同步删除旧 `pseudo_map` 语义，并补足缺失/重名冲突/大小写匹配回归。
- **方案**：
    1. 在 `qeflow/common.py` 新增 `extract_pseudo_element_token()`、`find_pseudo_file()`、`resolve_pseudo_files()` 和 `resolve_atomic_species_entries()`，统一处理赝势自动匹配与错误报告。
    2. 修改 `qeflow/qe_input/builders.py`、`qeflow/scf/qe_scf.py`、`qeflow/nscf/qe_nscf.py`、`qeflow/eband/qe_eband.py`，全部改为复用公共 `ATOMIC_SPECIES` 解析结果。
    3. 修改 `qeflow/templates.py`、`qeflow/README.md` 和 `test_auto/MgB2/input_qe.toml`，移除 `[pseudo_map]`，统一改为仅保留 `pp_dir`。
    4. 更新 `test_auto/MgB2/test_qeflow.py`、`test_auto/MgB2/test_qe_phonon.py`、`test_auto/MgB2/test_parameter_templates.py`，覆盖自动匹配、phonon builder 路径、大小写匹配、缺失赝势和重复候选报错。
- **状态**：已完成。

### [2026-04-24] 重构 substitution_continue 的状态重写与续算/提交边界
- **需求**：
    1. `substitution_continue` 中的“状态重写”和“续算/提交”必须彻底解耦。
    2. 废弃旧的 `--fix-error`，新增独立参数 `--rewrite-states/-rws`，其语义固定为“只重写状态 JSON，不执行续算，也不生成/提交脚本”。
    3. `-rws` 不能与 `-r/-s` 混用；`continue -r` 与 `continue -s` 必须始终先基于真实目录和运行状态刷新，而不是信任手工改写后的状态文件。
    4. `running` 状态必须保留，并继续按真实运行中语义写回 `case_status.json` 与 `workflow_status.json`。
- **方案**：
    1. 修改 `substitutionflow/cli_continue.py`，删除 `--fix-error`，新增 `--rewrite-states/-rws` 并增加互斥校验。
    2. 修改 `substitutionflow/workflow.py`，移除 `fix_error` 旁路，新增独立 `rewrite_workflow_states()`；默认模式只检查并输出摘要，`-r` 与 `-s` 都在真实刷新后推进。
    3. 修改 `substitutionflow/status.py`，删除 `reset_error_statuses()`，保留 `refresh_status()` 作为唯一状态重写真源，并补充本地进程工作目录匹配的 `running` 检测。
    4. 更新 `test_auto/MgB2/test_substitutionflow.py`，删除旧 `fix_error` 测试，新增 `-rws`、参数冲突、预刷新阻断和提交收集边界测试。
- **状态**：已完成。

### [2026-04-10] 新增 vasp_plot_cohp 用于 orbital-resolved COHP/ICOHP 分析
- **需求**：
    1. 新增一个 CLI，读取 `COHPCAR.lobster` 并对指定轨道对做 orbital-resolved `COHP / ICOHP` 分析。
    2. 最终对外输入语法与 `vasp_plot_kcohp` 统一，使用 `--interaction "元素A:轨道列表|元素B:轨道列表"`。
    3. 输出 `proj_cohp.dat`、`average_cohp.dat`，并分别绘制投影图和 average 图。
    4. 支持打印 `ICOHP @ E_F`，以及在给定 `[low, up]` 能窗时打印能窗差值表。
- **方案**：
    1. 新增 `vaspflow/cohp/vasp_plot_cohp.py`，作为纯后处理 CLI；内部使用 `pymatgen.io.lobster.outputs.Cohpcar` 惰性加载 `COHPCAR.lobster`。
    2. 轨道匹配与 `vasp_plot_kcohp` 复用同一套公共解析逻辑，因此 `Mg:3p|B:2p` 会自动聚合所有 `p_x/p_y/p_z` 分量。
    3. 数值策略保持旧脚本一致：匹配到的轨道贡献先求和，再除以总键数 `num_bonds`，不改成按匹配轨道对数平均。
    4. 默认输出 `proj_cohp.png` 与 `average_cohp.png` 两张图，不把 average 曲线和投影轨道曲线混画在同一张图里。
- **状态**：已完成。

### [2026-04-11] vasp_opt 的 -p 压力参数改为按 GPa 输入
- **需求**：
    1. `vasp_opt -p/--pressure` 接收的压力单位应为 `GPa`，与用户实际使用习惯一致。
    2. VASP `INCAR` 中 `PSTRESS` 的单位仍然是 `kbar`，写入前必须完成单位换算。
- **方案**：
    1. 在 `vaspflow/opt/vasp_opt.py` 中把命令行 `-p` 类型放宽为 `float`，支持非整数压力。
    2. 当显式传入 `-p` 时，内部统一执行 `GPa * 10 = kbar` 转换后再写入所有优化阶段的 `PSTRESS`。
    3. 保持目录命名仍使用用户输入的 `GPa` 数值，例如 `1.5/vasp_opt`，并补充自动化测试锁定该行为。
- **状态**：已完成。

### [2026-04-11] vasp_opt 的 -p 支持一次传入多个压强点
- **需求**：
    1. `vasp_opt -p` 不应只支持单个压力值，而应支持一次性传入多个 `GPa` 压强点。
    2. 多压强场景下，需要为每个压强分别生成独立目录，并保留单压强时的原有行为。
    3. `--run` 与 `--submit` 在多压强场景下也必须可用，不能只生成文件不支持批量执行。
- **方案**：
    1. 将 `vaspflow/opt/vasp_opt.py` 中的 `-p/--pressure` 改为 `nargs='+'`，支持 `-p 0 5 10` 这类输入。
    2. 为每个压强点分别生成 `<pressure_GPa>/vasp_opt` 目录，目录名保留用户输入的 `GPa` 数值格式。
    3. 多压强时在当前目录额外生成 `submit_all_vasp_opt.sh`，其中顺序进入各个 case 目录并执行对应优化脚本；`-r` 时顺序执行全部 case，`-s` 时提交该总脚本。
    4. 补充自动化测试，验证多目录生成、`GPa -> kbar` 单位换算以及总脚本写出行为。
- **状态**：已完成。

### [2026-04-11] vasp_opt 多压强总脚本改为直接展开执行命令
- **需求**：
    1. `submit_all_vasp_opt.sh` 进入各 case 目录后，应直接执行 `[vasp].executable_path` 对应命令。
    2. 不能继续通过 `bash fouropt.sh`、`bash oneopt.sh`、`bash fopt.sh` 间接调用，否则总脚本语义与用户预期不一致。
- **方案**：
    1. 在 `vaspflow/opt/vasp_opt.py` 中抽出按 `rv4/rv1/rvf` 生成命令序列的公共方法。
    2. 单目录脚本 `fouropt.sh`、`oneopt.sh`、`fopt.sh` 继续复用这套命令生成逻辑，避免重复实现。
    3. 多压强 `submit_all_vasp_opt.sh` 直接内联各目录对应的 VASP 命令序列，不再写出 `bash <case_script>`。
    4. 更新测试，显式断言总脚本中不再出现 `bash oneopt.sh` / `bash fouropt.sh`，而是直接包含 `executable_path` 命令。
- **状态**：已完成。

### [2026-04-10] 新增 vasp_plot_kcohp 用于 band-resolved pCOHP 绘图
- **需求**：
    1. 新增一个 CLI，读取 LOBSTER `kSpaceCOHP` 结果目录，绘制 band-resolved projected COHP 图。
    2. 保留旧脚本的解析策略：读取 `KspaceCOHPBand*.lobster`、`POSCAR`、`KPOINTS`、`KPATH.in`，并输出 `DAT + PNG`。
    3. 大幅简化输入，不再让用户手写 regex，而是改为 `--interaction "元素A:轨道列表|元素B:轨道列表"`。
    4. 费米能默认从 `OUTCAR` 读取，绘图纵坐标统一为 `E - E_F`。
- **方案**：
    1. 新增 `vaspflow/cohp/vasp_plot_kcohp.py`，作为纯后处理 CLI，不接入 `input.toml` 模板层。
    2. 每个 `--interaction` 对应一个子图；轨道匹配默认自动汇总所有匹配原子，例如 `Mg:3p|B:2p` 会自动聚合所有 `Mg*` 与 `B*` 的相应轨道。
    3. 输出 `.dat` 文件时保留按 band 分块的格式；输出 `.png` 时保留灰色背景能带、颜色表示 pCOHP、多子图布局与高对称点标记。
    4. 同步更新 `pyproject.toml`、README、`vaspflow/README.md` 和自动化测试。
- **状态**：已完成。

### [2026-04-14] 新增 vasp_process_eledos 用于 VASP DOS 后处理
- **需求**：
    1. 新增一个纯后处理 CLI，处理 `vasp_eledos` 目录中的 `DOSCAR/TDOS/PDOS` 数据。
    2. 支持按 `--elements "元素:轨道"` 指定需要聚合的元素轨道，例如 `"La:s,p,d"`、`"H:s"`。
    3. 在调用 `vaspkit` 之前，先将 `DOSCAR` 的费米能级同步为 `../vasp_scf/OUTCAR` 中的 `E-fermi`。
    4. 输出聚合后的 `CSV`、费米能处插值 `CSV`，并基于保存后的 `CSV` 自动绘制 DOS 图，图中标出费米能级。
- **方案**：
    1. 在 `vaspflow/eledos/vasp_process_eledos.py` 中实现纯后处理 CLI，不接入 `input.toml` 模板层。
    2. 默认调用 `vaspkit 11 -> 113` 生成元素分波 `PDOS_<元素>.dat`，调用 `vaspkit 11 -> 111` 生成 `TDOS.dat`；也支持 `--skip-vaspkit` 直接复用现有文件。
    3. 第一版按非自旋极化 `vaspkit` 输出实现 `s/p/d/f` 聚合，并支持 `--per-volume` 按晶胞体积归一化。
    4. 同步注册 `pyproject.toml` 入口，更新 README，并补充自动化测试覆盖费米能修正、CSV 输出和 PNG 绘图。
- **状态**：已完成。

### [2026-04-14] 新增 vasp_process_eband 用于 VASP 能带后处理
- **需求**：
    1. 新增一个纯后处理 CLI，处理 `vasp_eband` 目录中的 `BAND.dat / PBAND_<元素>.dat / KLABELS / FERMI_ENERGY` 数据。
    2. 支持按 `--elements "元素:轨道"` 指定需要聚合的元素轨道，例如 `"Mg:s,p"`、`"B:p"`。
    3. 输出聚合后的 `CSV`，并自动绘制投影能带图，图中包含灰色背景 band、费米能级水平线和高对称点竖线。
    4. 第一版优先复用 `vaspkit` 已生成的 `BAND/PBAND` 数据格式，不直接解析 `PROCAR`。
- **方案**：
    1. 在 `vaspflow/eband/vasp_process_eband.py` 中实现纯后处理 CLI，不接入 `input.toml` 模板层。
    2. 默认调用 `vaspkit` 生成 `BAND.dat` 与 `PBAND_<元素>.dat`；也支持 `--skip-vaspkit` 直接复用现有文件。
    3. 第一版按非自旋极化 `PBAND_<元素>.dat` 实现 `s/p/d/f` 聚合，能量默认平移到 `E - E_F`，其中费米能优先读取 `FERMI_ENERGY`，否则退回 `OUTCAR`。
    4. 同步注册 `pyproject.toml` 入口，更新 README，并补充自动化测试覆盖 CSV 输出和 PNG 绘图。
- **状态**：已完成。

### [2026-04-14] 新增 vasp_idos_computed 用于能量区间 IDOS 计算
- **需求**：
    1. 在 `vaspflow/eledos` 下新增一个纯后处理脚本，用于计算指定能量区间内的 IDOS。
    2. 同时支持 `TDOS` 与通过 `--elements "元素:轨道"` 指定的聚合 `PDOS` 区间积分。
    3. 输出结果 `CSV`，并在终端打印积分摘要。
- **方案**：
    1. 新增 `vaspflow/eledos/vasp_idos_computed.py`，命名固定为 `vasp_idos_computed`。
    2. 复用 `vasp_process_eledos` 中现有的费米能修正、`vaspkit` 调用、`TDOS/PDOS` 解析与体积读取逻辑，避免重复实现。
    3. 通过 `--low` / `--up` 指定单个能量区间，在 `E - E_F` 坐标下做端点线性插值后的梯形积分。
    4. 注册 `pyproject.toml` 入口，更新 README，并补充自动化测试覆盖区间积分与 CSV 输出。
- **状态**：已完成。

### [2026-04-14] 修正 DOSCAR 费米能替换格式，并收敛 vasp_process_eband 的 vaspkit 流程
- **需求**：
    1. 修改 `DOSCAR` 第 6 行费米能时，不能重排整行空格，否则可能导致 `vaspkit` 对 `DOSCAR` 的解析失败。
    2. 费米能替换应尽量保持原 token 的数值格式，只替换第 4 个数值本身。
    3. `vasp_process_eband` 需要参考现有 shell 工作流，在调用 `vaspkit` 前先同步 `DOSCAR` 的费米能，并执行 `21/211`、`21/213`、`21/215` 三步。
- **方案**：
    1. 在 `vaspflow/eledos/vasp_process_eledos.py` 中重写 `replace_doscar_fermi()`，改为基于第 6 行第 4 个非空白 token 的原位替换，保留原始空格布局。
    2. 替换时沿用原 token 的表示方式，优先保留原有小数位数与科学计数法风格，避免无意义格式漂移。
    3. 在 `vaspflow/eband/vasp_process_eband.py` 中新增 `--scf-outcar`、`--doscar` 参数；非 `--skip-vaspkit` 场景下先同步 `DOSCAR` 费米能，再运行 `211 + 213 + 215`。
    4. 更新 `vaspflow/README.md`，并补充自动化测试，验证 `DOSCAR` 空格模式保持不变以及 `vasp_process_eband` 的 `vaspkit` 调用顺序。
- **状态**：已完成。

### [2026-04-14] 扩展 vasp_process_eledos 与 vasp_process_eband 的精细轨道支持
- **需求**：
    1. `vasp_process_eledos` 与 `vasp_process_eband` 不仅要支持 `s/p/d/f`，还要支持更精细的角量子数轨道选择。
    2. 两个 CLI 的轨道语法必须保持一致，避免同一输入在不同后处理脚本中语义不同。
    3. 允许的精细轨道集合固定为 `s py pz px dxy dyz dz2 dxz dx2 fy3x2 fxyz fyz2 fz3 fxz2 fzx2 fx3`。
- **方案**：
    1. 在 `vaspflow/eledos/vasp_process_eledos.py` 中统一维护轨道白名单与“粗粒度轨道 -> 精细分量”映射。
    2. `vasp_process_eledos` 读取 `PDOS_<元素>.dat` 时，若用户指定粗粒度轨道则自动求和，若指定精细轨道则直接读取对应列。
    3. `vasp_process_eband` 直接复用 `vasp_process_eledos` 的轨道解析逻辑，并通过固定列索引映射读取 `PBAND_<元素>.dat` 中的精细分量或求和结果。
    4. 更新 README 和自动化测试，覆盖 `px/dz2/dx2` 这类精细轨道输入。
- **状态**：已完成。

### [2026-04-17] 新增 substitutionflow 高通量替换与续算工作流
- **需求**：
    1. 新增正式模块 `substitutionflow/`，用于对含 `XA/XB` 假元素的占位结构做高通量元素替换。
    2. 替换集合只从 `[substitutionflow]` 读取，语法使用原子序号范围，如 `XA = "1-3,4,6-9"`。
    3. 目录按 `<Z>.XA_<元素>/period_n/<Z>.<formula>` 组织，并在每个 case 下展开 `vasp_opt -> vasp_scf -> vasp_eledos -> vasp_process_eledos`。
    4. 从工作流开始就维护 `workflow_status.json`，并新增 `substitution_continue` 负责检查、汇总和续算。
- **方案**：
    1. 新增 `substitution_structure`、`substitution`、`substitution_continue` 三个 CLI。
    2. 在模块内拆分 `selection.py`、`structure_substitution.py`、`workflow.py`、`status.py`，分别承担元素选择、结构替换、工作流编排和状态管理。
    3. 第一版只支持两个占位位点 `XA/XB`，其余真实元素保持不变；目录树和状态文件在生成替换结构时一并初始化。
    4. 为满足工作流默认输出每种元素总投影，扩展 `vasp_process_eledos` 支持 `--elements-total` 模式。
    5. 更新 `pyproject.toml`、README、自动化测试和文档说明。
- **状态**：已完成。

### [2026-04-17] 收紧 substitution_structure 的结构输入来源
- **需求**：
    1. `substitution_structure` 不应再要求用户额外传 `-i`。
    2. 既然 `[substitutionflow].structure_filename` 已存在，结构文件路径应直接从配置文件中解析。
- **方案**：
    1. 删除 `substitution_structure` 的 `-i/--input` 参数。
    2. 新增按“配置文件所在目录 + structure_filename”解析输入结构路径的逻辑。
    3. 补充自动化测试，验证仅指定 `-c/-o` 即可正常生成替换目录。
- **状态**：已完成。

### [2026-04-17] 重构 substitutionflow 的状态判定与续算机制
- **需求**：
    1. `workflow_manifest.json` 改为放在 `period_n/` 目录，只保留静态 case 清单，不再承担动态状态职责。
    2. 每个实际体系目录新增 `case_status.json`，记录各压力点在 `vasp_opt / vasp_scf / vasp_eledos / vasp_process_eledos` 上的 `finished / running / error / unstarted` 状态。
    3. 输出根目录中的 `workflow_status.json` 改为纯汇总文件，供 `substitution_continue` 统计、扫描和续算使用。
    4. 状态判定必须按真实 VASP 输出语义实现：`vasp_opt` 检查结构优化收敛关键词，`vasp_scf/vasp_eledos` 检查电子步收敛、`NELM` 与关键输出，`vasp_process_eledos` 检查 `*_at_ef.csv`。
    5. `running` 状态需要通过 `squeue --format='%12A %12P %5t %25R %8M %Z'` 的工作目录匹配识别，而不是手工写死。
- **方案**：
    1. 重写 `substitutionflow/status.py`，新增 manifest 扫描、case 状态文件初始化、四步状态判定、根目录汇总构建与 `squeue` 运行中检测。
    2. 调整 `substitutionflow/structure_substitution.py`，按 `period_n/` 分组写出 `workflow_manifest.json`，不再在 case 目录写 manifest。
    3. 调整 `substitutionflow/workflow.py`，不再把根目录状态文件当真源，而是始终基于 `workflow_manifest.json + case_status.json` 刷新后再决定 setup、续算和提交脚本生成。
    4. 重写 `test_auto/MgB2/test_substitutionflow.py`，覆盖 period-level manifest、case-level status、`finished/error/running/unstarted` 判定以及 continue 续算链路。
- **状态**：已完成。

### [2026-04-18] 新增 substitution_prepare_potcar 用于准备本地 potcar_lib
- **需求**：
    1. 新增独立 CLI `substitution_prepare_potcar`，从 `input.toml` 中读取 `[substitutionflow]` 与占位结构，自动推导当前高通量工作流所需元素集合。
    2. 从 `--pot` 指定的原始 VASP 赝势库中搜索候选赝势；若同一元素存在多个候选，需要交互式逐个询问用户选择。
    3. 每个候选都要打印价电子数和价电子电子构型；价电子数优先读 `ZVAL`，回退到第二行，电子构型从 `Atomic configuration` 反向累加价电子得到。
    4. 选择结果需要保存到当前目录 `potcar_lib/potcar_lib.log`，脚本下次运行时默认直接读取该文件续选，不再提供 `-psf/--potcar-selected-file`。
    5. 准备好的赝势输出必须兼容现有 `vaspflow` 的 `potcar_dir` 读取方式，即保存成扁平文件 `potcar_lib/<element>`。
- **方案**：
    1. 新增 `substitutionflow/prepare_potcar.py`，实现元素推导、候选归类、POTCAR 信息解析、交互选择和 `potcar_lib.log` 读写，并在每完成一个元素选择后立即刷新记录文件。
    2. 在 `pyproject.toml` 注册 `substitution_prepare_potcar` CLI，并在 `README.md` 中补充用途说明。
    3. 候选归类按“目录名前缀等于元素符号”处理，例如 `Ce/Ce_GW` 归类为 `Ce`、`V/V_pv/V_sv` 归类为 `V`。
    4. 新增自动化测试，覆盖元素发现、POTCAR 解析、交互选择、`potcar_lib.log` 复用和输出兼容性。
- **状态**：已完成。

### [2026-04-18] 新增 mattersim_opt 并接入 substitutionflow
- **需求**：
    1. 在 `vasp_opt` 之前增加 MatterSim 预优化，以提高高压结构优化效率。
    2. 新增独立模块 `mattersimflow/` 和 CLI `mattersim_opt`，接口风格对齐 `vasp_opt`。
    3. MatterSim 预优化固定分两步：先只优化原子坐标，再在目标压力下优化晶格和原子坐标。
    4. `substitution` 的工作流顺序改为 `mattersim_opt -> vasp_opt -> vasp_scf -> vasp_eledos -> vasp_process_eledos`。
- **方案**：
    1. 新增 `mattersimflow/templates.py` 与 `mattersimflow/opt/mattersim_opt.py`，通过 `[mattersim].executable_path` 调用外部 MatterSim Python 环境，并在工作目录生成 runner 脚本、`run_mattersim.sh`、`CONTCAR` 与 `mattersim_opt.json`。
    2. 配置新增顶层 `mattersim_source_path` 与 `[mattersim_opt]`，用于控制模型、设备、两阶段步数与收敛阈值。
    3. 扩展 `substitutionflow/status.py` 的状态机，把 `mattersim_opt` 纳入 `finished / running / error / unstarted` 判定和根目录汇总。
    4. 修改 `substitutionflow/workflow.py`，使 `vasp_opt` 固定读取 `mattersim_opt/CONTCAR`，并更新提交脚本展开顺序。
    5. 增加自动化测试，覆盖 `mattersim_opt` 输入生成、多压力目录、提交接线以及高通量工作流的新步骤链。
- **状态**：已完成。

### [2026-04-18] substitutionflow 的批量提交改为按完整压强点单元分组
- **需求**：
    1. `substitution -s` 与 `substitution_continue -s` 不再把全部未完成任务写入一个总脚本。
    2. 提交的最小单元固定为“同一 case 的同一 pressure 的完整未完成步骤链”，即 `mattersim_opt -> vasp_opt -> vasp_scf -> vasp_eledos -> vasp_process_eledos`，不得拆分到不同脚本。
    3. 分组数量只由 `[substitutionflow].submit_groups` 控制，默认值为 `1`。
- **方案**：
    1. 在 `substitutionflow/workflow.py` 中新增待提交单元收集、顺序均分、分组脚本生成与逐组提交逻辑，脚本固定命名为 `submit_group_01.sh`、`submit_group_02.sh` 等。
    2. `running` 或已 `finished` 的压强点不进入新的分组脚本；某个压强点若已完成前半段步骤，则只把剩余步骤整段放入同一个组。
    3. `substitution` 与 `substitution_continue` 的 `-s` 都改为逐组 `sbatch`，任一组提交失败即停止后续提交。
    4. 更新 README、CLI 帮助文本和自动化测试，覆盖默认单组、多组均分、单元不可拆分、跳过 `running` 与逐组提交。
- **状态**：已完成。

### [2026-04-21] 新增 substitution_plot_eledos 周期表式 DOS 总览图
- **需求**：
    1. 新增独立 CLI `substitution_plot_eledos`，按 XA 目录汇总 `vasp_process_eledos` 的 `*_at_ef.csv` 结果。
    2. 图面布局必须复用 `substitutionflow/selection.py` 的周期表坐标和镧系/锕系对齐方式。
    3. 每个 case 位点固定绘制 `XA / XB / H / TDOS` 四个紧挨着的柱状图；未完成位点保留格子并标灰。
    4. `-d/--directory` 省略时默认扫描当前目录下全部 `[Z].XA_*` 目录，也支持传根目录或多个指定 XA 目录。
- **方案**：
    1. 新增 `substitutionflow/plot_eledos.py`，实现目录解析、`*_at_ef.csv` 读取、pressure 维度拆图以及周期表式绘图。
    2. 列提取固定按 case 元信息中的 `xa_symbol`、`xb_symbol`，外加 `H` 和 `TDOS`；任一缺失即整格按缺失处理。
    3. 同一张图统一使用全局 `ymax`，输出固定命名为 `<XA_dir>/eledos_summary_<pressure>.png`。
    4. 在 `pyproject.toml` 注册 `substitution_plot_eledos` 入口，并补充 README 与自动化测试。
- **状态**：已完成。

### [2026-04-10] 统一 vasp_plot_cohp 与 vasp_plot_kcohp 的相互作用输入语法
- **需求**：
    1. `vasp_plot_cohp` 与 `vasp_plot_kcohp` 对元素和轨道的设置方式必须一致。
    2. `vasp_plot_cohp` 不再继续使用旧的 `3s-2p` 轨道对子串接口，统一切换为 `--interaction "元素A:轨道列表|元素B:轨道列表"`。
- **方案**：
    1. 在 `vaspflow/cohp/orbital_interactions.py` 中抽出公共的相互作用解析与轨道匹配逻辑，统一处理元素筛选、壳层自动展开和标签格式。
    2. `vasp_plot_kcohp` 改为直接复用该公共模块，删除本地重复实现。
    3. `vasp_plot_cohp` 改为读取重复传入的 `--interaction`，并按与 `vasp_plot_kcohp` 相同的规则匹配 `COHPCAR.lobster` 中的 orbital-resolved 项。
    4. 同步更新测试与文档，明确两个 CLI 的输入语法完全一致。
- **状态**：已完成。

### [2026-04-10] vasp_cohp_fatband 的 kmesh_scheme 语义收紧
- **需求**：
    1. `vasp_cohp_fatband` 中 `kmesh_scheme` 只允许使用 VASPKIT 251 对应的两种模式名：`Monkhorst-Pack` 和 `Gamma`。
    2. `Monkhorst-Pack` 必须映射到菜单数字 `1`，`Gamma` 必须映射到菜单数字 `2`。
    3. 清理此前代码、模板、测试中混用 `monkhorst`、`gamma`、小写比较等不一致行为。
- **方案**：
    1. 修正 `vaspflow/cohp/vasp_cohp_fatband.py` 中的判断逻辑，不再对 `kmesh_scheme` 做 `.lower()` 后再与大写值比较。
    2. 固定仅接受 `Monkhorst-Pack` / `Gamma` 两个字符串，并分别映射到 `1` / `2`。
    3. 同步更新 `test_auto/MgB2/input_vasp.toml`、模板覆盖测试和 `vaspflow/README.md`，统一对外语义。
- **状态**：已完成。

### [2026-04-10] vasp_cohp 拆分为 uniform 与 fatband 两个 CLI
- **需求**：
    1. 原有 `vasp_cohp` 不再保留，拆分成 `vasp_cohp_uniform` 与 `vasp_cohp_fatband` 两个独立 CLI。
    2. `vasp_cohp_uniform` 保留原有均匀网格自洽逻辑，工作目录改为 `vasp_cohp_uniform`。
    3. `vasp_cohp_fatband` 新增独立目录 `vasp_cohp_fatband`，其 `KPOINTS` 生成流程固定为 `vaspkit 303 + 251`。
    4. 配置块改为 `[vasp_cohp_uniform]` 和 `[vasp_cohp_fatband]`；`fatband` 使用 `scf_kmesh`、`kpath_kmesh`、`kmesh_scheme`。
- **方案**：
    1. 删除旧 `vasp_cohp` CLI 注册与旧源码，新增 `vasp_cohp_uniform`、`vasp_cohp_fatband` 两个入口。
    2. 抽出 `cohp` 公共逻辑，复用 INCAR/POTCAR/CHGCAR/WAVECAR/运行脚本/距离分析流程。
    3. `uniform` 继续使用均匀网格 `KPOINTS`；`fatband` 在工作目录中先运行 `303` 生成 `KPATH.in`，再运行 `251` 生成最终混合 `KPOINTS`。
    4. 更新模板、测试配置、自动化测试与文档，彻底移除对旧 `[vasp_cohp]` 和 `vasp_cohp` 命名的依赖。
- **状态**：已完成。

### [2026-04-10] vasp_opt 去除 KPOINTS 依赖并统一使用 KSPACING
- **需求**：
    1. `vasp_opt` 不应生成 `KPOINTS`，也不应复制当前目录中的 `KPOINTS`。
    2. `rv1`、`rvf`、`rv4` 最后一步都应通过 `[vasp_opt].KSPACING` 或 `[vasp_opt].kmesh * 2π` 控制 k 点精度。
    3. `rv4` 的前三步继续保留各自独立的粗优化 `KSPACING`。
- **方案**：
    1. 删除 `vaspflow/opt/vasp_opt.py` 中残留的 `KPOINTS` 复制逻辑，使 `vasp_opt` 完全脱离 `KPOINTS` 文件依赖。
    2. 保留并测试 `rv1`、`rvf`、`rv4` 最终步的 `KSPACING` 计算：优先使用显式 `KSPACING`，否则使用 `kmesh * 2 * 3.1415926`。
    3. 在 `test_auto/MgB2/test_vaspflow.py` 中补充测试，验证不会复制 `KPOINTS`，并验证 `rv4` 前三步固定 `KSPACING` 与最终步换算逻辑。
    4. 更新 `vaspflow/README.md`，明确 `vasp_opt` 是 `vaspflow` 中通过 `KSPACING` 控制 k 点精度的特例。
- **状态**：已完成。

### [2026-04-10] 新增独立 vasp_lobsterin CLI
- **需求**：
    1. 在 `vaspflow/cohp/` 中新增独立工具，用于生成 `lobsterin`、准备 LOBSTER 运行目录并执行或提交作业。
    2. `lobster_executable_path` 作为顶层键配置，默认值固定为 `/home/mayuan/bin/lobster-5.1.1`，不放在任何 `[xxx]` 模块下。
    3. 不复活旧的 `vasp_lobster` / `vasp_process_cohp` 命名；VASP 侧 COHP 前置计算已拆分为 `vasp_cohp_uniform` 与 `vasp_cohp_fatband`。
- **方案**：
    1. 在 `vaspflow/cohp/vasp_lobsterin.py` 中实现独立 CLI，生成 `lobsterin`、软链接所需输入文件，并写出 `run_lobster.sh`。
    2. 在 `vaspflow/templates.py` 中新增 `get_vasp_lobsterin_parameters_template()`，只暴露顶层 `lobster_executable_path` 与 `[slurm]`。
    3. 在执行与提交阶段校验 `lobster_executable_path`，并沿用公共 `-r/--run`、`-s/--submit` 逻辑。
    4. 更新 `pyproject.toml`、README、测试配置与自动化测试，覆盖模板合并、脚本生成和 `--submit` 接线。
- **状态**：已完成。

### [2026-04-10] vasp_lobsterin 支持可选 orbitalWise 与 kSpaceCOHP
- **需求**：
    1. `vasp_lobsterin` 默认不再强制在 `cohpGenerator` 后追加 `orbitalWise`。
    2. 当显式指定 `--orbitalWise` 时，才在 `cohpGenerator` 行末追加 `orbitalWise`。
    3. 当显式指定 `--kSpaceCOHP` 时，在同一行末追加 `kSpaceCOHP`，并把执行目录所需的 `KPOINTS`、`KPATH.in` 一并准备好。
- **方案**：
    1. 在 `vaspflow/cohp/vasp_lobsterin.py` 中新增 `--orbitalWise` 与 `--kSpaceCOHP` 两个布尔开关，并按开关状态拼接 `cohpGenerator` 行。
    2. `kSpaceCOHP` 场景下不再软链接 `KPOINTS`，而是直接复制 `KPOINTS` 与 `KPATH.in` 到 LOBSTER 工作目录，避免后续修改原目录或路径缺失导致运行失败。
    3. 在 `test_auto/MgB2/test_vaspflow.py` 中补充自动化测试，验证默认不追加、显式追加以及 `KPOINTS/KPATH.in` 准备行为。
- **状态**：已完成。

### [2026-04-10] 移除 vasp_lobster / vasp_process_cohp 旧入口
- **需求**：
    1. 不再保留 `vasp_lobster` 与 `vasp_process_cohp` 相关对外接口；COHP 前置计算统一使用 `vasp_cohp_uniform` 与 `vasp_cohp_fatband`，LOBSTER 输入准备统一使用 `vasp_lobsterin`。
    2. 清理仓库内残留的旧入口注册、模板说明与文档引用，避免继续误导安装环境生成旧脚本。
- **方案**：
    1. 从 `pyproject.toml` 删除 `vasp_lobster = "vaspflow.cohp.vasp_process_cohp:main"` 旧 CLI 注册。
    2. 删除 `vaspflow/cohp/vasp_process_cohp.py` 旧入口源码，并移除 `vaspflow/templates.py` 中仅服务于该入口的 `get_vasp_lobster_allowed_keys()`。
    3. 同步更新 `README.md`、`vaspflow/README.md`、`CLAUDE.md`，移除旧入口并切换到新的 `vasp_cohp_uniform`、`vasp_cohp_fatband`、`vasp_lobsterin` 命名。
- **状态**：已完成。

### [2026-04-07] vasp_fermi 固定使用 vaspkit 261 生成 K 点
- **需求**：
    1. `vasp_fermi` 生成 K 点时，`vaspkit` 菜单必须使用 `261`，不能继续使用 `102`。
    2. 不做任何旧方式兼容，`vasp_fermi` 只允许 `261` 这一种生成方式。
- **方案**：
    1. 在 `vaspflow/fermisurface/vasp_fermisurface.py` 中，将 `generate_kpoints()` 的 `vaspkit` 交互输入固定为 `261` 和 `kmesh`。
    2. 删除旧的 `102 -> 2 -> kmesh` 语义，不增加回退或自动判断分支。
    3. 在 `test_auto/MgB2/test_vaspflow.py` 中新增测试，使用 `monkeypatch` 截获 `subprocess.Popen`，显式断言发送给 `vaspkit` 的输入为 `261\\n<kmesh>\\n`。
- **状态**：已完成。

### [2026-04-08] --submit 提交路径修复与 dummy 测试覆盖
- **需求**：
    1. 全面检查 `vaspflow`、`qeflow`、`wannier90flow` 中 `-s/--submit` 的接线是否正确，避免出现 `sbatch` 后脚本名为空的情况。
    2. 修复 `vasp_scf_converge -s` 提交失败的问题：当前它会生成各扫描目录，但最终调用成 `sbatch `，说明脚本名没有正确传入。
    3. 由于测试环境没有真实 Slurm，需要增加 dummy/mock 模式测试，验证 `sbatch <script>` 的参数是否正确。
- **方案**：
    1. 审计所有使用 `add_run_submit_arguments()` 与 `handle_job_action()` 的入口，确认大部分 CLI 已正确传入 `run_vasp.sh`、`run_qe.sh`、`run_wannier90.sh`、`run_postw90.sh`。
    2. 对 `vasp_scf_converge` 单独修复：在参数根目录下生成总脚本 `submit_all.sh`，并由该脚本逐个进入各扫描点目录。
    3. `vasp_scf_converge -s` 不再直接尝试提交空脚本名，而是提交根目录的 `submit_all.sh`。
    4. `submit_all.sh` 在各扫描点目录中直接运行 `[vasp].executable_path`，不再二次 `sbatch run_vasp.sh`。
    5. 在测试层用 `monkeypatch/mock` 模拟 `handle_job_action` 或 `submit_script`，验证：
        - `vasp_scf --submit` 传入 `run_vasp.sh`
        - `qe_scf --submit` 传入 `run_qe.sh`
        - `wannier_init --submit` 传入 `run_wannier90.sh`
        - `wannier_post --submit` 传入 `run_postw90.sh`
        - `vasp_scf_converge --submit` 传入 `submit_all.sh`
- **状态**：已完成。

### [2026-04-07] vasp_scf_converge 绘图与目录结构调整
- **需求**：
    1. `vasp_scf_converge` 的收敛测试目录不再额外嵌套 `vasp_scf`，改为更便于人工检查的两层结构，例如 `ENCUT/300`、`ENCUT/350`。
    2. 为 `vasp_scf_converge` 增加 `-p/--plot`，读取已有 `OUTCAR` 并生成收敛图。
    3. 收敛图不仅要显示每原子总能，还要体现相对于最高精度参考点是否已经收敛到 `1 meV/atom` 内。
- **方案**：
    1. 将 `vaspflow/scf/vasp_scf_converge.py` 的 case 目录生成逻辑改为 `参数名/参数值` 两层结构，`INCAR`、`POSCAR`、`POTCAR`、`KPOINTS`、`run_vasp.sh` 全部直接写入该目录。
    2. 新增 `-p/--plot` 独立子流程：读取 `[vasp_scf_converge]` 唯一扫描参数对应的各目录 `OUTCAR`，固定提取 `energy  without entropy`，并结合 `POSCAR` 统计总原子数后换算为每原子能量。
    3. 绘图输出文件名固定为 `converge_<参数名>.png`，默认保存在命令启动目录。
    4. 收敛图改为双子图：
        - 主图保留每原子总能 `Energy per atom (eV/atom)` 随扫描参数变化的曲线
        - 副图绘制相对最高精度参考点的 `ΔE (meV/atom)`
    5. 副图中显式绘制 `±1 meV/atom` 阈值线与收敛窗口，并标出第一个满足 `|ΔE| <= 1 meV/atom` 的参数点。
    6. 若部分扫描点缺少 `OUTCAR` 或无法提取能量，则打印警告并跳过；若所有点都无有效数据，则抛异常。
- **状态**：已完成。

### [2026-04-04] CLI 参数模板与块级配置隔离重构
- **需求**：
    1. 明确“模板”的含义：每个工作流都要有独立、可查看的默认参数模板变量，模板优先级最低，`input.toml` 只做覆盖，不再混杂默认值与用户赋值。
    2. 为 `vaspflow`、`qeflow`、`wannier90flow` 建立统一的“模板函数 + CLI 白名单过滤 + 用户配置覆盖”机制。
    3. 每个 CLI 只读取自己的参数块和共享块，不再读取其它 CLI 的参数块，避免跨模块参数互相污染。
    4. `vaspflow` 取消顶层通用 `kmesh`，不再兼容旧行为；改为每个 VASP CLI 自己的参数块里单独设置 `kmesh`，模板默认值统一为 `0.025`。
    5. 参数块命名改为“CLI 名称优先”，便于用户按命令名直接修改 `input.toml`。
- **方案**：
    1. 在 `public/config_templates.py` 中新增 CLI 配置构建公共层，包含“按允许键过滤”和“模板 + 覆盖递归合并”两步：
        - `filter_config_by_allowed_keys()`
        - `build_cli_config()`
    2. `vaspflow/templates.py` 重写为独立模板函数与允许键集合：
        - 共享块保留 `potcar_dir`、`chgcar_path`、`wavecar_path`、`[vasp]`、`[slurm]`
        - 参数块统一改为 `[vasp_scf]`、`[vasp_scf_converge]`、`[vasp_eband]`、`[vasp_eledos]`、`[vasp_fermi]`、`[vasp_cohp_uniform]`、`[vasp_cohp_fatband]`、`[vasp_locpot]`、`[vasp_charge]`、`[vasp_elf]`、`[vasp_opt]`
        - 各 VASP CLI 模板块都内置 `kmesh = 0.025`
    3. `qeflow/templates.py` 重写为：
        - 共享块 `qe`、`slurm`、`pseudo_map`
        - 参数块 `[qe_scf.*]`、`[qe_nscf.*]`、`[qe_eband.*]`、`[qe_eledos.*]`
    4. `wannier90flow/templates.py` 重写为：
        - 共享执行块保留 `[wannier90]`、`[pw2wannier90]`、`[postw90]`、`[slurm]`
        - 参数块统一为 `[wannier_init]`、`[wannier_init.projections]`、`[wannier_init.k_points]`、`[wannier_post]`
    5. 各 CLI 脚本只对自身允许键做过滤，再与自身模板合并，不再自动并入未授权参数块。
    6. 同步更新 `test_auto`、`test_manual`、README 与模板示例，确保文档、样例、代码三者一致。
- **状态**：已完成。

### [2026-04-04] vasp_scf_converge 读取边界与错误传播修正
- **需求**：
    1. `vasp_scf_converge` 只应读取共享块和 `[vasp_scf_converge]`，不能读取其它块中的列表参数，例如 `[vasp_charge].EINT` 不应影响收敛扫描。
    2. `vasp_scf_converge` 在输入生成阶段若真实失败，例如 `POTCAR` 缺失，不应吞掉异常并返回成功。
    3. 收敛测试目录仍应复用 `vasp_scf` 的输入生成逻辑，但扫描参数边界必须严格受限。
- **方案**：
    1. 在 `vaspflow/scf/vasp_scf_converge.py` 中仅对 `self.base_config["vasp_scf_converge"]` 子树执行唯一列表参数识别与展开。
    2. 新增运行时配置转换逻辑：将 `[vasp_scf_converge]` 中当前测试值映射为单次运行使用的 `[vasp_scf]` 配置，再复用 `VaspSetup`。
    3. 修改 `vaspflow/scf/vasp_scf.py`，`setup()` 捕获异常后打印错误信息并继续抛出，避免“表面成功、实际失败”的假阳性。
    4. 补充并更新 `test_auto/MgB2/test_vasp_scf_converge.py`，验证显式列表、区间展开、多列表报错，以及只扫描 `[vasp_scf_converge]` 的边界。
- **状态**：已完成。

### [2026-04-03] qeflow 与 wannier90flow 模板和 executable_path 语义整理
- **需求**：
    1. 明确 `qeflow` 的 `[qe].executable_path` 语义为“启动命令 + QE bin 目录”，并在模板与说明中体现自动拼接的 `*.x`。
    2. 将 `wannier90flow/input.toml` 整理为完整模板，覆盖 init 与 post 两类配置。
    3. 在 `wannier90flow/README.md` 中明确其 `executable_path` 语义与 `qeflow` 不同，避免混淆。
- **方案**：
    1. 更新 `qeflow/input.toml` 与测试样例，统一使用 `mpirun -np 8 /home/mayuan/soft/qe-7.4.1/bin` 示例，并新增 `qeflow/README.md` 说明自动拼接规则。
    2. 参考 `test_manual/YH6/inputw90init.toml` 与 `inputw90post.toml` 重写 `wannier90flow/input.toml`，补齐 `[wannier90]`、`[pw2wannier90]`、`[postw90]`、`[slurm]` 与后处理参数块。
    3. 修改 `wannier90flow/README.md`，统一模板文件说明并明确各子模块 `executable_path` 应填写完整路径或完整命令。
- **状态**：已完成，后续已在 [2026-04-04] 进一步统一为 `[wannier_init]` / `[wannier_post]` 命名。

### [2026-04-02] executable_path 语义修正与运行结果校验
- **需求**：
    1. 修正 `vaspflow` 中将 `[vasp].executable_path` 误当作“可执行文件路径”处理的问题，明确其语义是“完整执行命令”。
    2. 全面检查 `qeflow` 与 `wannier90flow` 中 `executable_path` 的处理方式，按软件分类保持一致语义。
    3. 修复 `--run` 时“脚本表面成功但程序未真正启动”仍返回成功的问题，确保运行失败时抛出异常。
- **方案**：
    1. 在公共层新增命令解析工具：`vasp` 原样执行完整命令，`qe` 将“前置命令 + bin 目录”解析为具体 `*.x` 命令，`wannier90` 保持子模块独立完整命令。
    2. 重构公共执行模块，保持 Slurm 脚本不过度拦截 VASP 标准输出错误，不在 Slurm 头中统一插入 `set -e`。
    3. 为 `vaspflow` 各运行入口增加关键输出文件校验，仅当本次运行实际生成目标文件时才视为成功；若未生成关键输出则抛出异常，并补充对应测试。
- **状态**：已完成。

### [2026-04-03] vasp_scf_converge 入口恢复
- **需求**：
    1. 恢复 `pyproject.toml` 已注册但缺失源码的 `vasp_scf_converge` CLI。
    2. 仅允许配置文件中存在一个列表参数，支持显式枚举与起止点/步长两种模式。
    3. 按“参数名/参数值”目录结构批量生成输入文件，并复用现有 `vasp_scf` 逻辑。
- **方案**：
    1. 新增 `vaspflow/scf/vasp_scf_converge.py`，负责唯一列表参数识别、测试值展开和目录调度。
    2. 每个测试值通过覆写内存中的配置并复用 `VaspSetup.setup()/execute()` 生成或执行对应 SCF 任务。
    3. 增加 API 与 CLI 测试，覆盖显式枚举、区间展开和多列表报错场景。
- **状态**：已完成，后续已在 [2026-04-07] 明确为两层目录结构并补充绘图能力。

### [2026-04-01] vaspflow 参数命名收敛与模块级 kmesh
- **需求**：
    1. 删除 `vaspflow` 中旧参数兼容入口，仅保留新的参数块命名方式。
    2. 初始阶段保留顶层 `kmesh` 作为默认值，并允许各功能模块用自身 `kmesh` 覆盖。
    3. 模块级 `kmesh` 仅用于脚本逻辑，不写入 `INCAR`。
- **方案**：
    1. 移除 `incar_params`、`band_params`、`dos_params`、`fermi_params` 兼容逻辑。
    2. 统一由 `scf_params` 与各功能块读取参数，并增加模块级 `kmesh` 查找函数。
    3. 更新示例配置，明确顶层与模块级 `kmesh` 的优先级。
- **状态**：已完成，后续已在 [2026-04-04] 进一步收紧为“删除顶层 `kmesh` 且不再兼容”。

### [2026-04-01] 统一运行与 Slurm 提交方式
- **需求**：
    1. 为各工作流脚本新增 `-s/--submit`，用于提交已生成的 Slurm 脚本。
    2. `-r/--run` 与 `-s/--submit` 必须互斥，只能二选一。
    3. 将重复的脚本生成、直接运行、Slurm 提交逻辑抽象为公共模块，减少重复代码。
- **方案**：
    1. 新增公共任务模块，统一处理运行脚本写出、本地执行、`sbatch` 提交以及 CLI 互斥参数。
    2. 将 `vaspflow`、`qeflow`、`wannier90flow` 中主要工作流脚本接入公共模块。
    3. 对 `wannier_post` 的多投影场景单独处理，提交脚本改为重新调用 `wannier_post -r`，避免依赖临时改写的 `.win` 状态。
- **状态**：已完成。

### [2026-04-01] vasp_scf 收敛测试脚本
- **需求**：
    1. 在 `vaspflow/scf/` 下新增 `vasp_scf_converge.py`，用于 SCF 参数收敛性测试。
    2. `input.toml` 中仅允许一个参数写为列表，支持显式枚举与起止点/步长两种模式。
    3. 按参数名建立父目录，按每个测试值建立子目录，并生成对应运行文件。
- **方案**：
    1. 复用 `vasp_scf.py` 的输入生成与执行逻辑，仅新增参数扫描与批量目录调度层。
    2. 自动识别唯一列表参数，`len == 3` 时优先按起止点/步长展开，无法构成合法区间时回退为显式三点枚举。
    3. 在 `pyproject.toml` 注册 `vasp_scf_converge` 入口，并在示例配置中补充用法注释。
- **状态**：已完成，后续目录与绘图行为已在 [2026-04-07] 进一步收敛。

### [2026-03-31] vaspflow 输入参数拆分为 scf 与功能参数
- **需求**：
    1. `input.toml` 中的参数区分为基础 `scf` 参数与各功能参数。
    2. 功能参数覆盖 `eband`、`charge`、`eledos`、`elf`、`fermisurface`、`locpot`、`opt` 等模块。
- **方案**：
    1. 新增公共配置读取模块，统一处理 TOML 加载与参数合并。
    2. 基础 INCAR 参数改由 `[scf_params]` 提供，各功能模块读取对应 `*_params` 块并覆盖基础参数。
    3. 兼容旧配置块名，如 `[incar_params]`、`[band_params]`、`[dos_params]`、`[fermi_params]`。
- **状态**：已完成，后续已在 [2026-04-04] 收敛为按 CLI 命名的新参数块，如 `[vasp_scf]`、`[vasp_eband]` 等。

### [2026-02-09] vasp_charge_diff 输出格式调整
- **需求**：
    1. `_write_vasp_charge` 以每行 10 个数写出电荷密度。
    2. 数值使用定点小数格式并对齐输出。
- **方案**：
    1. 将每行输出数量由 5 改为 10。
    2. 使用 `{value:12.4f}` 的格式写出电荷密度。
- **状态**：已完成。

### [2026-02-09] vasp_process_locpot 参数合并与多文件电荷密度
- **需求**：
    1. 删除 `--poscar/--locpot` 开关，默认使用当前目录 `POSCAR/LOCPOT`。
    2. 合并 `--parchg/--chgcar` 为 `--chg`，允许多个文件输入。
    3. 自动判断按 CHGCAR/PARCHG 读取与缺失处理，图中 label 使用文件路径名。
- **方案**：
    1. `--chg` 支持多文件并自动探测缺省 `PARCHG/CHGCAR`。
    2. 读取并校验网格一致性后统一按密度输出曲线。
    3. 绘图与数据表头使用文件路径作为标注。
- **状态**：已完成。

### [2026-02-09] vasp_process_locpot 未指定 --chg 时不绘制电荷密度
- **需求**：未指定 `--chg` 时不再自动探测 `PARCHG/CHGCAR`，图中仅绘制 LOCPOT。
- **方案**：仅在提供 `--chg` 时读取电荷密度并绘制曲线，否则跳过。
- **状态**：已完成。

### [2026-02-09] vasp_process_locpot 未指定 --chg 报错修复
- **需求**：未指定 `--chg` 时不应遍历空对象导致报错。
- **方案**：仅在 `--chg` 非空时迭代读取电荷密度文件。
- **状态**：已完成。

### [2026-02-09] vasp_opt 结构优化脚本
- **需求**：
    1. 新增结构优化脚本，支持 rv4/rv1/rvf 三种模式。
    2. 生成对应 INCAR 与 Slurm 脚本，工作目录为 `vasp_opt`。
    3. 注册脚本入口。
- **方案**：
    1. 新增 `vaspflow/opt/vasp_opt.py` 与 `__init__.py`。
    2. 按模板生成 INCAR_1~4 与 `fouropt.sh/oneopt.sh/fopt.sh`。
    3. 在 `pyproject.toml` 中注册 `vasp_opt` 入口。
- **状态**：已完成。

### [2026-02-10] vasp_opt 示例配置补充
- **需求**：补充 `tests/inputvasp.toml` 的 `[vasp_inputpara]` 示例配置。
- **方案**：新增结构优化所需字段示例值。
- **状态**：已完成。

### [2026-02-10] vasp_opt 配置合并至 incar_params
- **需求**：删除 `[vasp_inputpara]`，结构优化所需 INCAR 参数全部并入 `[incar_params]`。
- **方案**：`vasp_opt` 仅读取 `[incar_params]` 生成 rv4 最后一步与 rv1/rvf 的 INCAR。
- **状态**：已完成。

### [2026-02-10] vasp_opt KSPACING 由 kmesh 兜底
- **需求**：`KSPACING` 未配置时使用 `kmesh` 兜底。
- **方案**：初始实现为从顶层 `kmesh` 读取；后续已调整为从当前 CLI 参数块读取，现行为 `vasp_opt` 缺少 `KSPACING` 时使用 `[vasp_opt].kmesh` 计算。
- **状态**：已完成，旧的顶层 `kmesh` 兜底逻辑已被后续规则替代。

### [2026-02-10] vasp_opt 示例移除 KSPACING
- **需求**：示例配置仅保留 `kmesh`，不在参数块里写 `KSPACING`。
- **方案**：移除示例中的 `KSPACING`，由参数块内 `kmesh` 负责兜底。
- **状态**：已完成，当前规则见 [2026-04-04]。

### [2026-02-10] vasp_opt 生成 POTCAR
- **需求**：结构优化脚本参照 `vasp_scf.py` 自动合并生成 POTCAR。
- **方案**：按 POSCAR 元素顺序从 `potcar_dir` 合并写入 `vasp_opt/POTCAR`。
- **状态**：已完成。

### [2026-02-10] vasp_opt 支持 -p 覆盖 PSTRESS
- **需求**：增加 `-p` 参数覆盖 `PSTRESS`，并按压力目录创建 `vasp_opt`。
- **方案**：`-p` 为整数时将目录设为 `{p}/vasp_opt`，未指定则使用 `PSTRESS` 并在 `vasp_opt` 创建。
- **状态**：已完成。

### [2026-02-10] vasp_opt 执行命令简化
- **需求**：运行命令仅使用 `vasp_execmd`，不再附加 `vasp_std`。
- **方案**：更新 rv4/rv1/rvf 脚本中的执行行。
- **状态**：已完成。

### [2026-02-10] vasp_opt 清理执行日志重定向与尾随空格
- **需求**：移除 `> vasp.log 2>&1` 重定向，并清理脚本尾随空格。
- **方案**：统一 `submit.write` 输出，去除尾随空格与日志重定向。
- **状态**：已完成。

### [2026-02-10] vasp_elf ELF 计算脚本
- **需求**：
    1. 新增 ELF 计算脚本并支持 `--run` 执行。
    2. 固定 `LELF = .TRUE.`、`LWAVE = .FALSE.`、`LCHARG = .FALSE.`。
    3. 生成 POTCAR/KPOINTS 与运行脚本。
- **方案**：
    1. 新增 `vaspflow/elf/vasp_elf.py` 与 `__init__.py`。
    2. INCAR 基于固定模板并允许 `[incar_params]` 覆盖。
    3. 在 `pyproject.toml` 注册 `vasp_elf` 入口。
- **状态**：已完成。

### [2026-02-11] vasp_elf 拷贝 CHGCAR/WAVECAR
- **需求**：像 eledos 一样在 `vasp_elf` 内拷贝 `CHGCAR/WAVECAR`。
- **方案**：从 `chgcar_path` 与 `wavecar_path` 读取并拷贝。
- **状态**：已完成。

### [2026-02-11] vasp_elf ELF 公式说明文档
- **需求**：整理 ELF 计算公式与实现细节说明文档。
- **方案**：新增 `vaspflow/elf/README.md`，记录公式与源码位置。
- **状态**：已完成。

### [2026-02-11] vasp_opt 默认优化参数
- **需求**：`IBRION/ISIF/EDIFF/EDIFFG` 未配置时使用默认值。
- **方案**：在 `vasp_opt` 中对缺失参数设置默认 `IBRION=2`、`ISIF=3`、`EDIFF=1e-7`、`EDIFFG=-0.01`。
- **状态**：已完成。

### [2026-02-09] vasp_process_locpot 多点剖面分析
- **需求**：
    1. 支持多点路径抽样 LOCPOT/PARCHG/CHGCAR 的 1D 剖面。
    2. 支持分数/笛卡尔坐标与路径采样步长或点数。
    3. 输出数据文件、曲线图与几何路径图。
    4. 无点输入时保留 LOCPOT min/max 简化模式。
- **方案**：
    1. 复用 POSCAR 头解析与体数据读取逻辑。
    2. 实现路径重采样与三线性插值采样。
    3. 生成 `*.dat`、`*.png` 与 `*_geom.png` 输出。
    4. 无点输入时仅输出 LOCPOT min/max。
- **状态**：已完成。

### [2026-02-07] vaspflow skill（locpot 工作流）
- **需求**：
    1. 设计通用 vaspflow 技能，适配新增 `vasp_locpot` 子功能。
    2. 目录结构与脚本骨架对齐 `vaspflow/scf`。
    3. 明确配置字段、输出文件与命令入口规范。
- **方案**：
    1. 在 `skills/vaspflow/` 创建 `SKILL.md`。
    2. 定义目录模板、脚本方法清单、locpot 关键参数建议。
    3. 说明 input.toml 配置字段与输出规范。
- **状态**：已完成。

### [2026-02-07] vaspflow 概览 PPT
- **需求**：
    1. 基于 `vaspflow` 代码生成面向新入组学生的中文概览 PPT。
    2. 覆盖整体结构、模块职责、配置文件与运行方式。
    3. 输出放在 `vaspflow/` 目录下。
- **方案**：
    1. 读取 `vaspflow` 目录结构与各模块脚本概要。
    2. 按模块与通用流程组织 10+ 张 slides。
    3. 生成 `vaspflow_overview.pptx`。
- **状态**：已完成。

### [2026-02-07] vasp_locpot 脚本与配置
- **需求**：
    1. 新增 `vasp_locpot.py`，支持拷贝 SCF 的 `CHGCAR/WAVECAR`。
    2. 默认写入 `LVHAR = .TRUE.`，网格参数从配置读取。
    3. `input.toml` 增加 `[locpot_params]`，`pyproject.toml` 增加脚本入口。
- **方案**：
    1. 新增 `vaspflow/locpot/vasp_locpot.py` 与 `__init__.py`。
    2. `locpot_params` 覆盖 INCAR 网格参数（无配置则不写入）。
    3. 注册 `vasp_locpot` 脚本入口。
- **状态**：已完成。

### [2026-02-07] vasp_charge 能量范围电荷密度
- **需求**：
    1. 新增能量范围电荷密度脚本 `vasp_charge.py`。
    2. 默认写入 `LORBIT=11`、`LPARD=.TRUE.`。
    3. `EINT` 与网格参数由 `[charge_params]` 提供。
    4. 拷贝 SCF 的 `CHGCAR/WAVECAR`。
- **方案**：
    1. 新增 `vaspflow/charge/vasp_charge.py` 与 `__init__.py`。
    2. `input.toml` 增加 `[charge_params]`（含 `EINT` 与网格参数）。
    3. `pyproject.toml` 注册 `vasp_charge` 入口。
- **状态**：已完成。

### [2026-02-07] vasp_charge EINT 数组写入格式
- **需求**：`EINT = [8, 16.6861]` 读入后写为 `EINT = 8 16.6861`。
- **方案**：在 `vasp_charge.py` 中对 `incar_params.EINT` 做数组转空格字符串的处理，并移除 `charge_params` 读取。
- **状态**：已完成。

### [2026-02-07] vasp_charge 目录名按 EINT
- **需求**：根据 `EINT = [emin, emax]` 自动设置工作目录为 `vasp_charge___emin___emax`。
- **方案**：在 `vasp_charge.py` 初始化阶段读取 `incar_params.EINT`，若为数组则按三位小数格式更新 `work_dir`。
- **状态**：已完成。

### [2026-02-07] vasp_process_locpot 脚本入口
- **需求**：在 `pyproject.toml` 中新增 `vasp_process_locpot` 入口。
- **方案**：将入口指向 `vaspflow.locpot.vasp_process_locpot:main`。
- **状态**：已完成。

### [2026-02-07] vasp_charge_diff 电荷密度差分
- **需求**：
    1. 新增 `vasp_charge_diff`，计算电荷密度差 A - B。
    2. 同时支持 `PARCHG` 与 `CHGCAR`。
    3. 头信息/坐标/网格必须一致，否则报错。
    4. 输出文件名默认 `A_minus_B.vasp`。
- **方案**：
    1. 新增 `vaspflow/charge/vasp_charge_diff.py`。
    2. 读取两份电荷密度并逐点相减。
    3. 在 `pyproject.toml` 注册脚本入口。
- **补充**：兼容未包含元素行的 VASP 头信息，自动识别原子数行与坐标起始行。
- **补充**：写出数值时加空格分隔，避免连写。
- **状态**：已完成。

### [2026-01-30] postw90 多投影自动执行
- **需求**：
    1. `inputw90post.toml` 中出现多个 `dos_project` 时，循环执行多次 `postw90.x`。
    2. 每次输出按 `seedname_dos_1_5.dat` 形式重命名，避免覆盖。
    3. 支持 `dos_project = ["1:5", "6:11"]` 列表写法。
    4. 支持 `dos_project = { Y_d = "1:5", H_p = "6:11" }` 字典写法。
- **方案**：
    1. `wannier_post.py` 读取 `inputw90post.toml`，合并重复 `dos_project` 为列表。
    1. 允许 `dos_project` 直接为列表，按列表顺序逐次执行。
    2. 逐个投影写入临时 `.win`，执行 `postw90.x`，重命名输出文件。
    3. 运行结束后恢复原始 `.win` 内容。
    4. 新增 `tests/inputw90post.toml` 示例并更新文档。
    5. 字典写法使用键名作为输出文件后缀。
- **状态**：已完成。

### [2026-01-30] wannier_post 参数调整
- **需求**：
    1. `wannier_post.py` 不再支持 `-w/-p`，改为 `-i` 指定输入文件。
    2. 使用 `-i` 自动确定工作目录与前缀。
    3. 更新使用文档。
- **方案**：
    1. 解析 `-i` 为 `.win` 文件路径（或前缀），从文件名获取 prefix，从路径获取 work_dir。
    2. `run_postw90` 直接使用推断的目录与前缀执行。
    3. 更新 `wannier90flow/README.md` 示例。
- **状态**：已完成。

### [2026-01-29] Wannier90 postw90 流程拆分
- **需求**：
    1. 将 `postw90.x` 流程从 `wannier_init.py` 迁移到独立脚本。
    2. `wannier_init.py` 移除 `postw90.x` 的执行与脚本生成，避免重复。
    3. 新增 `wannier_post.py` 支持生成脚本与 `--run` 本地执行。
    4. 更新脚本入口与使用文档。
- **方案**：
    1. 新增 `wannier90flow/wannier_post.py`，读取 `[postw90].executable_path` 并生成 `run_postw90.sh`，`--run` 执行 `postw90.x`。
    2. `wannier90flow/wannier_init.py` 移除 `postw90.x` 相关逻辑与提示。
    3. `pyproject.toml` 增加 `wannier_post` 脚本入口。
    4. 更新 `wannier90flow/README.md` 与示例配置说明。
- **状态**：已完成。

### [2026-01-29] Wannier90 DOS 参数写入 .win
- **需求**：
    1. 在 `wannier90flow/wannier_init.py` 增加 `dos`、`dos_kmesh` 与 `dos_project` 参数读取。
    2. `dos`、`dos_kmesh`、`dos_project` 写入 `*.win`，并更新示例配置。
    3. `--run` 模式下补充执行 `postw90.x`。
- **方案**：
    1. 从 `[win]` 读取 `dos/dos_kmesh/dos_project`，写入 `.win` 文件。
    2. `dos_kmesh` 支持空格分隔字符串或数组输入，输出为 `dos_kmesh = 25 25 25` 形式。
    3. `dos_project` 支持字符串或数组输入，输出为 `dos_project = 1:5` 形式。
    4. `--run` 流程追加 `postw90.x`，路径从 `[postw90].executable_path` 读取。
    5. 更新 `tests/inputwannier.toml` 示例。
- **状态**：已完成。

### [2026-01-29] Wannier90 能带对比与 QE eband 输出命名调整
- **需求**：
    1. `wannier90_plot.py` 改为读取 `--qe/--w90` 目录并自动发现能带与高对称信息文件，默认当前目录。
    2. `qe_eband.py` 输出文件名调整：`filband` 为 `{prefix}_band`、`filproj` 为 `{prefix}_band_proj`，高对称点文件名为 `{prefix}__band.labelinfo.dat`。
    3. `wannier90_plot.py` 的 QE 与 Wannier90 文件匹配规则与 `.gnu` 能带格式解析。
    4. Wannier90 能带文件需按空行分段绘制，避免不同能带连线。
    5. `wannier90_plot.py` 删除 `--label` 参数，完全依赖目录自动匹配。
- **方案**：
    1. 新增目录内文件自动匹配逻辑与可选 `--label` 覆盖；标签文件优先从 Wannier90 目录查找，不存在则回退 QE 目录。
    2. `qe_eband.py` 生成输入文件时固定按前缀命名，并在写高对称点文件时使用新文件名。
    3. QE 目录匹配 `*_band.gnu`，Wannier90 目录匹配 `*_band.dat`，并兼容 `&plot`/`.gnu` 两种格式的 QE 能带解析。
    4. 解析 Wannier90 的 `*_band.dat` 时按空行拆分为单条能带并逐条绘制。
    5. 移除 `--label` 分支，标签文件仅通过目录匹配自动发现。
    6. QE/Wannier90 能带文件统一读取 `*_band.dat`，并跳过注释行。
    7. 绘图时 QE 使用虚线且置于上层，W90 置于下层；y 轴范围取 W90 能量最小值与最大值的 1.2 倍。
- **状态**：已完成。

### [2026-01-29] QE 投影能带 kpath 距离与 Wannier90 对齐
- **需求**：
    1. `qe_process_eband.py` 使用 `eband.in` 的 `CELL_PARAMETERS` 计算 kpath 距离，与 Wannier90 度量一致。
    2. 新增 `--cell` 参数（默认当前目录 `eband.in`），弃用旧的分数坐标欧氏距离算法。
    3. 输出 `f"{prefix}_band.labelinfo.dat"` 与两列能带 `f"{prefix}_band.dat"`。
- **方案**：
    1. 读取 `eband.in` 的晶胞参数，构造倒格子度量矩阵并计算 `kpath_dist`。
    2. 生成 `*_band.labelinfo.dat` 仅记录 `eband.in` 高对称点坐标与距离。
    3. 额外输出两列能带数据文件，按能带分段写入，并统一去除 `_band_proj` 前缀用于 `*_band.dat` 与 `*_band.labelinfo.dat` 命名。
- **补充**：读取 `K_POINTS crystal_b` 权重，跳转点（权重=1）不计入距离，以对齐 Wannier90 的 `kpath_dist`。
- **补充**：若 `K_POINTS` 权重与 bands 文件点数不匹配，则改用“步长突变阈值”自动识别跳转点。
- **补充**：KList 由 `K_POINTS crystal_b` 插值生成，能量仅从 `*_band` 读取，并校验生成点数与 nks 一致。
- **状态**：已完成。

### [2026-01-29] QE eband 高对称路径不连续处理
- **需求**：
    1. 解析 `KPATH.in` 识别连续路径段并输出每段路径。
    2. 生成 `*_band.labelinfo.dat` 时，若路径不连续则用上一段终点替换下一段起点，距离连续累积。
- **方案**：
    1. 按两行一段解析 `KPATH.in`，以“上一段终点坐标 == 下一段起点坐标(1e-6)”判定连续。
    2. 生成路径段日志，并在标签输出时替换不连续段起点坐标/标签。
    3. 写 `K_POINTS crystal_b` 时，对段末点设置权重为 1 以形成跳转点，且最后一个高对称点权重固定为 1。
    4. 额外输出 `{prefix}_band.labelinfo.dat`，包含索引列与与 Wannier90 一致的累计距离。
    5. `{prefix}_band.labelinfo.dat` 使用 Wannier90 的 2π 倒格子度量，`*_gun_band.labelinfo.dat` 保持 QE 2π/alat。
    6. 不连续段起点距离不累加（跳转段不计入距离），与 Wannier90 的 labelinfo 对齐。
- **状态**：已完成。

### [2026-01-29] QE 高对称路径标签文件归并
- **需求**：
    1. 将 `qe_process_eband.py` 中的 `*_band.labelinfo.dat` 输出移除。
    2. `qe_eband.py` 继续输出标签文件，但重命名为 `*_gun_band.labelinfo.dat`。
- **方案**：
    1. 删除 `qe_process_eband.py` 的 labelinfo 写入逻辑。
    2. `qe_eband.py` 将标签文件名改为 `_gun_band.labelinfo.dat`。
- **状态**：已完成。

### [2026-01-22] wannier_init 复制输入结构到工作目录
- **需求**：`-i` 指定的结构文件需拷贝到 `work_dir` 下并命名为 `POSCAR`。
- **方案**：在 `wannier90flow/wannier_init.py` 中解析结构后，复制输入文件到 `wannier90/POSCAR`。
- **状态**：已完成。

### [2026-01-22] 修复 vaspkit KPATH.in 解析
- **需求**：`parse_kpath` 需要正确解析 vaspkit 303 生成的 `KPATH.in`（两行一段）。
- **方案**：按“坐标+标签”两行一段收集点，再成对生成路径段。
- **状态**：已完成。

### [2026-01-22] QE 赝势名仅从 pseudo_map 读取
- **需求**：`ATOMIC_SPECIES` 伪势文件名应直接使用 `pseudo_map` 中对应元素的 `pseudo` 字段。
- **方案**：`qeflow/scf/qe_scf.py` 与 `qeflow/nscf/qe_nscf.py` 移除默认回退，缺失配置时报错。
- **状态**：已完成。

### [2026-01-22] 新增 QE eband 输入生成脚本
- **需求**：仿照 `qe_nscf.py` 新增 `qe_eband.py`，高对称路径通过 vaspkit 生成。
- **方案**：新增 `qeflow/eband/qe_eband.py`，调用 vaspkit 生成 `KPATH.in`，解析后写入 `K_POINTS crystal_b`；生成 `eband.in/elebanddata.in/elebandprojdata.in` 与运行脚本，并补充脚本入口。
- **状态**：已完成。

### [2026-01-22] QE 可执行路径统一为 bin 目录
- **需求**：`[qe].executable_path` 只保存到 `qe-7.4.1/bin`，各脚本自动拼接 `pw.x`、`bands.x`、`projwfc.x`。
- **方案**：`qe_scf.py`/`qe_nscf.py` 使用 `executable_path/pw.x`；`qe_eband.py` 使用 `executable_path/pw.x/bands.x/projwfc.x`，并更新示例配置。
- **状态**：已完成。

### [2026-01-20] QE K 点配置统一至 [k_points]
- **需求**：
    1. 删除 `[nscf]` 中的 K 点配置，统一在 `[k_points]` 管理 NSCF/SCF 的 K 点设置。
    2. `qe_scf.py` 需支持两种方式：`kmesh` 自动生成网格，或 `kpoints` 显式网格（SCF 直接写入 `K_POINTS automatic`）。
- **方案**：
    1. 更新 `qeflow/nscf/qe_nscf.py`，仅从 `[k_points]` 读取 `kpoints` 或 `kmesh`（`wan` 同步迁移），未配置时使用默认间距生成均匀网格。
    2. 更新 `qeflow/scf/qe_scf.py`，支持 `[k_points].kpoints` 优先，其次 `kmesh` 生成自动网格，不写显式坐标。
    3. 调整 `qeflow/input.toml`，将示例字段改为 `[k_points].kpoints`/`kmesh`，保留 `[nscf_params]` 供 NSCF 参数覆盖。
- **状态**：已完成。

### [2026-01-20] Wannier90 输入准备脚本
- **需求**：
    1. 提供脚本在 `wannier90flow` 下自动生成 `.win` 初始输入。
    2. 规划 Wannier90 使用步骤，简化流程。
- **方案**：
    1. 新增 `wannier90flow/wannier_init.py`：读取 POSCAR 与 TOML/JSON 配置，生成包含 `unit_cell_cart`、`atoms_*`、`projections`、`mp_grid`、显式 `kpoints` 及可选 `kpoint_path` 的 `.win` 文件（`kpoint_path` 通过 vaspkit 303 生成的 `KPATH.in` 自动读取）。
    2. 添加示例配置 (`wannier90flow/sample_wannier90.toml`，另有 `wannier90flow/input.toml` 可直接使用)，复用 Ce1Sc2H24 参数，演示投影、能窗与网格设置。
    3. 在 `pyproject.toml` 注册脚本入口 `wannier90_prepare`（指向 `wannier90flow.wannier_init:main`）。
- **状态**：已完成。

### [2026-01-20] Wannier90 工具参数化
- **需求**：`wannier90flow/wannier_window.py` 使用 argparse 管理输入参数。
- **方案**：改为 argparse 解析 xml 文件、模式（e/n）及对应 band_index 或能量区间。
- **状态**：已完成。

### [2026-01-20] Wannier90 能量窗口参数统一为子命令
- **需求**：`wannier90flow/wannier_window.py` 所有输入参数统一由 argparse 解析，并按模式拆分参数。
- **方案**：使用 `argparse` 子命令 `e`/`n` 分别接收 `band_index` 与 `emin/emax`，保留原有调用结构。
- **状态**：已完成。

### [2026-01-20] Wannier90 脚本入口补齐
- **需求**：在 `pyproject.toml` 中补齐 wannier 相关脚本入口。
- **方案**：在 `[project.scripts]` 新增 `wannier_window` 指向 `wannier90flow.wannier_window:main`，保留现有 `wannier_init`。
- **状态**：已完成。

### [2026-01-20] Wannier90 可执行路径与 --run 支持
- **需求**：在测试配置中补充 Wannier90 与 pw2wannier90 的可执行文件路径，并支持 `--run` 执行流程。
- **方案**：
    1. 在 `tests/inputwannier.toml` 增加 `[wannier90]` 与 `[pw2wannier90]` 的 `executable_path`。
    2. 在 `wannier90flow/wannier_init.py` 增加 `--run`，顺序执行 `wannier90 -pp`、`pw2wannier90`、`wannier90`。
    3. 运行脚本中加入 `pw2wannier90` 输入文件检查。
- **状态**：已完成。

### [2026-01-20] Wannier90 配置归并与 Slurm 脚本准备
- **需求**：将 .win 相关参数归并到 `[win]`，并增加 Slurm 脚本生成。
- **方案**：
    1. `wannier90flow/input.toml` 与 `tests/inputwannier.toml` 的 .win 参数移动至 `[win]`，读取逻辑兼容旧顶层参数。
    2. 在 `wannier90flow/wannier_init.py` 生成 `run_wannier90.slurm`，调用 `run_wannier90.sh` 执行流程。
- **状态**：已完成。

### [2026-01-20] Wannier90 bands_plot 与 kpoint_path 联动
- **需求**：避免在未提供 `kpoint_path` 时仍写入 `bands_plot = .true.` 导致 wannier90 报错。
- **方案**：
    1. `wannier90flow/wannier_init.py` 中当 `bands_plot` 为 true 且未检测到 `kpoint_path` 时自动关闭，并提示。
    2. `tests/inputwannier.toml` 默认关闭 `bands_plot`，避免无路径时报错。
- **状态**：已完成。

### [2026-01-20] pw2wannier90 输入文件自动生成
- **需求**：`--run` 时缺少 `seedname.pw2wan` 造成流程中断。
- **方案**：
    1. `wannier90flow/wannier_init.py` 自动解析/生成 `pw2wannier90` 输入文件（含 `outdir/prefix/seedname`），并兼容既有 `input_file`。
    2. `tests/inputwannier.toml` 补充 `pw2wannier90` 的 `input_file/prefix/outdir/seedname`。
- **状态**：已完成。

### [2026-01-20] Wannier90 运行脚本合并
- **需求**：合并 `run_wannier90.sh` 与 `run_wannier90.slurm`，仅保留一个带 Slurm 头的脚本。
- **方案**：`wannier90flow/wannier_init.py` 仅生成 `run_wannier90.sh`，内置默认 Slurm 头，不再生成 `.slurm` 文件。
- **状态**：已完成。

### [2026-01-20] Wannier90 运行脚本简化
- **需求**：`run_wannier90.sh` 按模板简化，直接执行命令不再定义变量。
- **方案**：在 `wannier90flow/wannier_init.py` 中直接写入 `wannier90.x` 与 `pw2wannier90.x` 命令行，并保留日志重定向。
- **状态**：已完成。

### [2026-01-20] Wannier90 本地运行打印命令
- **需求**：本地 `--run` 时打印每一步实际执行的命令，便于排查。
- **方案**：在 `wannier90flow/wannier_init.py` 中为三步命令构造并打印，再执行。
- **状态**：已完成。

### [2026-01-20] Wannier90 能量窗口参数增强
- **需求**：`wannier90flow/wannier_window.py` 支持 `-n`/`-e` 复合参数：能带范围与能量区间统计。
- **方案**：
    1. `-n` 接受 1 或 2 个能带编号，输出对应能量范围。
    2. `-e` 接受 1 或 2 个数值；2 个按能量区间统计 min/max 带数，1 个按旧行为视作 band_index。
- **状态**：已完成。

### [2026-01-20] Wannier90 自动估计能窗
- **需求**：新增 `--auto nbnd1 num_wann nbnd3` 自动确定 `dis_froz_min/max` 与 `dis_win_min/max`。
- **方案**：
    1. `dis_froz_min = dis_win_min = band(nbnd1) 最低值`。
    2. 自适应下调 `nbnd2 = nbnd1 + num_wann` 的上界，直到区间最大带数等于 `num_wann`，否则报错。
    3. `dis_win_max = band(nbnd2 + nbnd3) 最大值`。
- **状态**：已完成。

### [2026-01-21] Wannier90 自动能窗迭代打印
- **需求**：`--auto` 过程中打印每次 `nbnd2` 与对应最大带数，便于排查。
- **方案**：在自适应循环中输出 `nbnd2/dis_froz_max/max_nbnd`。
- **状态**：已完成。

### [2026-01-21] Wannier90 自动能窗边界偏移
- **需求**：`--auto` 计算的 `dis_froz_min/dis_win_min` 使用最小值减 0.02，`dis_froz_max/dis_win_max` 使用最大值加 0.02。
- **方案**：在 `wannier90flow/wannier_window.py` 中对自动能窗边界增加 ±0.02 偏移。
- **状态**：已完成。

### [2026-01-21] Wannier90 自动能窗偏移可配置
- **需求**：支持 `--auto-delta` 配置自动能窗边界偏移，默认 0.02。
- **方案**：新增 `--auto-delta` 参数并应用到 `dis_froz_min/max` 与 `dis_win_min/max`。
- **状态**：已完成。

### [2026-01-21] Wannier90 自动能窗绘图
- **需求**：`--auto` 完成后绘制能带能量范围图，并保存为 `band_ranges.png`。
- **方案**：在 `wannier90flow/wannier_window.py` 中使用 matplotlib 绘制 `1..(nbnd_win+8)` 的能带能量区间柱状图，并标出 `dis_froz_min/max` 与 `dis_win_min/max` 虚线，若缺少 matplotlib 则提示跳过。
- **状态**：已完成。

### [2026-01-21] Wannier90flow 使用方法文档
- **需求**：补充 `wannier_init` 与 `wannier_window` 的使用方法说明。
- **方案**：新增 `wannier90flow/README.md`，包含功能、示例与参数说明。
- **状态**：已完成。

### [2026-01-21] Wannier90 额外输出参数支持
- **需求**：增加 `wvfn_formatted`、`wannier_plot`、`wannier_plot_format` 并在测试配置中说明用途。
- **方案**：
    1. `wannier90flow/wannier_init.py` 写入 `wannier_plot` 与 `wannier_plot_format` 到 `.win`，并支持 `wvfn_formatted` 写入 `pw2wan` 输入。
    2. `tests/inputwannier.toml` 增加参数并写明功能说明。
- **状态**：已完成。

### [2026-01-21] wvfn_formatted 写入 .win
- **需求**：`wvfn_formatted` 应写入 `*.win` 而非 `pw2wan`。
- **方案**：从 `pw2wan` 生成逻辑移除该字段，改为写入 `*.win`，并在 `tests/inputwannier.toml` 的 `[win]` 中说明。
- **状态**：已完成。

### [2026-01-21] Wannier90 fermi 参数支持
- **需求**：支持 `fermi_energy` 与 `fermi_surface_plot` 写入 `*.win`。
- **方案**：在 `wannier90flow/wannier_init.py` 的 `.win` 输出中增加对应字段。
- **状态**：已完成。

### [2026-01-21] Wannier90 UNK 文件生成提示
- **需求**：在测试配置中标注 `write_unk` 用途，避免 `wannier_plot` 报错缺少 UNK。
- **方案**：在 `tests/inputwannier.toml` 的 `[pw2wannier90]` 中增加 `write_unk = true` 并说明用途。
- **状态**：已完成。

### [2026-01-21] bands_plot 自动生成 KPATH
- **需求**：当 `bands_plot = true` 且缺少 `KPATH.in` 时自动运行 vaspkit 303；`bands_plot = false` 时不执行。
- **方案**：在 `wannier90flow/wannier_init.py` 中检测 `bands_plot` 并调用 vaspkit 生成 `KPATH.in`，失败则报错退出。
- **状态**：已完成。

### [2026-01-23] 新增：QE eledos 输入生成脚本
- **需求**：新增 `qe_eledos.py`，基于 `qe_scf` 的自洽结果生成电子 DOS 所需输入并支持运行。
- **方案**：
    1. 新增 `qeflow/eledos/qe_eledos.py`：生成 `eletdos.in/elepdos.in`，拷贝 `qe_scf` 的 `prefix.save` 数据，支持 `--run` 执行 `dos.x/projwfc.x`。
    2. `pyproject.toml` 注册 `qe_eledos` 入口，`qeflow/input.toml` 补充 `[eledos]` 示例参数。
- **状态**：已完成。

### [2026-01-23] 新增：QE 投影能带数据处理脚本
- **需求**：新增 `qe_process_eband.py`，将 `projwfc` 投影能带文件处理为三列数据（横坐标/能量/轨道贡献），并输出所有轨道及 p/d/f 汇总。
- **方案**：
    1. 新增 `qeflow/eband/qe_process_eband.py`：解析 `*.projwfc_up/down`，按能带顺序输出各轨道与 p/d/f 汇总，生成 `<prefix>_eleband_proj.dat`。
    2. `pyproject.toml` 注册 `qe_process_eband` 入口。
- **状态**：已完成。

### [2026-01-23] 新增：QE 投影 DOS 数据处理脚本
- **需求**：新增 `qe_process_dos.py`，处理 `*.tdos` 与 `pdos_atm` 文件，输出 DOS/IDOS 表格并包含 TDOS/ITDOS。
- **方案**：
    1. 新增 `qeflow/eledos/qe_process_dos.py`：解析 `*.tdos`、`pdos_atm` 与 `projwfc`，生成 `<prefix>_dos_proj*.dat` 与 `<prefix>_idos_proj*.dat`。
    2. 输出包含同元素轨道汇总与未汇总两种版本，TDOS/ITDOS 置于最后一列。
- **状态**：已完成。

### [2026-04-23] QE split 声子工作流补齐 merge 阶段并新增 qe_merge
- **需求**：
    1. `qe_phonon_split_assign_*` 与 `qe_ph_split_assign_*` 在全部 `q_XX` split 任务完成后，必须先执行 merge，再进入 `q2r -> matdyn`。
    2. `workflow_status.json` 新增显式 `"merge"` 字段，状态固定为 `unstarted / finished / error`。
    3. 新增独立 CLI `qe_merge`，并作为 split merge 的唯一实现入口。
    4. `qe_ph_split_assign_* -s` 在 split 全部完成后，需要先本地 merge；成功后再提交 `q2r + matdyn`，失败则打印完整缺失明细并停止。
- **方案**：
    1. 在 `qeflow/phonon/workflow.py` 中新增 merge 检测、`run_merge.sh` 生成、根目录回收实现以及 split 四阶段推进逻辑。
    2. 在 `qeflow/check/checks.py` 中新增 `check_merge()`，通过 `q_index_map.json + 根目录 merge 产物` 判断 merge 状态，不再让 `q2r` 隐含承担“尚未 merge”语义。
    3. 新增 `qeflow/phonon/qe_merge.py` 并在 `pyproject.toml` 注册 `qe_merge` 入口，`-r/-s` 对齐 `qe_q2r` 等单步 CLI。
    4. 更新 `qeflow/README.md` 与 `test_auto/MgB2/test_qe_phonon.py`，覆盖 merge 状态、四阶段提交、独立 `qe_merge`、split 本地 merge 后再提后处理等场景。
- **状态**：已完成。

### [2026-01-23] 修正：qe_process_eledos 脚本入口
- **需求**：`qe_process_eledos` 入口需可用。
- **方案**：`pyproject.toml` 中将 `qe_process_eledos` 指向 `qeflow.eledos.qe_process_dos:main`。
- **状态**：已完成。

### [2026-01-22] vaspkit 303 静默运行
- **需求**：执行 vaspkit 303 时不打印输出到屏幕。
- **方案**：调用 vaspkit 时重定向 stdout/stderr 到 DEVNULL。
- **状态**：已完成。

### [2026-01-22] bands_plot 写入位置统一
- **需求**：`bands_plot` 仅由统一配置块写入，不在 `kpoint_path` 段重复写入。
- **方案**：移除 `kpoint_path` 写入中的 `bands_plot = .true.`，保留统一逻辑。
- **状态**：已完成。

### [2026-01-21] 移除误添加的子模块
- **需求**：撤销误引入的 `wannier90flow/external/wannier90` 子模块配置。
- **方案**：删除 `.gitmodules` 并移除子模块记录，清理 README 引用。
- **状态**：已完成。

### [2026-01-19] 功能扩展与规范化：Quantum ESPRESSO 支持及目录命名规范
- **需求**：
    1. 在 `qeflow` 目录下增加 `qe_scf.py`。
    2. 统一任务目录命名规范：`qeflow` 脚本使用 `qe_xxx`（如 `qe_scf`），`vaspflow` 脚本使用 `vasp_xxx`（如 `vasp_scf`）。
- **方案**：
    1. 在 `qeflow/scf/` 目录下创建 `qe_scf.py`。
    2. 更新 `vaspflow` 下所有脚本（scf, eband, eledos, cohp, fermisurface）的 `work_dir` 为 `vasp_xxx`。
    3. 更新 `qeflow/scf/qe_scf.py` 的 `work_dir` 为 `qe_scf`。
    4. 更新 `vaspflow/input.toml` 中的 `chgcar_path` 和 `wavecar_path` 引用路径。
    5. 在 `pyproject.toml` 中注册 `qe_scf` 脚本。
- **状态**：已完成。

### [2026-10-09] 用 Rust 复刻 QE ph.x 的 dyn0 不可约 q 点生成（Issue 0004 子任务）
- **需求**：现行 split 声子流程要先“跑一次 ph.x 仅生成 dyn0 再杀掉”才能拿到不可约 q 点列表，打断点且不流畅。要求把 ph.x 生成 dyn0 的算法单独抽取，用 Rust 重写，使生成的 q 点**数量与坐标（含精确小数位）与 ph.x 字节级一致**，从而跳过 ph.x 的 dyn0 阶段，从 scf 一个脚本即可一口气推进到各分 q 点计算。
- **方案**：
  1. 阅读 `external/q-e`：`PHonon/PH/q_points.f90`（dyn0 写出驱动）、`PW/src/kpoint_grid.f90`（Monkhorst-Pack 网格 + 对称折叠）、`Modules/cryst_to_car.f90`（`cryst_to_cart`）。
  2. 关键结论经真实 `test_auto/MgB2/.../Mg1B2.dyn0` 字节级验证：(a) dyn0 坐标为**笛卡尔坐标**（tpiba），由 `cryst_to_cart` 得到，并非晶体坐标；(b) `kpoint_grid` 内部 `NINT` 为**半数远离零**取整（与 Rust `f64::round()` 一致）；(c) 字段格式为 Fortran `(3i4)/(i4)/(3e24.15)`，`E24.15` 用半数取偶舍入。
  3. 在 `qeflow-rust/crates/qf-phonon` 新增 `xml.rs`（解析 `data-file-schema.xml` 的倒格矢/对称矩阵/time_reversal）、`qgrid.rs`（复刻 `kpoint_grid`）、`dyn0.rs`（`E24.15` 格式化器 + dyn0 写出 + `q_index_map.json` 写出）、`bin/gen_dyn0.rs`（CLI `qf-gen-dyn0`）。
  4. 同时产出 `q_index_map.json`（含**晶体坐标** q_crystal），作为拆分步骤的规范来源，避免下游 coords 模式把 dyn0 的笛卡尔坐标误当晶体坐标。
  5. 适配 `qeflow/phonon/split_prepare.py`：优先从 `q_index_map.json` 读取晶体坐标，缺失时回退旧逻辑，并避免覆盖 Rust 产出的规范文件。
- **验证**：用 Python 预言机（`qeflow-rust/tools/validate_dyn0.py`）复刻完全相同算法 + `E24.15` 格式，对真实 `MgB2.dyn0` 逐字节比对通过（`BYTE-EXACT MATCH`）。本机未安装 Rust 工具链，Rust 代码经人工复核 + 预言机证明算法/格式正确，构建命令为 `cargo build -p qf-phonon --bin qf-gen-dyn0`。
- **收尾**：已在本机 `cargo build -p qf-phonon --bin qf-gen-dyn0` 通过，Rust 产出的 `Mg1B2.dyn0` 与真实 `Mg1B2.dyn0` **字节级一致**；已在 `split_workflow.py` 用 `run_dyn0_via_rust` 替代 `prepare_ph_nosplit_step(dyn0_only=True)+run_ph_to_dyn0`，两个 split CLI（`qe_phonon_split_index/coords`）与 standalone `qe_phonon --dyn0` 亦切换为 Rust；`split_prepare` 从 Rust 产出的 `q_points.json` 读取晶体坐标（拆分流 `dyn0` 状态仍由 `prefix.dyn0` 非空判定）。Rust 二进制可通过 `[qe_phonon].qf_gen_dyn0` 或环境变量 `QF_GEN_DYN0` 或 PATH 定位。
- **状态**：已完成。

### [2026-10-14] 新增 epwpy 外部子模块
- **需求与授权**：用户要求将 `https://gitlab.com/madegan/epwpy` 放入仓库 `external/`。
- **实施**：以 Git 子模块形式登记为 `external/epwpy`，远程地址保持 HTTPS，固定当前上游提交 `b54bb53d4f5de665ed08958cb6194be8efdf645d`。
- **验证**：`.gitmodules`、子模块 gitlink 与工作区状态已核对；未纳入本次操作前已存在的其他未提交修改。
