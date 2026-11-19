# 0061 验收记录

说明：前部保留最初平层版本的需求及验收记录；2026-10-28 已确认的八主阶段方案见本文后部追加章节，旧的“所有段禁止子表”限制已由该方案替代。

## 实现与配置

科学配置改为同层独立调用，原生输入段由单一映射表判定；自动绑定登记在程序内。深层配置、未知字段、别名及自动值双重定义明确拒绝。输入生成、指纹和回执仍读取同一有效配置。

当前 MgB₂ TOML 已直接改写为 21 个同层调用，328 个科学赋值逐项对照保持；SCF ecutwfc=40 Ry，NSCF 沿用已确认 0060 的 [4,4,2]，第 7 步 temps=[20]，iso/aniso 各为 [20,30,40] K。机器文件保持忽略，不强制纳入 Git。配置 SHA-256：`da2afef0f9dad80868948e054f6a86e608eecfc70edb82336e61dfaa7d1db507`。

## 验证证据

- 从 HEAD 与本任务差异构造独立提交候选，6 组针对性回归共 75 项通过、1 项既有显式跳过；包括阶段独立性、原生名称和单位、数组、卡片、冲突拒绝、深层旧配置拒绝及参数回执。候选未吸收其他会话尚未提交的 0060 实现。
- 受影响共享组件：qeflow 34 项通过；qf-input 11 项通过。第一次隔离源树缺少子模块中的官方 kmesh.pl，导致其逐字节对照测试失败；复制相同官方脚本后复测通过，脚本 SHA-256 为 `c47e454b934e05c5107d8d7687f9fef478ec775c87f3f830285557f88b3c9e91`。
- EPW 整体集成回归：308 项通过、1 项旧路径断言失败、12 项显式跳过。失败来自完整公开流程测试仍要求 `stages...workflow` 旧诊断路径；程序已正确输出 `workflow.6.epw-coarse` 与 `workflow.3.qe-bands`，仅修正测试断言并单独重跑，单独重跑 1 项通过。按用例去重为 309 项通过、12 项显式跳过；这是整套结果加失败项修复复测，未声称单次整套零失败。本次集成源树包含工作区已有 0060 实现，用于核验用户确认的简写网格接续。
- 私有 release 程序打包至 `docs/local/0061-native-bin/`；三个计算 CLI 和清理工具的帮助入口通过。使用该 epw_phonon 对实际 MgB₂ 执行 `--check -c epwinput.toml --json`，配置、输入文件、程序路径及 MPI 检查通过，workflow_complete=false。

完整原始日志在 `/tmp/dftflow-0061-isolated-tests.log`、`/tmp/dftflow-0061-shared-tests.log`、`/tmp/dftflow-0061-shared-retest.log`、`/tmp/dftflow-0061-release-verified.log`、`/tmp/dftflow-0061-public-retest.log`；本地配置审计、源码清单及只读检查在 `docs/local/0061-epw-flat-stage-config/`。打包源件及可执行件指纹见私有 bin 内两份 SHA256SUMS。

## 过程中的失败与证据边界

最初调试版回归发现旧路径断言和深层测试配置未改全，修复后独立候选测试通过；长时间运行的调试版被主动终止，不能算整套通过。首次集成 release 复用了旧源树的 Cargo 时间戳缓存，出现不存在 verified_states 的编译错误；对隔离集成源树更新时间戳后重新编译验证。保留失败与中断日志，不与最终通过日志混算。

当前工作区的 0060 修改、其未跟踪测试和 SiP₂ 文档属于其他任务，独立保留。0061 提交候选中的 NSCF 卡片沿用已提交基线；实际工作区及私有集成程序使用 0060 简写网格，二者验证分别记录。旧冻结配置不迁移，新格式不能直接恢复旧格式现场。

本轮没有清理科学目录、提交 Slurm 或执行新科学计算，没有切换默认安装、推送或操作线上 Issue。0059 的八阶段及六个温度点仍属于历史科学证据；本轮软件验证不代替科学重算、Tc 或网格收敛验收。

## 2026-10-28 八主阶段配置验收

