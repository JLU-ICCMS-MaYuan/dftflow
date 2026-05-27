# DFTFlow

DFTFlow 用于组织高通量第一性原理计算工作流，并把结构替换、压力点计算和后处理结果汇总成可比较的批量结果。

## Language

**Substitution Batch**：
一次围绕同一占位结构、同一替换规则和同一组压力点展开的高通量替换任务。
_Avoid_：batch root, output folder, high-throughput run

**Placeholder Element**：
输入结构中用于表达可替换位点的假元素标签。
_Avoid_：fake atom, dummy element

**XA Series**：
固定 **XA Placeholder** 后，由多个 **XB Placeholder** 替换结果组成的一组候选体系。
_Avoid_：XA directory, element group

**Substituted Structure**：
将占位元素替换为真实元素后得到的一个具体候选体系。
_Avoid_：case, formula directory

**Pressure Point**：
同一个 **Substituted Structure** 在指定外压条件下的一次计算实例。
_Avoid_：pressure folder, pressure case

**Workflow Status**：
一个 **Pressure Point** 在工作流步骤上的三态进度分类：`finished`、`error` 或 `unstarted`。
_Avoid_：job state, running state

**Phonon Stability**：
一个 **Pressure Point** 基于最小声子 DOS 频率得到的二元稳定性分类：`stable` 或 `unstable`。
_Avoid_：dynamical stability, imaginary-frequency status

**Stable Phonon Case**：
最小声子 DOS 频率不低于 -1 THz 的 **Pressure Point**。
_Avoid_：no imaginary frequency, passed phonon

**Unstable Phonon Case**：
最小声子 DOS 频率低于 -1 THz 的 **Pressure Point**。
_Avoid_：has imaginary frequency, failed phonon

## Relationships

- 一个 **Substitution Batch** 包含一个或多个 **XA Series**。
- 一个 **XA Series** 包含一个或多个 **Substituted Structure**。
- 一个 **Substituted Structure** 包含一个或多个 **Pressure Point**。
- 一个 **Pressure Point** 有一组 **Workflow Status**，并且最多有一个 **Phonon Stability** 结果。
- **Stable Phonon Case** 与 **Unstable Phonon Case** 是 **Phonon Stability** 的互斥结果。

## Example Dialogue

> **Dev:** “这个 50 GPa 的 Li-Be-H 体系可以算 stable 吗？”
> **Domain expert:** “看这个 **Pressure Point** 的 **Phonon Stability**。如果最小声子 DOS 频率低于 -1 THz，它就是 **Unstable Phonon Case**；否则是 **Stable Phonon Case**。”

## Flagged Ambiguities

- “虚频”最初可能指任意声子谱频率；在当前 substitutionflow 汇总语境中，已收敛为用于 **Phonon Stability** 判定的最小声子 DOS 频率。
- “case”在代码中常指目录或执行单元；领域语言中应区分 **Substituted Structure** 与 **Pressure Point**。
