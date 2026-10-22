# Feature 规格：EPW 声子阶段一次提交完整交付

**Issue**：[0048 本地 Issue 草案](issue-draft.md)；线上 Gitee 由用户管理，平台 Issue 尚未创建，远程双向链接待验证。

**创建日期**：2026-10-22

**状态**：草稿；本轮记录需求，整版最终草案待确认，未实施。

## 原始需求记录

```text
为什么我刚刚反复执行了epw_wannier -c "epwinput.toml" --stage coarse -s
好几次的这个命令，但是每次都没有执行粗网格插值。我以为 qe的 声子和电子计算已经完成了，大师还是反复卡在那两步。
第一次执行]$ epw_wannier -c "epwinput.toml" --stage wannier -s，显示：
reuse: scf
reuse: nscf
reuse: electronic
job_id: 1329343
script: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/5.epw-wannier/submit-epw-18e0c385ce234068-18535-12.sh
directory: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/5.epw-wannier
workflow_status: submitted
next: 作业结束后再次执行同一公开命令 -s
plan: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/native-plan.json
这说明在进行 wannier 插值。

$ epw_wannier -c "epwinput.toml" --stage coarse -s

reuse: scf
reuse: nscf
reuse: electronic
job_id: 1329733
script: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/4.qe-phonon-nosplit/submit-q2r-18e0c55885b00efa-3e603-14.sh
directory: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/4.qe-phonon-nosplit
workflow_status: submitted
next: 作业结束后再次执行同一公开命令 -s
plan: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/native-plan.json
然后我执行 coarse 插值，但是却跑到4.qe-phonon-nosplit的声子q2r计算。

$ epw_wannier -c "epwinput.toml" --stage coarse -s
reuse: scf
reuse: nscf
reuse: electronic
reuse: PH 响应与 save 清单；继续 q2r/matdyn 后处理

job_id: 1329752
script: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/4.qe-phonon-nosplit/submit-matdyn_phband-18e0c56930380aac-3fab8-13.sh
directory: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/4.qe-phonon-nosplit
workflow_status: submitted
next: 作业结束后再次执行同一公开命令 -s
plan: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/native-plan.json
这里又开始声子的 matdyn计算。

$ epw_wannier -c "epwinput.toml" --stage coarse -s
reuse: scf
reuse: nscf
reuse: electronic
reuse: PH 响应与 save 清单；继续 q2r/matdyn 后处理
job_id: 1329766
script: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/4.qe-phonon-nosplit/submit-matdyn_phdos-18e0c5768029dd7d-40ac6-15.sh
directory: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/4.qe-phonon-nosplit
workflow_status: submitted
next: 作业结束后再次执行同一公开命令 -s
plan: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/native-plan.json
这里又开始声子的后处理。

为什么我之前明明执行了：$ epw_phonon -c "epwinput.toml" --stage phonon -s
reuse: scf
reuse: nscf
reuse: electronic/pw-bands
reuse: electronic/bands
job_id: 1329275
script: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/4.qe-phonon-nosplit/submit-ph-18e0c32ce3551327-10239-1a.sh
directory: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/4.qe-phonon-nosplit
workflow_status: submitted
next: 作业结束后再次执行同一公开命令 -s
plan: /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/native-plan.json
但是没有一次性完成所有的声子计算，q2r, matdyn以及后续的出图后处理以及额外保存出来 epw 插值需要用的 save 目录文件？为什么没有一次性做这些操作，反而一致在 coarse 阶段运行声子计算。

你帮我解释一下这个问题，给出一个合理的解释，以及这个程序运行的流程图。要通俗要能把握住流程，不要给出无意义的细节。我想知道这里面的续算流程是什么，为什么不按照命令对应的含义计算？

当然我不是说你做的有问题，你做的很好，这样也避免了前面的计算没完成，后面的计算报错的问题，你来决定是不是要补完没做的计算也有好处，我可以更无脑的使用这个脚本。但是我还是希望知道为什么不按照我的理解执行。
```

> 所以我希望执行一次epw_phonon -c "epwinput.toml" --stage phonon -s就可以一次性把 ph.x, epw 需要的 save 文件保存，q2r.x， matdyn.x，以及声子谱和ph-tdos,ph-pdos态密度的图都画好。save 文件都处理好。

> 对。

> 我希望你按照我新的要求来做。然后你来新建 issu 和 spec 按照 skill 要求来完成。记得把我的需求都记录记录下啦。

## 附件与推断来源

无上传附件或图片。

```yaml
images: []
```

两条用户批注所选的助手原文如下，只作指代上下文，不冒充用户原话：

> 电子这一边已经走到 Wannier，声子这一边只完成了 PH。选择 coarse 后，程序发现两边尚未汇合，就先补齐声子阶段。