用户认可用途子段后，在 `fce03fe` 基线上实现外部边界适配；八个父段与计算目录对应，主调用仍平铺，其他调用使用用途子段。唯一映射用于展开、导出和诊断，仍复用既有原生白名单、内部科学视图和输入生成。格式标记仅驻留内存，不进入序列化科学参数或阶段身份。历史平层版本 2 保持读取及原格式输出，`Config::to_stage_toml()` 显式导出新表达。

### 固定 release 制品回归

- 库测试 60 项通过、4 项显式跳过；其中新 SiP₂ 专项随后显式执行通过，剩余 3 项需要其他真实现场的专项未执行，不计通过。
- 历史平层配置 7 项、原生阶段配置 14 项通过，覆盖原生参数、单位、独立性、深层及未知字段拒绝。
- 公开进程测试 19 项通过：八日志协议 10 项及阶段交付 9 项。新增两项使用八主阶段配置，验证真实公开 CLI 的旧状态转换及 Wannier 提交／恢复；后者修改两处 num_iter 后仅增加一次 Wannier 调用，上游 QE／PH 调用记录不变。其余覆盖完整替身阶段链、失败恢复、仅补派生产物、路径标题和损坏证据拒绝。这些测试不启动真实科学程序或调度器。
- SiP₂ 只读专项 1 项通过：分别读取改写前副本和当前配置，按真实路径解析，内部科学视图及八个阶段科学键完全一致，SCF／NSCF 生成输入逐字节一致。不调用配置驱动的 CLI，避免触发入口自动迁移。

本轮最终固定 release 的上述用例合计 101 项通过；没有宣称整个项目所有回归或新的科学拟合验收完成。

### 真实 SiP₂ 配置与保护范围

`examples/SiP2/epwinput.toml` 仅改写 15 个表头：14 个科学调用表头及第 3 步 workflow 名。解析后逐项比较所有原生值、顶层科学设置和工作流值，全部相等。原配置 SHA-256 为 `deba4df68f6e1ffa480d402806cc4ae576b3c0bf04428e66793d61d7853df59e`，新配置为 `c6f33e305536a915da187e457b69713dfa626ccffade9bb49a6c0843270a54df`。本例继续 `4.qe-phonon-nosplit`、96 个 Wannier 函数、Si/P 的 s,p 投影、num_iter=500；其他数值逐项保留。历史编号配置不改写。

KPATH、POSCAR 和 `epw-state/imports/phonon-source.json` 的保护哈希前后相同。未执行实际 SiP₂ 状态转换、清理或科学提交；没有声称对全部大体积科学文件进行了重新哈希。README 保留既有操作段落，新增八主阶段归属说明；既有迁移命令及“旧状态直接删除、无搬迁备份”的说明保留。

### 首轮失败与修复

调试版首轮库测试 59 项通过、1 项失败、4 项跳过，平层测试 6 项通过、1 项失败。前者来自测试直接切换 PH 模式后保留非活动 index 逐 q 节点，已改为读取实际对应模式的外部配置；后者发现新增展开包装过早转换旧格式报错路径，已修复为只转换新格式的位置。两项均在最终固定 release 复测通过。首次编译另发现测试中对 Deref 视图同时可变借用多个程序字段，已改为分别赋值。旧失败日志保留，不混作通过证据。最终审查另发现通用声子后处理诊断默认映射到 nosplit，现已显式传入实际模式，并在 nosplit/index/coords 三种模式各核对错误子段位置。最终制品再次执行上述同组回归后发布，先前 release 日志及发布哈希另行保留。

原始构建、测试、配置表头对照及保护哈希在忽略的 `docs/local/0061-eight-stage-layout-20261028/implementation/`。本轮不推送、不操作线上 Gitee；线上 Issue 状态继续由用户管理。

### 正式入口发布与文档门

`bash scripts/build-native.sh build --release -p epwflow --bins --locked` 成功，四个产品已覆盖 `bin/epwflow/`。逐一核对 Cargo release 源件与 `bin/.build/published/*.json` 的源哈希、正式文件与发布哈希一致。`epw_wannier` 新发布 SHA-256 为 `9f20b093f631557c295510eb17cffa296d8bbb31a37e6911e422947f4f018366`，与原正式版本不同；源码清单在构建前固定，并在验收后再次核对未变。

