# 0061 需求规格

## 原始需求记录

完整原话逐字保存在 [Issue 草案](issue-draft.md#原始需求记录)，本文件同步保存如下。

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

## 澄清结论

用户确认同层独立配置段，包含 `3.qe-bands/3.qe-dos/3.qe-pdos` 和 `8.epw-iso/8.epw-aniso`。不再要求用户区分程序名或 namelist 层级。实现授权沿用本轮配置简化请求及上述实际答复，不再重复请求许可。

## 用户故事

- US1：直接在 SCF、NSCF 等配置段修改原生参数，清楚知道影响哪次调用。
- US2：DOS、PDOS、iso、aniso 等同名参数各自设置，互不覆盖。
- US3：程序自动绑定运行目录和结构信息，仍能从实际输入追溯手填参数或内部规则。

## 功能需求

- FR-001：所有科学配置只到 `[stages."编号.用途"]`，段内不包含子表。
- FR-002：各次调用同层独立，保留原生名称、单位、数组和卡片，不增加前缀别名或科学参数继承。
- FR-003：不暴露 `workflow.derived`；内部登记允许自动绑定的字段，拒绝手填与内部绑定重复，不允许未登记生成参数通过。
- FR-004：用户可编辑的工作流设置独立于科学参数；自动选窗、来源、PH 模式、调度和搜索能力保留。
- FR-005：生成输入、配置快照、指纹、恢复和参数映射使用同一有效配置；用户诊断和来源记录指向实际平层配置位置。
- FR-006：三个计算 CLI、八阶段工作目录和既有计算行为不变；保留 schema_version=2 的原生参数版本，旧多层表达明确拒绝，不提供迁移工具。
- FR-007：改写当前 MgB₂ TOML 和测试配置，保持现有 PBE、小网格、温度与科学数值；保留 0060 的 NSCF `[A,B,C]` 网格展开和验证。

## 成功标准

- SC-001：配置序列化后每个科学段无子表，且不包含推导登记表；所有已有科学调用可生成输入。
- SC-002：分别修改 DOS/PDOS、iso/aniso、SCF/NSCF 仅影响所属输入；第七步温度独立。
- SC-003：未知参数、深层配置、自动绑定重复、网格冲突和未登记生成参数均拒绝，错误可定位到平层字段。
- SC-004：公开 CLI 的生成、提交、恢复和发布回归通过；受影响共享组件完成合理验证。
- SC-005：参数映射和实际输入可追溯；文档与实现一致，任务差异独立提交且不推送。

## 非目标

不新增物理参数、科学计算或迁移命令，不清理现有结果，不改变默认安装，不把格式测试当作新的 Tc 或科学验收。

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

追加澄清答复原文：

> 会有那个参数冲突啊？你先举例子，我再判断

当前未确认程序子段或前缀方案。只读核对 SiP₂ 原配置：第 3 步 PW 能带的 degauss=0.02，而 DOS/PDOS 为 0.007349864435130998；第 5 步 Wannier90 的 bands_plot=true，postw90 为 false；第 6 步粗网格与比较调用的 epwread/epwwrite/band_plot 分别不同；第 8 步 iso/aniso 的 liso/laniso 相反。num_iter 两处均为 500，不构成当前取值冲突。对话先给具体例子，配置结构待用户判断；不修改代码或真实 TOML。
