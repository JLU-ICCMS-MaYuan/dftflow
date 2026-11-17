# 0061：EPW 原生参数使用同层独立配置段

说明：前部保留最初平层版本的需求及验收记录；2026-10-28 已确认的八主阶段方案见本文后部追加章节，旧的“所有段禁止子表”限制已由该方案替代。

类型：`type:feature`。承接 [0059](../0059-epw-native-stage-config/issue-draft.md)，关联 [0060](../0060-epw-nscf-grid-contract/issue-draft.md)。本地暂定编号，线上 Gitee 继续由用户管理。

## 原始需求记录

> 我希望的是只区分到[stages."1.qe-scf"]就行，至于 1.qe-scf又有什么分类，我觉得就不别再细分，像[stages."1.qe-scf".pw.ELECTRONS]在1.qe-scf的基础上又细分了 2 层，太复杂了。同样的道理也应用到 2.qe-nscf, 3.qe-xxx, 4.xxx, 5.xxx, 6.xxx, 7.xxx
>
> [stages."1.qe-scf".workflow.derived]
> "pw.CONTROL.outdir" = "当前阶段工作目录和已绑定赝势源"
> "pw.CONTROL.pseudo_dir" = "当前阶段工作目录和已绑定赝势源"
> "pw.SYSTEM.ibrav" = "已绑定 POSCAR 生成 CELL_PARAMETERS 和 ATOMIC_POSITIONS"
> "pw.SYSTEM.nat" = "已绑定 POSCAR 生成 CELL_PARAMETERS 和 ATOMIC_POSITIONS"
> "pw.SYSTEM.ntyp" = "已绑定 POSCAR 生成 CELL_PARAMETERS 和 ATOMIC_POSITIONS"
> 像这种自动绑定的也不需要展示出来，就在程序内部确认即可。

> 采用同层独立配置段（推荐）

## 已确认结论

每次科学调用各用一个 `[stages."编号.用途"]`，直接填写原生参数；多个程序以及 iso/aniso 使用同层独立段，保留重复参数的独立设置。自动绑定由程序校验，用户不再填写推导登记表。此次确认替代 0059 的多层展示方式；其原生名称、单位和阶段独立性要求继续适用。用户已授权实施该调整。

## 范围与验收

需求与标准见 [Spec](spec.md)。只调整配置表达、内部解析和参数来源记录；不改变科学设置，不清理或重跑现有计算，不改默认安装。保留 0060 三整数 NSCF 网格契约；不兼容多层配置，不新增迁移工具。

## Documentation Impact

- 更新 Spec、Plan、Research、Tasks、契约、验收记录。
- 实现验证后同步 Overview、模块说明及当前 MgB₂ 算例。
- 线上状态与远程链接由用户管理，本地提交不代表线上关闭。

## 2026-10-28 追加批注与实施授权

用户选择的原说明：

> 应看到八份阶段 .log，以及保留的 imports/；旧的 history/、native-jobs/、native-progress/ 等应已撤下。

用户批注：

> 撤到哪里了？是被删除了，还是被移动到别的地方了？

用户选择的命令：

```bash
cd "/data/home/mayuan/code/dftflow/examples/SiP2"
source "/data/home/mayuan/code/dftflow/scripts/env.sh"

epw_wannier --config "epwinput.toml" --migrate-state
```

用户批注：

> 这个命令追加写到/data/home/mayuan/code/dftflow/examples/SiP2/README.md里面。这很重要

用户选择的配置段：`[stages."5.epw-wannier"]`。用户批注：

> 我挺喜欢这个名字的，我不希望 5.wannier90的名字。我原本就还是希望你能用1.qe-scf          4.qe-phonon-coords  7.epw-fine  2.qe-nscf         5.epw-wannier       8.epw-superconductivity 3.qe-electronic     6.epw-coarse      来标记 epwinput.toml里面的状态的

> 开始。

当前追加需求确认八个主阶段名与计算目录一致，不再使用独立的 5.wannier90 标记。多程序同名字段需要保留独立值，已提出“主阶段下程序子段”与“仅八个平铺段且冲突字段加前缀”的一次澄清；答复前不修改配置语法、实际 TOML 或科学设置。0064 的迁移命令文档独立推进。

追加澄清答复原文：

> 会有那个参数冲突啊？你先举例子，我再判断