加载 `scripts/env.sh` 后，四个命令均解析到本仓库 `bin/epwflow/`，四个帮助入口正常。使用当前 SiP₂ 科学设置的隔离副本，将工作目录、scratch 和账本改到含空格的独立目录；三个正式计算 CLI 跨配置目录执行 `--check --json` 均返回 configured，未调用科学程序或提交任务。该检查仅验证正式入口接受新配置，不代表隔离副本已有科学结果，也没有对实际 SiP₂ 执行入口自动迁移。

Overview 入口及参数页、模块说明、SiP₂ 算例和 0061 文档已同步；38 个 Bash 代码块语法及 100 个本地链接目标通过。历史 README 的操作段落完整保留，改写仅为首段格式说明与末尾新增说明。任务范围内修改自动提交，不推送。

## 2026-11-17：公共参数／数组实现及 Ubuntu MgB₂ 输入同步

### 已完成范围

按用户“input.toml按照我当前的 spec 和 issue 都改好了吗？帮我改一改”实施 T012～T016。本轮补齐此前仅文档定义的解析能力，并更新当前 Ubuntu 输入；未改写历史配置或启动科学计算。

- 七项 `[baseparams]` 字段只回填给原生白名单支持的调用，阶段显式值优先，非法值不回退；未知公共字段及公共网格拒绝。
- 物种二维数组在边界转换为原生卡片；网格 `nk/nq/nkf/nqf` 在所属调用展开，严格检查三正整数、适用调用及方向字段混写。来源和数组写法只作为内存元数据用于错误位置和往返。
- 修复对象插入顺序影响科学键的问题：仅在科学键序列化前递归排序对象，不排序数组、物种或点表。现行复用仍比较解析后的实际参数，不改旧记录。
- 当前输入使用八主阶段／用途子段，七项公共字段集中填写；12 MPI、各阶段科学数值、四个 Wannier 能窗与排除带保留。按 0070 删除已废弃双初值／搜索 workflow 字段，保留原生 iso／aniso 20/30/40 K 列表。

### 验证证据与限制

- 新增 `common_stage_config` 专项 4 项通过，覆盖公共回填／独立覆盖、非法值不回退、卡片和数组往返、未知及混写拒绝、原生输入、八阶段参数和科学键等价；真实 MgB₂ 对照显式用例另 1 项通过。真实用例只读取新旧 TOML，不读取计算产物或电荷密度／波函数。
- 扩大 release 回归结果为 **186 通过、92 失败、7 忽略**，未宣称全绿。固定修改前基线 `4f4d5eae` 单独复跑两个失败组，同样 **92 项失败，失败名称集合完全一致，本轮新增失败为 0**。其中平层配置旧用例仍要求显式费米能必须失败；91 项旧全链用例仍依赖自动补算上游等已改变行为。保留基线与本轮原始日志，不把失败或忽略项计为通过。
- 无失败的相关组包括库 82、原生配置 16、NSCF 网格 8、程序路径 4、原生契约 19、三程序 6，以及新增专项 4，合计 **139 通过、6 忽略**。另外两组中通过项计入上面的 186，不重复累计；未修复与本次配置表达无关的旧全链断言。
- 第一轮科学键等价专项发现 JSON 值相同而键不同，确认为对象插入顺序，排序修复后专项全部通过；失败日志保留。未以删除等价断言绕过。
- release 构建及项目发布器成功覆盖四个固定入口，逐一核对正式文件与本次 Cargo 产物一致，不创建版本副本。三个计算 CLI 对新输入的隔离 `--check --json` 全部返回 `configured`，核对 configuration、input_files、program_paths、mpi_pool_image。检查副本将输入文件路径转为绝对路径、工作目录转向临时目录，未改真实工作区状态。
- 初次正式检查发现 `repair/pp/` 已缺失。本轮恢复 0066 的同模型转换：同版 QE readvan 的官方 unformatted 读取适配、write_upf 导出及 read_ps_new 重读，Mg 的元素、泛函、价电子、径向网格、投影、局域势、增广数据和原子径向函数对照的最大归一化差为 **1.530126542096696×10⁻¹⁵**，低于 10⁻¹²；B 以相对链接引用原文件。赝势及临时转换程序均不纳入 Git；没有执行 SCF 或其它科学计算。该原子赝势对照不核验任何 PW 电荷密度或波函数。
- 证据保存在忽略目录 `docs/local/0061-input-sync-20261117/`。Spec、计划、研究、契约、任务及说明同步；保留并行 Issue 迁移和文档整理，不恢复已迁移的本地 Issue 草稿，不操作线上 Issue，不推送。