> 当前程序要求的完整声子阶段，包含 q2r/matdyn；这比 EPW 读取原始声子响应所需的最低数据更多。 这样可以同时准备常规 QE 声子结果和后面的比较图。这是程序选择的工作流边界。

AI 对“对。”的理解：用户确认保留包含 q2r/matdyn 的完整声子阶段，不把声子交付缩减为仅有 EPW 所需原始响应。AI 对 `ph-pdos` 的理解沿用已有 [0047](../0047-qe-stage-plots/spec.md)：本次为 QE 单侧元素投影声子 DOS，不扩展为电子轨道投影或插值侧元素投影对比。

## 背景与目标

用户认为 `--stage phonon -s` 提交的是整个声子阶段，但当前命令在提交一次原生调用后返回。没有再次调用就不会继续后处理，后来的 `coarse` 命令才替它补齐，导致阶段边界难以理解。

本功能让用户只执行一次声子公开命令，即能在正常条件下自动得到该阶段的完整交付。提交是启动请求，计算可以异步完成；用户可以查询状态，但查询或再次执行命令不应成为推进条件。

### 需求结论与变更取舍

- 执行入口保持 `epw_phonon -c "epwinput.toml" --stage phonon -s`，不要求新增选项或新公共程序。
- 完整交付同时包括 PH 原始响应、整理好的 EPW `save/`、q2r、声子色散和 DOS 的 matdyn 结果、QE 声子谱／ph-TDOS／元素 ph-PDOS 图及最终核验。
- 用户“对。”确认保留完整 QE 后处理职责；不为尽快进入 coarse 而跳过 q2r/matdyn 或绘图。
- 自动补齐合法依赖与复用有效上游的能力继续保留。已完整交付的声子阶段供 coarse 直接复用；缺失或损坏的必要证据仍按既有规则处理。
- 新要求替代 0026 对本声子目标的逐任务人工推进要求；其他目标没有获得相同变更要求。
- ph-PDOS 沿用 0047 已明确的 QE 单侧元素投影范围，保留旧 0014 的历史暂缓记录，不恢复插值侧投影对比。
- “一次执行公开命令”与“恰好一个 Slurm 作业”是不同约束。用户未指定后者；本 Spec 只规定用户体验与可验收结果，不预定作业组织架构。

## 用户场景与验收

### US1：一次提交即可拿到完整声子结果（优先级：P1）

用户提交一次声子阶段后等待完成，无需反复输入同一命令或借助 coarse 推进。

**优先级理由**：直接解决本次反馈中的重复人工推进问题。

**独立验收**：有效电子／SCF 上游和未开始的声子阶段，只调用一次目标命令；不再调用推进命令，自动得到全部交付。

**验收场景**：

1. **假如**依赖有效、声子尚未开始，**当**提交一次目标命令，**那么**自动执行 PH、整理 save、q2r、两次 matdyn、绘图和最终核验，用户无需在任意子任务结束后再次提交。
2. **假如**任一声子子任务仍在运行，**当**用户只读查看状态，**那么**能区分整阶段运行与完成；不把 PH 结束当作完整交付，不要求查询来驱动后续计算。
3. **假如**完整流程成功，**当**用户之后才查看状态，**那么**完成证据和图表已经生成，无需再运行 `-s` 才认可成功。

### US2：已有结果与失败现场可以接着用（优先级：P1）

用户复用已算好的部分，并能在失败修复后从剩余部分恢复。

**优先级理由**：避免自动串联以重复昂贵计算为代价。

**独立验收**：准备 PH 完成、q2r 完成、仅缺图等不同检查点；分别一次启动补齐余项，核对科学程序调用次数。

**验收场景**：

1. **假如**PH 及其严格响应清单有效、后处理未完成，**当**执行一次目标命令，**那么**自动补齐 save 交付和全部后处理，PH 不重跑。
2. **假如**q2r 失败，**当**本阶段自动执行到失败处，**那么**不继续运行依赖它的 matdyn，不发布完整成功；修复后显式恢复，复用通过核验的 PH，从失败处继续。
3. **假如**科学数据全部有效但图缺失或绘图失败，**当**按对应恢复方式继续，**那么**只补图与完成登记，不重跑 PH、q2r 或 matdyn。
4. **假如**同一请求仍活动或提交结果未知，**当**重复命令，**那么**按既有保护报告当前情况，不因自动化增加重复提交。

### US3：声子阶段自己交齐数据和图，下游直接复用（优先级：P1）

用户在声子阶段目录查看仅 QE 的声子结果，然后继续粗网格插值。

**优先级理由**：让命令对应的交付明确可见，免于把依赖补算误认为粗网格计算。

**独立验收**：无 Wannier／EPW 插值产物时完成声子阶段，检查 save 和三类图；之后继续 coarse，核对声子不重算。

