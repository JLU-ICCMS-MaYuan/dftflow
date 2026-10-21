# 0026：EPW 提交脚本直接调用科学可执行程序

- 唯一类型：`type:maintenance`。
- 平台：origin 为 Gitee；沿用本地 Issue 草稿、线上 Gitee 由用户管理的仓库约定。
- 编号：0026 是本地需求编号，不代表已创建线上 Issue。
- 详细需求：[Spec 草稿](spec.md)。当前仅保存可审查草稿，本版最终草案尚待确认，未进入实施。

## 原始需求记录

> 而且我希望你能把epwflow里面用到的提交任务的脚本，直接提交相应的运行那个 epw， wannier， qe 的可执行程序的脚本就行。你可以参考/data/home/mayuan/code/my_script/epw和/data/home/mayuan/code/my_script/qe里面提交任务的脚本。当我指定-s 提交任务的时候，就这么做。不然我根本不知道你是怎么提交任务的。而且这样简单，也方便我 debug。
>
> 新建 issue 和 spec 来完成。

## 问题与目标

目前编号工作区的 `submit.sh` 通过 `--workspace-attempt` 调用 EPW 阶段程序，原生子任务通过 `--workspace-task` 间接执行，用户无法从实际提交脚本直接读出科学命令。

目标是：使用三个公开 EPW 入口的 `-s` 时，实际提交给 Slurm 的作业脚本直接写出相应 QE、Wannier90、EPW 可执行程序、参数、工作目录以及输入输出，便于检查和 debug。

## 范围

- 覆盖 `epw_phonon`、`epw_wannier`、`epw_superconductivity` 的 `-s`，包括其支持的阶段选择、恢复与总提交路径中实际使用的科学作业脚本。
- 参考用户指定的 `epw_writesubmit.py`、`epw_submitjob.py`、`qe_writesubmit.py`、`qe_submitjob.py` 的直接命令写法；资源、路径与参数仍读取 DFTFlow 当前配置。
- 实际科学调用直接出现在被提交的脚本中；不能仅新增展示脚本，实际仍提交内部执行器包装脚本。
- 输入准备、数据交接、质量核验和运行记录职责须明确；必要辅助步骤可以保留，但不得经辅助入口再次隐藏科学程序启动。
- 保留相容上游复用、失败阻断、预算、身份核验和防重复提交等既有约束；脚本简化不构成取消这些约束的授权。
- 提交成功输出可定位的实际脚本路径和作业号，用户能够对照脚本找到本次输入及原生日志。

## 范围外事项

不扩展 EPW 公开程序数量，不重写科学算法，不改变物理参数、选窗策略、质量标准或作业资源；不把独立 Wannier90 路径与 EPW 内置 Wannier 路径强行互换。不整体重构独立 qeflow／wannierflow 提交方式，不复制参考脚本中的 Python 实现、机器硬编码或无关清理命令。不覆盖历史冻结脚本、默认安装或旧计算数据，不提交真实科学作业，不自动推送。

## 验收与关闭条件

- [ ] 三个公开入口的实际 `-s` 提交均有行为验证；捕获的 `sbatch` 目标脚本含相应原生科学命令。
- [ ] 脚本可直接读出可执行程序、launcher、参数、工作目录、输入和输出；不以 `--workspace-attempt`、`--workspace-task` 或等价内部科学执行器代替直接调用。
- [ ] 支持的独立 Wannier90、EPW 内置 Wannier及 QE／EPW 后处理路径按原科学流程执行，不为统一脚本而漏步骤或换算法。
- [ ] 正常执行、关键步骤失败、恢复复用、活动／未知任务、身份冲突及预算边界均有行为证据；失败不执行后续依赖科学命令或发布成功交付。
- [ ] 实际脚本可定位、语法正确，带空格路径与输入输出重定向正确；日志文件名及日志位置沿用 0024 的共同验收要求。
- [ ] 文档回写完成；脚本提交验证与真实科学验收分别记录，不以替身回归代替真实科学验收。

## 依赖与关系

- 无新增父 Issue；作为独立维护需求关联 [0007 EPW 总体需求](../0007-epwflow-superconductivity/issue-draft.md)。不改写既有 Epic 的范围或完成状态。
- [0018 编号工作区](../0018-epwflow-stage-workspaces/spec.md)、[0023 独立 Wannier 启动](../0023-epw-wannier-native-start/issue-draft.md)、[0025 可见运行状态](../0025-epw-visible-run-state/issue-draft.md)：保留科学阶段、数据来源及可见记录约束，实施前核对最终代码基线。
- [0024 脚本简化](../0024-slurm-script-simplification/spec.md)：负责日志文件名、关键失败停止及 shell 写法；本项新增“实际提交脚本直接执行科学程序”的要求，共用相关验收，不另立冲突规则。
- [0019 脚本选项](../0019-shell-script-options/spec.md)：继续禁止自动严格选项，不通过恢复 `set -euo pipefail` 简化实现。
- 线上 Issue、平台关系与 Issue／Spec 远程双向链接尚未建立或验证，本地链接不表示远程链接有效。

## 文档影响（Documentation Impact）

- [ ] 完善本目录 Spec；最终草案确认后建立 Plan、Research、Tasks、操作验证说明及必要的数据／接口契约。
- [ ] 实施并验收后，更新 `docs/overview/epwflow/README.md` 的实际提交方式与恢复边界；按需调整 Overview 导航。
- [ ] 更新 `epwflow/README.md` 的 `-s`、阶段脚本、总提交及 debug 用法；根 README 仅在入口导航受影响时修改。
- 当前未改变实现，不将直接执行目标写入当前功能 Overview，不关闭关联需求。

## 最终草案确认

待确认。用户已要求新建 Issue 与 Spec；这不等于本版最终正文已经经过确认。当前未操作线上 Gitee，未修改生产生成器或安装。
