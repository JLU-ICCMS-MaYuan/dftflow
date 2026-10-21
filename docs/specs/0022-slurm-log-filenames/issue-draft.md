# 0022：Slurm 日志指令使用简短文件名并输出到提交脚本目录

- 唯一类型：`type:maintenance`。
- 平台：origin 为 Gitee；沿用仓库本地 Issue 草稿、线上 Gitee 用户管理约定。0022 为经本地目录查重后的需求编号，不是已创建的线上 Issue number。
- 实施授权：用户已明确要求“开始落地这个需求。”；沿用本项维护需求，实施记录见 [设计与验收](acceptance.md)。线上 Issue 仍由用户管理。

## 原始需求记录

> 我希望你再新建一个 issue 要求：脚本生成的 slurm 脚本不要在 --output=和--error=后面写这么长的绝对路径，可读性非常差，反正你也是要把这些日志输出在提交任务脚本所在的目录下，没必要写这么长，只需要定义好名字就行。

后续问题反馈：

> 我记得有一个 issue 是要求不在提交任务的脚本里面写过长的#SBATCH --output=和#SBATCH --error=，为什么我看现在的脚本还是写这么长呢？比如：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/6.epw-coarse/submit.sh
>
> #SBATCH --output=/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/6.epw-coarse/slurm-%j.out
> #SBATCH --error=/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/6.epw-coarse/slurm-%j.err
>
> 你到底有没有落实这个需求。

实施授权原文：

> 开始落地这个需求。

## 目标与用户价值

自动生成的 Slurm 脚本在 `#SBATCH --output=` 和 `#SBATCH --error=` 后仅写日志文件名，去掉冗长的绝对目录前缀，便于阅读、检查和迁移。标准输出与标准错误仍保存到对应提交脚本所在目录。

示例（现有命名规则允许保留，不强制所有阶段改用同一前缀）：

```bash
#SBATCH --output=slurm-%j.out
#SBATCH --error=slurm-%j.err
```

## 实施前依据与实施范围

- `epwflow/src/foundation/workspace.rs` 的 `script()` 将目录与 `slurm-%j.out`／`slurm-%j.err` 拼接后写入脚本；`start()` 当前以阶段目录调用 `sbatch`。实施时须核对脚本目录与提交工作目录是否一致。
- `epwflow/src/foundation/workflow.rs` 的 `generate_phase_for()` 同样写入完整日志路径；旧提交路径还通过 `sbatch --output`／`--error` 参数传入完整目录，需要检查是否覆盖脚本中的简短名字。
- 核查各 flow 的自动 Slurm 脚本生成器，统一修正同类自动生成行为；已经只用文件名的生成器保持原有命名。
- 同时处理公开提交入口、总提交脚本和直接提交生成脚本时的工作目录，使日志位置满足目标。不能只删目录前缀而忽略 Slurm 对相对路径的解析时机，也不能仅依赖脚本正文中的 `cd`。
- 保留日志中的作业号占位符及阶段区分，避免不同作业的日志相互覆盖。

## 范围外事项

- 不更改计算参数、调度资源、阶段依赖、预算和失败恢复逻辑。
- 不批量改写已冻结的历史脚本、作业身份、日志或科学结果。
- 不擅自覆盖用户自定义 Slurm header；本需求针对程序自动生成的日志指令。
- 本轮不提交计算、不切换默认安装、不推送，不创建或关闭线上 Issue。

## 验收标准与关闭条件

- [x] 自动生成的 `--output=` 与 `--error=` 值仅含文件名，无绝对路径或相对目录前缀；相关 `sbatch` 参数不再覆盖为长路径。
- [x] 通过公开命令提交、总脚本提交以及从其他目录提交生成脚本时，日志均落到该提交脚本所在目录；实际支持方式在文档中说明。
- [x] 带空格的脚本目录处理正确；保留 `%j` 等现有必要占位符，标准输出与标准错误可明确区分。
- [x] 增加必要的生成／提交回归，验证日志指令内容及提交工作目录；生成脚本通过 Bash 语法检查。
- [x] 历史冻结脚本及来源数据不变，作业身份、重复提交和失败恢复保护继续有效。
- [x] 实施后按下述文档影响回写，并提供可复查的验证依据；记录需求或生成脚本本身不等于验收通过。

核验依据：专项回归在当前工作树及独立基线各 6 项通过；独立基线相关检查合计 61 项通过、1 项既有真实数据用例未执行。详细覆盖和真实集群／安装边界见 [设计与验收](acceptance.md)。本地技术条件通过不表示线上 Issue 已关闭。

## 依赖与关系

- 独立维护项，无父 Issue，无需等待新的科学计算结果。
- 关联 [0006 声子提交链](../0006-qe-phonon-submit-chain/issue-draft.md)、[0008 EPW 基础能力](../0008-epwflow-foundation/issue-draft.md)、[0018 阶段工作区](../0018-epwflow-stage-workspaces/issue-draft.md) 和 [0019 脚本选项](../0019-shell-script-options/issue-draft.md)；保留既有脚本与提交保护约束。
- 本项为维护需求，复用原范围与验收条件；设计、实现说明及分层验证集中保存在 [设计与验收](acceptance.md)，不另建 Feature。
- 线上编号、关系和远程链接由用户上线后核验；本地记录不表示平台协作状态已变更。

## 文档影响（Documentation Impact）

- [x] 更新 `docs/overview/README.md` 及 EPW／Wannier90 页面，记录统一日志命名、目录控制和历史脚本边界。
- [x] 同步更新 `epwflow/README.md` 的生成与生效范围；根 README 的命令入口未变，无需修改。
- 仅在实现与验证后回写当前事实；已有二进制、冻结脚本和真实集群作业的验证分别报告。