**验收场景**：

1. **假如**无插值侧产物，**当**声子阶段成功，**那么**其目录里有完整 save、QE 声子谱、TDOS 和真实元素 PDOS 及可复查数据，不等待插值结果。
2. **假如**缺少完整真实原子投影，**当**生成元素 PDOS，**那么**明确报告该交付失败，不用 TDOS、模式编号、零曲线或部分原子冒充完整元素图。
3. **假如**声子完整交付且必要证据未变，**当**执行 coarse，**那么**复用声子阶段，不再提交 PH／q2r／两次 matdyn 来收尾上一阶段；其他缺失依赖仍按实际状态处理。

### 边界与异常场景

- save 归集／校验失败时，不向 EPW 发布可用声子交付，不用作业退出 0 替代数据校验。
- 任一科学子任务失败时停止依赖它的后续科学计算；图失败保留有效科学数据，但整个声子交付不得标为完整。
- 已完整完成且请求相容时，再次提交不产生新科学计算；只读查询不改输入和科学产物。
- 原有逐任务手动运行留下的合法检查点仍可复用；本次不要求清空目录或重算才能采用自动流程。
- 输入改变、产物缺失、内容损坏或提交结果未知时，分别遵守当前的重建／拒绝／防重复规则，不将全部异常当作缓存未命中。
- 缺失前置依赖时沿用既有依赖规则，不跳过必要科学计算；此次变更的验收基线明确以前置依赖有效为条件，不凭推断重排电子与声子职责。
- 自动执行被调度器取消、超时或资源拒绝时必须保留可用证据并报告未完成；不承诺无条件成功，也不自动放宽参数或资源上限。

## 需求

### 功能需求

- **FR-001**：一次执行指定 `phonon -s` 命令必须启动自动推进整个声子阶段的流程；正常执行不依赖用户再次运行推进命令、查询命令或切换 coarse。
- **FR-002**：完整阶段必须包含 `ph.x`、EPW save 数据整理与校验、`q2r.x`、声子色散 `matdyn.x` 和声子 DOS `matdyn.x`；相容完成项允许复用。
- **FR-003**：阶段必须完整保存 EPW 所需动力学矩阵、逐 q 响应势、phsave、q 对应关系及可核验清单；使用现有可见目录和交付约定，不以目录存在代替内容有效。
- **FR-004**：阶段必须自动交付仅 QE 的声子谱、ph-TDOS 和元素 ph-PDOS 图及其表格／来源数据，归属声子阶段，不等待 Wannier／EPW 插值结果；复用 0047 的绘图职责。
- **FR-005**：元素 ph-PDOS 必须来自完整真实逐原子投影，并按真实结构汇总；缺失、不完整或非有限的数据不得伪装成有效元素贡献。
- **FR-006**：用户未再次执行命令时，流程仍必须自动完成输出校验、状态登记和图表交付；提交、运行中、失败和完整完成必须可区分，不能以单个作业完成代替整个阶段完成。
- **FR-007**：失败必须阻断依赖该失败结果的后续科学计算，并保留有效已完成结果；采用现有显式恢复语义，恢复后自动执行全部剩余部分，不要求每步手动接力。
- **FR-008**：已有有效 PH、q2r、matdyn 或最终图表必须按其实际完成证据复用；仅缺图时只补图，整阶段完成后重复调用不产生新科学作业。
- **FR-009**：自动推进必须保留活动／未知作业防重复、预算与来源校验；采用既有缺失重建及损坏拒绝规则，不因自动化扩大预算、放宽科学标准或删除历史结果。
- **FR-010**：完整声子阶段必须可由 coarse 复用，不再由 coarse 收尾有效的上一阶段；仍保留对合法缺失依赖的补齐能力。
- **FR-011**：本次自动整阶段提交仅调整 `epw_phonon --stage phonon -s` 及对应恢复路径，不新增公共 CLI，不自动启动下游 Wannier／coarse／fine／超导，不改变其他目标的提交粒度。
- **FR-012**：操作说明必须解释一次提交的异步含义、完整产物、阶段完成判断与失败恢复，并给出通俗流程图；不再要求正常声子流程在每个子任务后重复 `-s`。

### 关键实体

- **声子阶段请求**：由当前配置、科学输入和既有依赖共同确定的一次完整声子目标；与提交命令次数、Slurm 作业个数分开。
- **可复用子结果**：PH、q2r、matdyn 等已通过来源与数据有效性核验的结果，失败恢复时保留其价值。
- **EPW save 交付**：动力学矩阵、响应势和 phsave 等真实数据及其对应关系／来源证据，供后续 EPW 使用。
- **声子图表交付**：仅 QE 的声子谱、TDOS 和元素 PDOS 及其真实数据，与后续插值对比区分。
- **阶段完成结果**：全部必需数据与图表已经通过核验的状态，不能由 PH 或单个后处理作业的终态推定。