公共分发和数组展开集中在配置适配边界，复用已有调用白名单与内部生成器，不建立通用继承框架；旧科学格式和当前配置共享同一内部视图，符合 KISS／DRY。

## 2026-11-18：全局费米来源

实施范围为 FR-019～021、SC-012。公共值在现有配置适配边界回填第 5、6、7 步 workflow；保留 Interpolation 的既有第 7 步视图，不新增运行时顶层副本或 schema 版本。各阶段显式值优先，公共和阶段都缺失报错；两种合法枚举均支持，非法公共值和非法阶段值均按实际字段拒绝。

定向回归覆盖全局／阶段表达的有效配置和全部八阶段科学身份等价、历史平层与八主阶段往返、显式覆盖保留、运行时修改后序列化及缺失／非法拒绝。实际 MgB₂ 输入以前后两个版本运行 `common_stage_config::actual_mgb2_public_defaults_preserve_science_without_reading_calculation_data`，1 项通过，确认各阶段有效科学参数不变；该测试只读配置，不读取已有计算数据。

`bash scripts/build-native.sh build --release -j 2 -p epwflow --bins --locked` 成功，覆盖四个固定模块入口。逐一比较 epw_phonon、epw_wannier、epw_superconductivity、epw_clear 的 release 制品与公开文件 SHA-256，全部一致。epw_wannier 的 SHA-256 为 `a1a6050d3524dbff6cd2f9bc9b767093cf8ebd283a8f3f82501b35d3620671a3`。

发布后的 epw_wannier 使用隔离目录及当前 MgB₂ 输入的绝对路径副本执行 `--stage wannier --check --json`，返回 `operation_status=ok`；非法公共来源由同一入口拒绝，错误包含 `baseparams.fermi_source`。隔离目录为 `/tmp/dftflow-61-cli-check/`，日志在 `/tmp/dftflow-61-{tests,mgb2,build}.log`，属于本机临时证据。

构建使用当前工作树，包含原先已有的绘图与超导批处理修改；这些差异不作为本需求的提交或验收结果。此次只验证配置适配沿用现有科学链，不声称重新验证两种费米来源的物理算法。没有科学作业提交、已有运行目录 CLI 调用、历史快照改写或推送；线上 Issue 与远程 Spec 链接仍不作关闭处理。

初次全回归受到临时目录环境影响：`/tmp` 写入返回 `Disk quota exceeded`；改到仓库缓存后，仓库所在 `fuseblk` 的权限及目录删除语义又导致两个无关单元测试失败。没有修改生产代码来适配测试环境。最终使用主目录文件系统的 `TMPDIR=/home/mayuan/.cache/dftflow-issue61-tests`，同一单元测试制品完整复跑 **87 通过、0 失败、4 忽略**。最终定向配置回归 **7 通过、0 失败、1 忽略**，该忽略用例另按实际 MgB₂ 配置显式执行通过。最终单元日志为 `/home/mayuan/.cache/dftflow-issue61-unit.log`；忽略不计入通过。

完整集成回归另发现 `foundation::refuses_conflicts_and_no_science_success_on_failed_submission` 的状态来源断言过期：SCF 提交失败由 `workspace::start_stage` 写入本阶段日志的 `Stage allocation.Status = submission unknown`，原测试却查询原生超导任务列表并对不存在的记录 unwrap。修正测试为读取 SCF 阶段日志，保留提交失败、无科学执行及无旧隐藏状态文件的检查；不修改调度实现或阶段状态所有权。

原生插值回归另有旧夹具假定 `coarse_wannier` 仍在粗网格产物清单，构造篡改现场时对不存在条目 unwrap。移除该过期夹具设置，保留实际 wout 窗口篡改、细网格拒绝、无额外原生调用的检查；定向复测 1 项通过。修正后的 foundation 目标 11 项全部通过。相应日志为 `/home/mayuan/.cache/dftflow-issue61-{foundation,window}.log`。

