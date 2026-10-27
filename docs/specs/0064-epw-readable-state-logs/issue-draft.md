# 0064：EPW 八阶段可读日志与状态目录简化

唯一类型：`type:feature`。创建日期：2026-10-27。状态：本地实现、受控回归和文档完成；2026-10-28 已按追加授权编译发布，指定 MgB₂ 真实重算证据见 [验收](acceptance.md)。线上状态由用户管理。

0064 为本地暂定编号，沿用线上 Gitee 由用户管理的约定，未创建线上 Issue，不表示平台已分配该编号。关联 [Spec](spec.md)；远程链接待验证。

## 原始需求记录

后续真实重算授权：

> 你把整个代码重新编译拿到新的 CLI 之后, 把/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow里面的那些计算目录和epw-state, results都删除，然后重新测试。

> 旧输入已被归档，但 `native-active.json` 仍引用原路径，程序在进入恢复流程前就因找不到旧输入而停止了。 我觉得这很有问题。就是这些文件一直在阻止我重算。我觉得这些防御性设计让这个程序就很难用。epw-state里面的各个文件都是做什么用的？我总感觉保存这么多东西，没用。

下列文件名是用户批注的上下文；引用块逐字保留对应批注。

`layout.json`：

> 这个部分保留到 4.qe-phonon-xxx.log里面。但是不要写成 json 格式。尽可能美观有条理的展示给我。

`current-config.toml`：

> 不需要保存这个，而是改造成 0000-config.toml, config.toml，只要修改一次输入文件，就备份一次。这样我就可以拿到每一次修改后的 toml文件。

`parameter-map.json`：

> 删除不需要

`active.json`：

> 也放到 4.qe-phonon-xxx.log里面

`attempts/`：

> 我觉得这个完全不需要保留，里面的内容也都不需要保留。代码上也完全不需要这部分的暴露。

`native-active.json`：

> 不需要保留这个输出

`native-active-iso/aniso.json`：

> 这两个记录放在 8.epw-superconductivity.log里面。

`native-jobs/`：

> 我建议为每个任务新建一个 xxx.jobs，比如 1.qe-scf.jobs，专门用来存放任务号，用来判断这个阶段的任务完成没完成。

`native-jobs/history/`：

> 不需要保留。

`native-progress/`：

> 这个其实很重要，这个我需要和你探讨一下，能不能在 xxx.log和 xxx.jobs里面兼容这个续算的功能。或者你可以把续算的功能放到各个计算目录里面记录，统一都叫做 continue.log

`requests/`：

> 不保留。对于上一个步骤的依赖，可以从代码上去上一个目录里面检查。我记得你有这部分代码，为什么还要再暴露一个 requests目录呢？都删了这部分代码。不暴露输出 requests

`native-attempts/`：

> 不保留

`qe-data/、coarse/、interpolation/`：

> 不保留。根本用不上，我也不会看这些目录。

`post/`：

> 不需要保留。你直接去 4.qe-phonon-xxx目录里面去找。

`history/`：

> 不需要保留，就直接覆盖就行。

`imports/`：

> 这个很重要，这个需要保留，以后导入外部计算得到的 save 目录，就用这个目录来完成。

> 我现在的思路是在 epw-state里面只放 8 个文件，这8 个文件对应 8 个计算状态：1.qe-scf.log   2.qe-nscf.log   3.qe-electronic.log    4.qe-phonon- xxx.log(因为)    5.epw-wannier.log    6.epw-coarse.log     7.epw-fine.log    8.epw-superconductivity.log  ，然后你往这样的 8 个 文件也要规范输出并且具有人类的可读性。人类的可读性非常重要，你是给我看的，不光是你自己看。
>
> 新建一个 issue 和 spec 来完成。

续算记录位置的澄清答复：

> 各计算目录只放 xxx.jobs：续算信息统一并入 epw-state/ 的八份阶段日志。

配置备份触发时机的澄清答复：

> 每次运行 CLI 时检查（推荐）：内容变了就保存为下一份 0000-config.toml、0001-config.toml；不改内容就不重复备份。

> 开始落地

## 问题与目标

失败输入已归档，旧活动记录却仍指向原路径；程序在重试前校验旧记录，导致无法生成新输入。用户希望直接阅读八个阶段的现状、失败原因与下一步，不再维护多套 JSON、请求指纹、提交尝试和历史目录。

本项重新定义 EPW 工作流状态的持久化：八份人类可读日志承载阶段状态和续算信息；各计算目录的 `xxx.jobs` 保存当前任务号；外部导入保留 `imports/`；输入配置按内容变化保存原文版本。

## 需求结论