## 成功标准

- **SC-001（US1；FR-001～004、006）**：依赖有效且声子未开始的工作区，公开目标命令调用次数为 1；之后无任何推进调用，自动得到 PH、save、q2r、两次 matdyn、三类图及阶段最终完成记录。
- **SC-002（US2；FR-007～009）**：PH 已完成和 q2r 已完成的两个恢复样本，各只需一次启动余项；所有通过核验的先前科学子任务新增调用次数均为 0，剩余流程自动完成。
- **SC-003（US2；FR-006～009）**：分别注入 PH、save 校验、q2r、matdyn 和绘图失败，完整成功状态出现次数为 0；依赖失败输入的后续科学调用次数为 0。修复并恢复后能完成，成功结果无需重算。
- **SC-004（US3；FR-003～005）**：无 Wannier／EPW 产物时可实际打开三类真实 QE 图，MgB₂ 的 PDOS 包含来源对应的 Mg、B 贡献；save 所需文件、q 映射与清单验证有效。缺投影负向样本不得被判为完整交付。
- **SC-005（US2／US3；FR-008、010）**：仅缺图样本补图前后成功科学任务调用次数不变；声子完成后重复命令或继续 coarse，新增 PH／q2r／声子两类 matdyn 调用次数均为 0。
- **SC-006（US1／US2；FR-009、011）**：活动、未知提交、损坏来源和资源受限样本按已有保护处理，无重复有效作业、预算重置或参数放宽；除指定声子目标外，其他目标的提交契约回归保持通过。
- **SC-007（US1～US3；FR-012）**：实施后文档中的示例只调用一次声子提交命令；用流程图明确区分提交、自动子任务、数据与图表、最终完成和失败恢复，读者无需从 coarse 命令代办声子后处理。

## 假设与依赖

- 基线为编号工作区和用户反馈中的 nosplit 路径；沿用现有配置、资源和科学数据契约，不自行把本需求扩大成多 PH 模式并行改造。
- [0047](../0047-qe-stage-plots/spec.md)提供声子单侧图与元素范围契约；0048 自动执行链必须衔接该能力。0047 的代码或文档存在不能代替本项验收。
- [0028](../0028-phonon-postprocess-checkpoint/spec.md)提供已归集 PH 的严格复用边界，[0025](../0025-epw-visible-run-state/spec.md)规定可见状态；预算约束继续沿用，不由本 Feature 重新分配额度。
- [0046](../0046-epw-submit-simplification/spec.md)规定脚本格式和工作区锁处理；本功能不借自动推进恢复已移除锁或全面改写任务状态系统。
- 共享 qeflow／qf-phonon 已有输入生成、归集和绘图能力；实施时核验主路径能否直接复用，不另建声子计算算法。具体任务组织属于后续 Plan，不在 Spec 里预定。

## 待确认决策

没有需要改变已明确产品目标的新范围选择；本版完整草案尚待用户审阅确认。单分配顺序执行或依赖作业链等技术方案应在 Spec 确认后研究；本草案未将其中任何方案当作用户决定。若方案确需改变已有明确约束，再单独澄清，不静默扩展范围。

## 范围外事项

- 将所有 EPW 命令改成一次自动跑完整项目，或自动提交下一物理阶段。
- 电子元素／轨道 PDOS、Wannier／EPW 元素声子 PDOS 对比。
- 新的物理参数、网格、拟合窗口、科学判据或预算额度。
- 全面重构状态、全面去锁、修改原始历史数据、静默迁移全部旧工作区。
- 本轮的生产代码实现、科学作业提交、安装切换、线上写入及推送。
- 把计划能力提前写成 Overview 当前事实。

## 澄清记录

- 初始反馈要求解释反复提交时的真实推进流程，并认可依赖补齐的价值；后续明确要求声子目标一次提交完成全部工作，因此保留依赖复用与检查，改变该目标的手动接力体验。
- 第一条批注明确 PH、save、q2r、matdyn、声子谱、TDOS、PDOS 全部纳入本阶段交付；第二条“对。”确认完整 QE 后处理边界。完整原话均保留于“原始需求记录”。
- 最新请求明确创建新的 Issue／Spec 并保存需求；本轮按本地草案约定执行，不据此声称草案定稿、代码已实现或默认安装已变化。

## 最终草案确认

待确认。用户已明确目标和本轮文档记录要求，尚未对本版完整 Issue／Spec 正文作出最终确认。本版可作为需求草案保存与审阅；Plan、Tasks、生产实现和真实验收尚未开始，不借用其他 Feature 的实施授权。
