# 0048：EPW 声子阶段一次提交自动完成计算、save 归档与出图

唯一类型建议：`type:feature`。日期：2026-10-22。状态：本地 Issue 草案，用户已要求记录，整版最终草案待确认，未实施。

origin 为 Gitee。沿用仓库 `AGENTS.md` 中线上由用户管理、本地保存草案的约定；本机未安装 `gitee` CLI。0048 是本地暂定编号，不代表平台 Issue 已创建；平台登记后用真实编号校准目录。尚未查询线上开放 Issue，远程双向链接待验证。本文件仅保存待发布正文，不充当平台 open／closed 状态副本。

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

## 问题与目标

当前编号工作区每次 `-s` 最多提交一个原生调用，PH 结束后还要再次执行公开命令，才能整理数据并依次推进 q2r、两次 matdyn 和后处理。用户切换到 `coarse` 后，这些尚未完成的依赖被补算，因此操作与“提交整个声子阶段”的预期不符。

新目标：用户执行一次 `epw_phonon -c "epwinput.toml" --stage phonon -s` 后，只要计算与后处理成功，无需再次执行任何命令，即自动完成本声子阶段的计算、EPW 数据整理、图表及最终完成登记。命令提交返回不表示计算立即结束。

## 范围与交付

1. 自动执行或复用同一请求下的 `ph.x`，完整保存并校验 EPW 要用的 `save/`，包括动力学矩阵、响应势、phsave 及相应清单。
2. 自动执行或复用 `q2r.x`、声子色散 `matdyn.x`、声子 DOS `matdyn.x`，保留完整的 QE 声子后处理职责。
3. 自动生成仅 QE 的声子谱、ph-TDOS 和元素 ph-PDOS 图及其可复查数据；MgB₂ 展示真实 Mg／B 元素贡献，不等待 Wannier／EPW 插值结果。
4. 自动核验并登记阶段最终结果。成功路径中用户不需要再次运行 `-s` 来触发下一任务、整理 save、出图或完成登记。
5. 保留已验证结果复用和失败阻断；失败后按现有显式恢复语义，从未完成部分继续，成功的科学计算不重复执行。
6. 保留后续目标补齐依赖的能力；本命令完整成功后，正常执行 `coarse` 应复用声子交付，无须再代办声子后处理。

“一次提交”约束用户只调用一次公开命令；不把它擅自解释为必须恰好一个 Slurm job。具体自动串联方式留待 Spec 确认后的设计阶段评估，不在本草案预先规定。

## 可验收结果

- 依赖已有效、声子阶段未开始时，只调用一次上述命令，随后不再调用任何推进命令，最终自动得到 PH、save、q2r、两次 matdyn、三类图和阶段完成证据。
- 已有有效 PH、缺后处理时，一次命令补齐全部余项，PH 调用次数不增加；只有图缺失时仅补图。
- q2r／任一 matdyn／save 校验／绘图失败时，未满足的交付不标为完整；依赖失败不得让后续科学计算带错运行。恢复保留成功结果。
- 元素投影必须来自完整真实原子贡献并按元素汇总；缺数据不能拿总 DOS、模式编号或零曲线冒充。
- 阶段完整后重复命令不产生新科学作业；继续 coarse 时不重跑有效声子任务，且缺失或损坏的必要证据仍按已有规则处理。
- 详细场景与量化标准见 [Spec](spec.md) 的 US1～US3、FR-001～012、SC-001～007。

## 依赖、关联与查重

- 关联功能域 [0007](../0007-epwflow-superconductivity/issue-draft.md)，不新建 Epic，不声称已建立平台父子关系。
- [0047 即时出图](../0047-qe-stage-plots/spec.md)负责 QE 单侧图的内容、归属与共享绘图能力；本项负责一次命令自动推进声子阶段，并把这些图纳入最终交付。两者联合验收，不重复实现绘图，不修改 0047 的电子或插值图范围。
- [0026 逐原生任务提交](../0026-epw-direct-native-submit/spec.md)要求人工反复执行 `-s`。本次较新要求明确替代该约束在 `epw_phonon --stage phonon -s` 上的行为；不扩大为所有公共命令自动串联。
- [0006 QE 声子一次提交](../0006-qe-phonon-submit-chain/spec.md)属于共享 QE 能力，不能据此认为 EPW 编号入口已经具备本需求；实施时评估复用，不维护第二套科学算法。
- [0028 声子检查点](../0028-phonon-postprocess-checkpoint/spec.md)和 [0025 可见状态](../0025-epw-visible-run-state/spec.md)的复用、验源与可见状态约束继续适用。
- [0046 脚本简化](../0046-epw-submit-simplification/spec.md)的脚本格式与去除工作区控制／执行锁边界不因本项回退；不借自动串联重做全部状态管理。
- 本地查重结果：0006 面向共享 QE，0026 为相反的人工推进策略，0047 为出图内容与时机；本次新增范围是 EPW 编号声子阶段一次命令自动完成。线上查重未完成。

## 范围外事项

不将 Wannier、coarse、fine、超导等其他目标一并改为自动整链；不自动启动这些下游计算。不增加公共 CLI、电子 PDOS 或 EPW 元素声子投影，不修改科学参数或数值标准，不重置预算。不把一次命令承诺成一次作业必然成功；失败需明确报告及恢复。

本轮只记录 Issue／Spec 草案与需求检查，不修改生产代码、实际计算数据或安装，不提交科学作业，不推送。

## Documentation Impact

- [ ] 实现并验证后，更新 `docs/overview/epwflow/README.md` 的声子提交、完成与恢复行为；其他目标仍按其实际规则说明。
- [x] 新增本目录 Issue、Spec 和需求检查表；证据：本轮逐段核对用户原话、FR／SC 覆盖与本地链接，检查结论见 [requirements.md](checklists/requirements.md)。不表示 Spec 已确认或实现验收通过。
- [ ] 实现后更新 `epwflow/README.md` 与 MgB₂ 算例 README，提供一次提交的完整产物与状态说明、通俗流程图及失败恢复示例；核对根 `README.md`，仅在其入口说明受影响时修改。
- [ ] 平台登记后核验 Issue／Spec 双向远程链接；未推送前不勾选，不自动推送以补链接。

## 关闭条件

全部 SC 满足，并提供公开入口自动执行、失败恢复、旧结果复用及真实图像查看证据；完成对应 Documentation Impact，核验平台关系和远程链接。仅记录文档不关闭需求。

## 最终草案确认

用户明确提出新目标并要求新建 Issue／Spec，授权本轮本地需求文档记录。本版完整正文尚未由用户审阅确认，不冒用 0047 或历史功能的授权作为本版 Spec 定稿或实施授权；Plan、Tasks 和实现尚未开始。