当前未确认程序子段或前缀方案。只读核对 SiP₂ 原配置：第 3 步 PW 能带的 degauss=0.02，而 DOS/PDOS 为 0.007349864435130998；第 5 步 Wannier90 的 bands_plot=true，postw90 为 false；第 6 步粗网格与比较调用的 epwread/epwwrite/band_plot 分别不同；第 8 步 iso/aniso 的 liso/laniso 相反。num_iter 两处均为 500，不构成当前取值冲突。对话先给具体例子，配置结构待用户判断；不修改代码或真实 TOML。

## 八主阶段方案确认与本轮范围

用户实际答复：

> 认可你的这个需求，确实需要这个设计。

采用前文推荐的八个主阶段加用途子段：主阶段名与实际计算目录一致；SCF、NSCF 和主调用参数仍直接写在阶段内，多程序独立值以一层用途子段区分，不暴露 namelist。第 7 步可选额外谱保留具名 spectra 子任务。SiP₂ 继续原 nosplit 模式，不因命名调整切换为 coords。当前源 TOML 更新为新表达；历史配置版本和科学产物不改写，旧平层版本 2 保留严格兼容读取及原格式序列化，不接受新旧混写。

该确认替代本项早先“所有科学段一律无子表”的展示限制，原生参数、独立数值、自动绑定和同一内部科学视图的要求不变。保留科学参数逐项相等和阶段科学键相等；本轮不执行 SiP₂ 迁移、清理或科学提交。验证后编译发布四个 EPW CLI，并按任务范围提交，不推送。


本轮实现及文档影响已完成，固定 release 回归、SiP₂ 只读对照、正式发布和命令路径核对见 [验收记录](acceptance.md#2026-10-28-八主阶段配置验收)。实际 SiP₂ 状态转换与科学提交未执行；线上 Gitee 未操作，本地完成记录不代表线上 Issue 已关闭。

## 2026-11-17：公共参数与网格数组补充

### 原始需求记录（追加原文）

> ATOMIC_SPECIES = [["Mg", 24.305, "mg_pbe_v1.4.UPF"],["B", 10.811, "b_pbe_v1.4.uspp.F.UPF"]]\
> 我希望这么设置ATOMIC_SPECIES，而且这个参数是比较通过，而且这个参数是比较通用的，我希望能放在[baseparams]里面。包括\
> prefix \
> ecutwfc\
> ecutrho\
> Emin，Emax\
> 也可以放到这里\
> DeltaE\
> 都可以放进去。
>
> nq1 = 2\
> nq2 = 2\
> nq3 = 1
> 修改为 nq = [2,2,1]
>
> nk1 = 8\
> nk2 = 8\
> nk3 = 4
> 修改为 nk = [8,8,4]
>
> nkf1 = 4\
> nkf2 = 4\
> nkf3 = 2\
> nqf1 = 2\
> nqf2 = 2\
> nqf3 = 1
> 同理写为 nkf和 nqf。
>
> 网格的设置不能放到 baseparams 里面
>
> 修改之前的 issue 和 spec 完成

### 本轮需求结论

沿用本 Issue 的 `type:feature` 和 [Spec](spec.md)，不新建编号，不操作线上 Gitee。顶层新增 `[baseparams]`，允许公共 `ATOMIC_SPECIES`、`prefix`、`ecutwfc`、`ecutrho`、`Emin`、`Emax`、`DeltaE`。`ATOMIC_SPECIES` 使用 `[元素, 质量, 赝势文件]` 三元组数组。

网格保留在对应阶段或用途子段，以 `nk/nq/nkf/nqf = [整数, 整数, 整数]` 表达，禁止放入 `[baseparams]`。各阶段网格保留独立值；原生输入展开为原有带方向编号的参数。已有 NSCF `K_POINTS` 网格与路径设置保持各自职责，不改名为 `nk`。

本轮只完成需求文档补充；实现、回归和安装未完成，不沿用历史验收声称新增语法已可用。公共值与阶段同名值按用户确认采用阶段显式值优先，阶段未填写才使用公共值；展开后仍核验既有科学约束。

### 同名值规则澄清原文

> 阶段值优先，未填写时使用 baseparams（推荐）

该答复确认公共参数的取值规则；不改变各阶段独立网格要求，不代表新增解析功能已经实现。

### 补充验收与文档影响

- 验收追加 Spec 的 SC-008～011：公共参数分发、物种三元组、阶段独立网格、原生输入与科学身份等价，以及非法配置定位。
- 本轮更新本目录 Issue、Spec、配置契约、计划与任务；后续实现通过后再同步 Overview、模块说明和算例，不提前改写现状文档。
- 历史完成记录不覆盖本轮新增任务；线上状态及远程链接继续由用户管理。