完整回归尚未通过：`slurm_log_filenames` 的 `absent_header_does_not_invent_resource_directives`、`numbered_scripts_keep_logs_local_when_submitted_from_other_directories`、`public_submission_uses_short_names_and_refuses_duplicate_queued_jobs` 失败。单独复现首项，生成 electronic 脚本时拒绝缺失 SCF 的 `data-file-schema.xml`；属于当前独立阶段依赖与旧夹具的差异，本次不改生产检查或伪造上游完成。`generated_chain_keeps_logs_local_and_preserves_frozen_scripts` 的旧 `--all` 生成进程长时间未结束，已终止本次自有回归进程，未中断其他任务。完整日志为 `/home/mayuan/.cache/dftflow-issue61-tests-final.log`，不得把已通过的定向检查表述为完整回归通过。

按仓库“验证失败不得提交”约定，本轮不执行 git commit，不推送、不关闭 Issue；T017/T018 按定向证据完成，T019 的最终收尾仍未完成。原有未提交改动及 PDF 删除保持原状，未暂存任何文件。配置支持与发布结果不等于整个仓库验收通过。

## 完成确认后的测试收尾

用户已明确确认核心功能完成并授权关闭，前述“暂不提交”是上一轮状态。本轮 Slurm 日志目标在优化构建下 **8 项全部通过**（约 4.25 秒），包括此前长时间未结束的旧链式生成用例。调试制品约 311 MiB，链式生成多次核验程序身份；优化入口约 45 MiB，原测试保留全部脚本／程序身份检查即可正常结束，不增加超时或放宽身份核验。

两个 electronic 夹具通过已有 QE 进程替身和公开 CLI 顺序生成 SCF／NSCF 上游，再生成电子阶段脚本；不手写 XML 或完成记录。它们保留空格路径，其余六项继续覆盖含单引号路径。含单引号的 QE 原生绝对路径引用不属于本次配置收尾范围；替身按合法 Fortran 字符串解析，没有为非法引用增加容错。

SCF 公开提交断言更新为 `stage-scf.sh`。从其它目录重新 sbatch 已冻结的脚本时，显式传入新作业号 202，检查 `input_conflict: Phase script binding different jobs`、非零退出、目标目录日志和无 `scf.out`，确认真实分配绑定拒绝，避免仅因缺少 Slurm 环境而通过的假阳性。

两路只读诊断（当前 Codex 独立助手、独立 Codex CLI）核对了根因与上述修正。旧 CLI 不支持当前模型，单次调用改用 VS Code 扩展自带版本，不安装软件、不修改长期配置；诊断过程未改变工作树。遵循 KISS／DRY，仅提取夹具工作目录构造，复用现有原生替身与公开入口，保留生产检查。

最终相关验证：配置目标 7 项、平层兼容 7 项、foundation 11 项、Slurm 日志目标 8 项和实际 MgB₂ 配置对照 1 项均通过；默认忽略的 MgB₂ 对照已单独显式运行，不重复计数。窗口篡改负向回归另单独执行。完整仓库的历史失败日志保留，本次不把相关目标通过表述为全仓重新验收。

最终日志：`/home/mayuan/.cache/dftflow-issue61-{core,slurm,mgb2,window}-final.log`。重新执行固定入口 release 构建成功，四个公开文件与 release 制品 SHA-256 一致；epw_wannier 为 `12f7ffffda1f3dba4c3d6f166769e9a8c9cb11b61fd4559564133197b588d5a3`。构建含原有绘图差异，该文件及三份原有 PDF 删除不纳入本需求提交。

Documentation Impact：当前功能文档、根 README、模块 README、配置契约及 Spec 产物已同步。以登录身份 `JLU-ICCMS-MaYuan` 核验 GitHub API 返回的 `rust-master` Spec，文件确为 0061 且指回 #61；远程现有文档 SHA 为 `d444371e4e2aff80aa094964e4a43668bf03b8ed`，URL 为 https://github.com/JLU-ICCMS-MaYuan/dftflow/blob/rust-master/docs/specs/0061-epw-flat-stage-config/spec.md 。本次最新增量只提交本地，不自动推送；关闭说明明确区分已有远程 Spec 和本地最新实现。
