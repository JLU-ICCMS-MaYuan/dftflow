# Issue 0006：QE 分 q 点声子一次提交完成

类型：type-feature（Gitee 沿用合法 type-feature 标签）。线上由用户自管，不创建或关闭线上 Issue。

## 原始需求记录

> 不需要2 套网格计算，一套网格 scf 就行。我希望你能把这整个流程都写成一个提交脚本，只要一提交，就可以自动全部跑完后面的全部流程。并且我需要你提供分 q 点的模式，而且是两种分 q 点的模式。你需要查看 python 那个分支的设计。使用 issue 和 spec 来实现
> 一次提交，多个关联作业 (Recommended)
> 只算一次并复用 (Recommended)
> 包含真实计算验收 (Recommended)
> 安装[https://github.com/JLU-ICCMS-MaYuan/Big-Project-Skill](https://github.com/JLU-ICCMS-MaYuan/Big-Project-Skill)这个 skill 到系统里面。然后用这个 skill 来实现这个需求。


> PLEASE IMPLEMENT THIS PLAN:
> # Issue 0006：QE 分 q 点声子工作流一次提交完成
>
> ## 目标与设计依据
>
> 实现以下完整链路，用户只需执行一个提交入口：
>
> ```text
> 一次 SCF → 声子初始化与 q 点列表
> → 各 q 点独立作业
> → 自动等待全部成功
> → 合并 → q2r → matdyn 声子谱与 DOS → 收集与绘图
> ```
>
> 已确认采用多个关联的 Slurm 作业，并将一次 SCF 的结果复制到各 q 点目录，不再逐 q 点重复 SCF。
>
> 设计参考 `origin/python` 的提交 `7a172fed`。保留其两种拆分模式、数字目录、索引映射及合并方式；补齐旧版需要手动再次提交后处理的缺口。显式坐标模式按 QE 7.4.1 的坐标约定修正，不照搬旧实现中的坐标命名错误。
>
> ## 实现与接口
>
> ### 两种模式及统一入口
>
> - **index**：`ldisp=true`，各目录设置 `start_q=last_q=i`，计算指定序号。
> - **coords**：`ldisp=false`，输入对应 q 点的笛卡尔坐标，单位为 `2π/a₀`。
> - 保留 `qe_phonon_split_index`、`qe_phonon_split_coords`。不带执行参数时只准备；`-s` 改为自动安排完整链路；`-r` 在已有计算资源中顺序完成完整链路。
> - 为 `qe_split_submit` 增加 `--chain`，供前置作业提交 q 点及后处理；其原有独立提交用法保留。
> - 算例准备入口支持 `nosplit|index|coords`，默认 `index`。每种模式使用独立运行目录，避免混用结果。
> - 每个模式生成一个 `submit_phonon.sh`。分 q 点模式通过 `bash submit_phonon.sh` 启动完整提交链；手册明确入口脚本与内部 Slurm 作业脚本的区别。
>
> ### SCF、初始化和数据隔离
>
> - 前置作业完成 SCF，检查收敛、正常终止及保存文件，再保存完整 SCF 数据快照，包括波函数及相关文件。
> - 用 QE `ph.x` 的 `only_init=true`、`ldisp=true`、`lqdir=true` 正常完成初始化，从实际产生的 `dyn0` 获取不可约 q 点及其顺序；不以强制终止 MPI 的方式获取列表。
> - 映射同时保存序号、晶体坐标和 QE 笛卡尔坐标。index、coords 使用同一份 q 点列表。
> - 各 q 点复制 SCF 快照到自己的 `outdir`，只运行 `ph.x`；禁止共用可写的保存目录。
> - 子任务显式关闭初始化专用选项。修改现有任务状态判断，使其支持“复用 SCF、仅执行 PH”，并识别旧版包含逐 q 点 SCF 的目录。
>
> ### Slurm 自动依赖与失败处理
>
> - 用户提交入口首先安排前置作业。前置作业完成输入物化后，提交所有 q 点作业及一个合并后处理作业，不在计算节点占用资源轮询等待。
> - q 点作业依赖前置作业成功；后处理依赖前置作业和全部 q 点成功。使用 `afterok` 和无效依赖取消机制。
> - 使用原生 `sbatch --parsable` 提交，严格检查退出码和作业号；不依赖无限重试的 `safe_sbatch`。
> - 保存阶段与 q 点作业号，重复提交前查询已有作业，避免重复计算。完成项跳过，失败项允许再次执行入口续算。
> - 任一步失败均不继续下游，不把提交成功当作计算完成。部分提交失败时保留已提交记录，并报告具体阶段。
>
> ### 合并与后处理
>
> - 按配置中的实际 `outdir` 定位 `_ph0`，修正当前合并代码对 `tmp/` 的假设。
> - 合并前检查全部 q 点正常结束、动力学矩阵存在且对应正确的 q 点；coords 输出按映射规范化为 `prefix.dyn<i>`。
> - 正确处理各模式的 q 目录和 patterns 文件，避免重复嵌套复制或覆盖其他 q 点的文件。
> - 合并成功后依次运行 q2r、两项 matdyn、收集和绘图，修复后处理命令读取工作目录、prefix 和配置路径的不一致。
> - 全链路使用绝对配置路径、明确工作目录和失败退出检查；只有全部输出验证成功才标记完成。
>
> ## Issue、Spec 与文档
>
> 新建 `docs/specs/0006-qe-phonon-submit-chain/`，包含：
>
> - `issue-draft.md`：用户原始需求、确认选择、Python 参考版本及与 0004 的关联。
> - `spec.md`、`plan.md`、`tasks.md`：接口、数据衔接、作业依赖、恢复行为及验收标准。
> - `acceptance.md`：自动测试和实际作业证据，未通过项如实留待办。
>
> 沿用现有约定：线上 Gitee Issue 由用户管理，本次准备本地草稿，不操作线上 Issue。
>
> 更新 MgB₂ 三台机器的手册、qeflow 手册及 AGENTS.md。sugon-amd 使用 QE 7.4.1、`amd7513`、单节点 64 MPI 进程；inspur、chem 保留待填写机器配置。真实路径及运行证据只存本地忽略目录。实现沿用 Rust 和 Bash，完成后按仓库规范提交。
>
> ## 测试与验收
>
> - **自动测试**：使用模拟 QE/Slurm 验证一次入口安排完整链路、SCF 只执行一次、依赖顺序、作业号解析、部分提交失败、q 点失败阻断后处理及重复提交不重复运行。
> - **数据测试**：验证快照复制隔离、两种模式映射一致、非正交晶胞坐标转换、实际 outdir 合并、旧目录兼容，以及配置和脚本从不同工作目录启动。
> - **真实验收**：已获授权，自动测试通过后，在 sugon-amd 提交 index、coords 两套 MgB₂ 小网格流程，并用 nosplit 同参数结果作对照。
> - 核对全部动力学矩阵、声子谱与 DOS CSV、独立图及联合图；记录作业号、程序版本、输入哈希和 SCF 执行次数。
> - 数值标准提前写入 Spec：对应 q 点及谱线频率差不超过 `0.1 cm⁻¹`，总 DOS 积分相对差不超过 `10⁻³`；不得在失败后临时放宽。该验收验证工作流，不代替材料参数收敛研究。
>
> 验收通过后更新本机安装的 Rust 程序并核对入口。现有运行目录先保留归档，新模式使用独立目录，不覆盖已有计算。

## 目标与边界

一次 SCF、多关联 Slurm 作业、index/coords、自动合并和绘图；详细需求见 [spec.md](spec.md)，技术方案见 [plan.md](plan.md)。关联 [0004](../0004-dftflow-rust/spec.md)。

## 验收

SC-001～SC-004，结果见 [acceptance.md](acceptance.md)。

## Documentation Impact

- [x] 更新 qeflow 与根 README、examples 手册。
- [x] 更新 docs/overview/dftflow-rust.md。
- [x] 完成本 Feature 文档、测试与真实计算证据。
- [ ] 远程 Issue/Spec 双向链接：待用户自建与自行推送后验证。

## 补充原始需求：dyn0 工具独立化

> 如果可以的话，那么就删除这部分的代码？

用户完整需求原文：

> 初始化的关键设置是：
> ```ini
> ldisp = .true.
> only_init = .true.
> lqdir = .true.
> ```
>
> `only_init=true` 告诉 `ph.x`：完成初始化就结束，不继续计算完整声子。这个阶段会生成 `dyn0` 中的不可约 q 点列表，也可能进行初始化所需的电子态准备。&#x20;
>
>
>
> 我去这么聪明的吗？？？？？？我咋以前不知道，那我记得让你模仿 qe 写一个程序专门用来获得 dyn0，是不是也可以废弃了。如果可以的话，那么就删除这部分的代码？

澄清与执行授权原文：

> 我觉得可以保留这个小程序，但是删除把它嵌入到整个项目里面的代码。他只是作为一个独立的小程序或者脚本使用。我知道 qe_ph_nosplit有--dyn0的参数，使用这个参数，就直接调用初始化 ph的参数，然后提交任务计算即可。
>
> 使用 issue 和 spec 执行这个 plane
>
> 继续

结论：以澄清后的保留独立工具为准；用户已阅读并确认完整「dyn0 工具独立化与 QE 初始化入口统一」方案。线上 Issue 仍由用户管理。

## 补充原始需求：全部重新编译并使用最新版本

> 全部重新编译，我要新版的代码。

用户明确要求重新编译并切换本机默认命令，无需继续等待先前排队门。执行时查询到作业 1250893 实际已于 10 月 14 日正常完成，将核验真实产物后回填验收。五份主程序与独立工具均使用新的构建目录进行 release 构建；独立工具单独安装，不恢复主项目内嵌生成器。

## 补充原始需求：统一默认 outdir

> 我希望默认的 outdir为'./tmp'你帮我修改并且更新 spec，issue 和 overview

需求结论：未配置 `[qe_phonon.scf].outdir` 时默认使用 `./tmp`，与现有电子计算默认值一致；显式设置 `./out` 或其他目录时仍以用户配置为准。修改适用于新生成的 SCF、PH 初始化、两种分 q 模式及合并读取；不改写或搬迁已经完成的计算文件。新算例准备配置同步采用 `./tmp`。

追踪：Spec FR-010 / SC-007，任务 T016～T018；实现与验证证据更新至 acceptance.md。线上 Issue 继续由用户管理，本次仅更新本地草稿。

US4 实现与验证完成：32 项定向测试通过，五主程序已重新编译安装，安装后的四种声子入口验证默认 tmp 及显式 out 覆盖。具体边界见 [US4 验收](acceptance.md)；本次不重新提交真实 QE 作业。

## 补充 Bug：nosplit 缺少结构输入参数

### 原始需求记录

> qe_phonon_nosplit 为什么他没有 -i 参数，其他2 个qe-phonon-split-coords和qe_phonon_split_index 都有 -i
>
> 你使用 issue 和 spec 把这个 bug 修了
>
> 什么叫沿用 0006，又说作为独立 bug 修复。说到底还是新建一个 issue 吗
>
> ok

复用 0006，不创建新 Issue；本项性质为 Bug，不改变原 Issue 主类型。关联 [US5 Spec](spec.md)。用户已确认完整补充草案与复用方案。验收采用 FR-011/012、SC-008。

### Documentation Impact

更新本目录 Spec/Plan/Research/Tasks/验收、nosplit Overview、根 README 和 AGENTS；本项不修复其他 nosplit 全链路缺口，不安装、不提交计算，远程链接尚未验证。

本项源码已修复；SC-008 两项新测试和十四项现有回归通过，见 [验收](acceptance.md)。未更新安装制品，不关闭线上 Issue。

### 后续安装

> 更新新命令

用户授权后已更新默认命令；51 入口帮助与已安装 nosplit -i 实际输入准备通过。此前“未更新安装”的记录描述当时状态。squeue 查询报错尚未修复。

## US6：三种声子聚合入口输出精简

原始需求完整逐字记录在本地 `docs/local/0006-us6-original-request.md`（原话含机器路径，遵守仓库路径本地化规则）；公开引用如下，命令输出路径已脱敏。

> $ qe_phonon_nosplit -i POSCAR -c input.toml  -s
> [qe_phonon_nosplit] wrote run_scf.sh, run_ph.sh, run_q2r.sh (<仓库目录>/examples/MgB2/sugon-amd/run/qe_phonon_nosplit/q2r.in), run_matdyn_phband.sh (<仓库目录>/examples/MgB2/sugon-amd/run/qe_phonon_nosplit/matdyn_phband.in), run_matdyn_phdos.sh (<仓库目录>/examples/MgB2/sugon-amd/run/qe_phonon_nosplit/matdyn_phdos.in) 为什么会输出这么多？
>
> $ qe_phonon_split_coords -i POSCAR -c input.toml -s
> 已提交前置作业 1251460；后续 q 点及合并绘图将自动安排
>
> 但是这个就输出这么少。
>
> 我希望能不要输出那么多没有用的？保持一致的精简。使用 issue 和 spec
>
> 继续

用户已确认完整草案：复用 0006，FR-013/014、SC-009；准备单行、提交保留作业号、完成保留工作目录，失败保留错误。Documentation Impact：本目录 Spec/Plan/Research/Tasks/验收、Overview、AGENTS；安装状态记录本地。线上仍由用户管理。

US6 源码与 18 项测试完成，验收见 [acceptance.md](acceptance.md)。本地 Issue/Spec 保留线上未核验边界。

## 2026-10-15 追加需求（已授权实施）

### 原始需求记录

这是刚刚第一个需求的建议：首先我希望默认的计算态密度的声子网格是 ph.in里面网格的 a,b,c三个方向的2 倍。其次我希望展开宽度能稍微再明显一点，但是我不知道是该让 sigma 变大还是变小。

第二个需求的建议：确实我觉得应该让已提交作业使用提交时固定的配置，并且需要把配置文件拷贝input.toml备份到工作目录中，叫做 input-init.toml, 并且结构也要备份，叫做原本的结构名+init

> 你帮我修改输入文件为：`[4, 4, 2]`
> 我的意思是，我不知道绘图默认使用的 `σ = 10 cm⁻¹` 高斯平滑应该调大，然后让他更加平滑，是吗？

> 继续完成那 2 个追加的 issue 的需求

### 已确认结论

- FR-015：聚合入口在工作目录保存有效配置 input-init.toml 和原结构名加 -init 的结构快照；内部 POSCAR 保留兼容。
- FR-016：新作业及后续检查/后处理只读快照；外部配置和结构变化不得改变已提交链路。已有快照不得静默覆盖，续算沿用快照。
- SC-010：真实 CLI 生成脚本，修改/删除外部输入后执行 pre、q 检查及 post 仍完成原目录；篡改快照明确失败；nosplit 脚本亦固定配置。

沿用本地 Issue 草稿，不操作线上 Issue；包含实现、安装与隔离验证，不提交真实计算。输入网格已按用户要求改为 [4,4,2]。

### 追加交付结果

本地实现、43 项定向测试、安装与使用文档更新完成。详细证据见 [验收记录](acceptance.md) 的 2026-10-15 追加验收；线上 Issue 状态及远程链接由用户维护。