- `epw-state/` 顶层恰好八个阶段日志文件，另允许用户明确保留的 `imports/` 目录；声子日志按当前实际模式命名。
- `layout.json` 的布局／声子模式及 `active.json` 的声子阶段任务信息并入第 4 步日志；第 3 步等其他阶段分配写各自日志。
- iso／aniso 状态分别显示在第 8 步日志内；续算统一进入八份日志，不生成 `continue.log`。
- `xxx.jobs` 位于对应编号计算目录，记录子任务与当前 Slurm 任务号；任务实时状态向 Slurm 查询，科学完成另核验实际输出。
- 不再生成或读取 `current-config.toml`、`parameter-map.json`、`active.json`、`attempts/`、`native-active*.json`、`native-jobs/`、`native-progress/`、`requests/`、`native-attempts/`、共享 `qe-data/`／`coarse/`／`interpolation/`、`post/` 和 `history/`。删除相应持久化、路径门槛和对外暴露代码，不能换位置保留同一套记录。
- 上游依赖直接检查所属计算目录中的科学输入输出；声子后处理就近使用第 4 步结果，不另建共享缓存。
- 当前输入文件旁保存 `0000-config.toml`、`0001-config.toml` 等原文快照。每次配置驱动的 CLI 调用检查，内容变化才备份；不保存程序展开后的第二套配置，不强制改名用户现有输入。
- 重算覆盖本阶段旧现场，不自动建立历史；旧记录缺文件不能阻止已结束任务按当前配置重算。正在运行或提交结果未知的任务不能被覆盖。
- 保留外部 `save/` 导入能力与来源追踪；旧目录退出、下游检查和清理命令同步改造。

## 范围与验收

1. 八份日志均为有标题、分区、中文状态和可执行下一步的纯文本；不能把 JSON 改成 `.log` 后缀。
2. 任务号、运行状态、科学完成、收敛与后处理分开表达；多子任务、多个展宽、iso／aniso 和温度进度均能续接，不能退化成只有一个“完成”字段。
3. 重现 SiP₂ 类旧输入缺失场景，公开 CLI 能按当前输入重新准备；有效上游不重复执行。
4. 相同配置重复调用不增加备份；每次观察到新内容保存一份原文字节相同、编号不覆盖的配置版本。
5. PH／q2r／matdyn、粗／细矩阵和超导的直接读取、格式检查及结果发布全部通过；不存在依赖已删共享清单的残余主路径。
6. 已结束任务直接覆盖，八份日志及 `.jobs` 反映本批次；运行中、提交未知、记录半写与并发方法更新有明确处理，不会丢任务号或重复提交。
7. 旧工作区转换可重试，不依赖旧输入路径或旧锁解除重算；先保留导入数据和有效现用科学结果，再退出旧状态。转换不新建历史备份。
8. 完整标准见 [Spec 的成功标准](spec.md#成功标准)；文档检查不代表上述软件行为已实现。

## 依赖与关系

关联 [0025 可见状态](../0025-epw-visible-run-state/issue-draft.md)、[0026 原生任务提交](../0026-epw-direct-native-submit/issue-draft.md)、[0048 声子阶段链](../0048-epw-phonon-stage-chain/issue-draft.md)、[0049 多温度提交](../0049-epw-multitemperature-submit/issue-draft.md)、[0056 稳定工作目录](../0056-epw-stable-workdir/issue-draft.md)、[0057 历史清理](../0057-epw-clear-history/issue-draft.md)、[0061 平层配置](../0061-epw-flat-stage-config/issue-draft.md)、[0062 输入头清理](../0062-epw-clean-input-headers/issue-draft.md)、[0063 路径及归档修复](../0063-epw-high-symmetry-path/issue-draft.md)与 [0010 外部声子接续](../0010-epwflow-qe-data/issue-draft.md)。

本项拟替代上述文档中与多套工作流状态、参数映射输出、共享持久清单和历史归档冲突的要求；不把历史验收改写成新方案已经通过。不新增父子关系或改动既有线上状态。

## 范围外事项

本轮实施并验证运行代码；不修改现有科学输入／计算数据、不切换安装、不清理真实现场、不提交 Slurm、不推送。功能实施不改变科学模型、数值参数、材料路径、收敛标准和结果包的数据格式；预算账本等项目级功能不借本项整体删除。结果包清单不充当新的工作流状态仓库。

## Documentation Impact

- [x] 实施并验证后更新 `docs/overview/epwflow/README.md`、`workflow.md`、`preflight-and-reuse.md`、`epw_clear.md` 和相关配置说明。
- [x] 最终需求确认后补齐本项 Plan、Research、必要的数据／文件契约、Tasks 与操作验证说明。
- [x] 实施后更新 `epwflow/README.md`、MgB₂／SiP₂ 示例及测试说明。
- [x] 核对根 `README.md` 的实际影响并据实更新；不能仅因公共命令名称未变而跳过。
- [ ] 由用户管理线上 Issue 类型、关联及 Issue／Spec 双向远程链接。
- 本项设计、Overview、模块／示例和根 README 已按已验证源码同步；首轮受控回归与追加构建发布／真实重算分别见 [验收](acceptance.md)。

## 关闭条件

Spec 的全部 SC 有真实生产路径验证，八份日志可读性和所有续算场景均通过，旧状态输出及读取依赖已退出，Documentation Impact 已落实；源码、提交、安装和真实科学验收分别报告。本地实现、受控验收及追加安装／MgB₂ 八阶段与六点真实验收均已完成，性能和数值限制如实保留；线上类型、链接和关闭仍由用户管理，不以本地提交冒充线上关闭。

## 最终草案确认

用户在本版草案建立后明确要求“开始落地”，当前按本版需求与两项澄清实施；线上 Gitee 继续由用户管理，不重复请求本地实施许可。
