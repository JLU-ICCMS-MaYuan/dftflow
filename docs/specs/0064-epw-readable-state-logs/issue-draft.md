# 0064：EPW 八阶段可读日志与状态目录简化

> 2026-11-17 最新约定：预算账本功能已按用户要求彻底撤销。本文件涉及账本、campaign 身份、预算预留／结算及全局两个作业限制的旧条款均已失效；当前计算只依赖工作流记录、Slurm 作业身份及科学核验。历史消耗与验收记录不改写。详见 [0058 最新需求](../0058-epw-remove-resource-limits/spec.md)。

唯一类型：`type:feature`。创建日期：2026-10-27。状态：本地实现、受控回归和文档完成；2026-10-28 已按追加授权编译发布，指定 MgB₂ 真实重算证据见 [验收](acceptance.md)。线上状态由用户管理。

0064 为本地暂定编号，沿用线上 Gitee 由用户管理的约定，未创建线上 Issue，不表示平台已分配该编号。关联 [Spec](spec.md)；远程链接待验证。

## 问题与目标

失败输入已归档，旧活动记录却仍指向原路径；程序在重试前校验旧记录，导致无法生成新输入。用户希望直接阅读八个阶段的现状、失败原因与下一步，不再维护多套 JSON、请求指纹、提交尝试和历史目录。

本项重新定义 EPW 工作流状态的持久化：八份人类可读日志承载阶段状态和续算信息；各计算目录的 `xxx.jobs` 保存当前任务号；外部导入保留 `imports/`；输入配置按内容变化保存原文版本。

## 范围与验收

1. 八份日志均为有标题、分区、英文状态和可执行下一步的纯文本；不能把 JSON 改成 `.log` 后缀。
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

本项补充迁移文档：明确“撤下”为删除固定旧状态及其历史内容，不移动、不备份；当前计算目录、结果和 imports 保留。在 SiP₂ README 追加正式入口、无提交转换命令、检查及失败处理，不能把写入文档视为已转换真实 SiP₂。配置阶段命名调整由 [0061](../0061-epw-flat-stage-config/spec.md) 统筹。

## 原始需求记录追加：SiP₂ 旧赝势目录接续（2026-10-28）

> 接续还有一处问题：第 3 步日志报输入不符，[eband.in 第 5 行 (line 5)](/data/home/mayuan/code/dftflow/examples/SiP2/3.qe-electronic/qe_eband/eband.in:5) 仍引用已删除的旧 `attempts/.../pp` 赝势目录。这是后续需要处理的兼容问题；本轮没有改写科学结果或提交重算。把这个处理好

本次修复沿用 0064：旧能带输入只因赝势目录迁移而不匹配时，必须核对保存树中的实际赝势内容和完整输入，允许已有结果继续使用。赝势内容、其他科学参数、路径或证据不符仍拒绝；保留原始输入和 XML，不调用科学程序或提交重算。显式迁移命令可以重新核验此前转换失败的电子阶段，核验成功后才登记完成。

本地修复已验证并安装：SiP₂ 第 3 步通过正式 `--migrate-state` 重新核验，保留批次 `converted-electronic` 和历史科学文件，清除旧输入冲突。Documentation Impact：SiP₂ README 和 Quickstart 已补重验命令，Overview 已说明路径与内容的区别；源码、受控回归、安装哈希和真实边界见 [验收](acceptance.md)。线上 Issue 仍由用户管理，本次不操作线上或推送。

## 全部输出英文的后续补充

用户已确认“全部包括，统一英文”，覆盖运行提示、持久化阶段日志、帮助、报错和程序生成的说明。采用 Spec 的 FR-033～035、SC-017；所有复用标记带编号阶段及实际声子模式。旧中文日志在读取边界兼容，科学状态与任务身份保持原含义，不因语言变化重算。

文档影响：更新 Spec／Plan／研究／文件契约、快速验证与验收，并将已验证的英文输出行为回写总览。本地草稿不代表线上 Issue 已更新或关闭。
