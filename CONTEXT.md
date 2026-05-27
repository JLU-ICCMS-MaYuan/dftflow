# DFTFlow

DFTFlow 用于组织高通量第一性原理计算工作流，并把结构替换、压力点计算和后处理结果汇总成可比较的批量结果。

## Language

**Substitution Batch**：  
一次围绕同一占位结构、同一替换规则和同一组压力点展开的高通量替换任务。  
*Avoid*：batch root, output folder, high-throughput run

**Placeholder Element**：  
输入结构中用于表达可替换位点的假元素标签。  
*Avoid*：fake atom, dummy element

**XA Series**：  
固定 **XA Placeholder** 后，由多个 **XB Placeholder** 替换结果组成的一组候选体系。  
*Avoid*：XA directory, element group

**Substituted Structure**：  
将占位元素替换为真实元素后得到的一个具体候选体系。  
*Avoid*：case, formula directory

**Pressure Point**：  
同一个 **Substituted Structure** 在指定外压条件下的一次计算实例。  
*Avoid*：pressure folder, pressure case

**Workflow Status**：  
一个 **Pressure Point** 在工作流步骤上的三态进度分类：`finished`、`error` 或 `unstarted`。  
*Avoid*：job state, running state

**Phonon Workflow Status Machine**：  
维护 QE 声子工作目录 `workflow_status.json` 的状态语义 Module，负责刷新 `scf/ph/merge/q2r/matdyn/plot` 三态、汇总 split q 点任务、判断下一续算阶段。  
*Avoid*：status helper, phonon checker, resume utility

**Split Phonon Workflow**：  
QE 声子 split 总工作流的统一编排 Module，覆盖 `split_assign_index` 与 `split_assign_coords` 两种 q 点拆分方式，并保持二者共享同一套 `scf -> dyn0 -> split -> merge -> process -> plot` 生命周期。  
*Avoid*：split index workflow, split coords workflow, split script driver

**Phonon Postprocess Workflow**：
QE 声子后处理的统一编排 Module，负责 `q2r -> matdyn_phband -> collect phband -> matdyn_phdos -> collect phdos -> plot` 生命周期，并把归档、表格解析和绘图委托给专门 Module。
*Avoid*：process helper, plotting script, post script

**Split Merge Workflow**：
QE split 声子结果回收 Module，负责按 `split_assign_index` 或 `split_assign_coords` 策略合并 dyn、dvscf、phsave 和 patterns 文件，并输出 merge 结果。
*Avoid*：merge helper, dyn copier, split collector

**QE Phonon Input Deck**：
为 QE 声子链路写出 `scf.in/ph.in/q2r.in/matdyn_*.in/run_*.sh` 的输入文件 Module。
*Avoid*：builder, input helper, file writer

**Crystal Structure Model**：
负责结构文件解析、化学式派生、kmesh 派生和晶格参数识别，为 QE/VASP/Wannier 等上层工作流提供与具体输入 deck 语法无关的晶体结构语义。
*Avoid*：structure helper, input parser, qe input structure

**QE Input Deck Writer**：
负责把配置、结构模型和计算参数写成 QE 原生 `.in` 输入 deck；按 `pw/ph/q2r/matdyn/bands/dos` 等 QE 程序拆分职责。
*Avoid*：builder, facade, setup helper

**Phonon Workspace**：
负责 QE 声子工作目录定位、配置真源解析、结构副本准备和 split 子目录发现。
*Avoid*：fileio, path helper, input resolver

**Phonon Stability**：  
一个 **Pressure Point** 基于最小声子 DOS 频率得到的二元稳定性分类：`stable` 或 `unstable`。  
*Avoid*：dynamical stability, imaginary-frequency status

**Stable Phonon Case**：  
最小声子 DOS 频率不低于 -1 THz 的 **Pressure Point**。  
*Avoid*：no imaginary frequency, passed phonon

**Unstable Phonon Case**：  
最小声子 DOS 频率低于 -1 THz 的 **Pressure Point**。  
*Avoid*：has imaginary frequency, failed phonon

**QE Calculation Setup**：  
为一个 Quantum ESPRESSO 计算目录准备可执行输入的一次生命周期，包括读取配置、解析结构、派生 prefix、准备依赖数据、委托 **QE Input Deck Writer** 写输入、写运行脚本，以及按需运行或提交。  
*Avoid*：input writer, setup class, QE service

## Relationships

- 一个 **Substitution Batch** 包含一个或多个 **XA Series**。
- 一个 **XA Series** 包含一个或多个 **Substituted Structure**。
- 一个 **Substituted Structure** 包含一个或多个 **Pressure Point**。
- 一个 **Pressure Point** 有一组 **Workflow Status**，并且最多有一个 **Phonon Stability** 结果。
- 一个 QE 声子 **QE Calculation Setup** 由 **Phonon Workflow Status Machine** 维护 `workflow_status.json` 并给出续算阶段。
- 一个 **QE Calculation Setup** 依赖 **Crystal Structure Model** 解析结构，并依赖 **QE Input Deck Writer** 生成 QE 原生输入文件。
- 一个 **Phonon Workspace** 为 **Split Phonon Workflow** 与 **Phonon Postprocess Workflow** 提供工作目录定位与配置真源解析。
- 一个 **Split Phonon Workflow** 依赖 **Phonon Workflow Status Machine** 判断当前阶段，并按拆分方式选择 q 点输入物化策略。
- 一个 **Phonon Postprocess Workflow** 依赖 **QE Phonon Input Deck** 准备后处理输入，并依赖 **Phonon Workflow Status Machine** 刷新后处理阶段。
- 一个 **Split Merge Workflow** 是 **Split Phonon Workflow** 的回收阶段。
- **Stable Phonon Case** 与 **Unstable Phonon Case** 是 **Phonon Stability** 的互斥结果。
- 一个 **Pressure Point** 可以包含一个或多个 **QE Calculation Setup**，例如 SCF、NSCF、EBAND、ELEDOS 或声子前置 SCF。

## Example Dialogue

> **Dev:** “这个 50 GPa 的 Li-Be-H 体系可以算 stable 吗？”  
**Domain expert:** “看这个 **Pressure Point** 的 **Phonon Stability**。如果最小声子 DOS 频率低于 -1 THz，它就是 **Unstable Phonon Case**；否则是 **Stable Phonon Case**。”

## Flagged Ambiguities

- “虚频”最初可能指任意声子谱频率；在当前 substitutionflow 汇总语境中，已收敛为用于 **Phonon Stability** 判定的最小声子 DOS 频率。
- “case”在代码中常指目录或执行单元；领域语言中应区分 **Substituted Structure** 与 **Pressure Point**。
