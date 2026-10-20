# 0020：修复 QE 能带后处理并行失败及原生任务续算

唯一类型：`type:bug`。本地编号 0020 未占用；线上 Gitee 由用户维护，不创建或关闭线上 Issue，远程链接待验证。

## 原始需求记录

完整逐字原文及用户要求实施的完整方案见 [原始需求记录](original-request.md)。用户已明确要求实施该方案，不重复请求授权。

## 问题、目标与关系

MgB₂ 作业 1306688 在 PW 能带成功后因 bands.x 单 pool 的 slab FFT 分解出现空进程而失败；1306749 因修改提交脚本而被身份保护拒绝。修复 bands.x 并行并保存电子子任务完成证据，使失败恢复复用相容的 PW 能带数据。

关联 [0018](../0018-epwflow-stage-workspaces/issue-draft.md) 和 [0019](../0019-shell-script-options/issue-draft.md)；无独立父 Issue，不改变物理参数、EPW pool 规则、预算及活动任务保护，不代提交科学作业。

## 验收标准

- [x] SC-001：32 MPI 的 bands.x 使用 pencil 分解，其他程序参数不被污染。
- [x] SC-002：PW 成功而 bands 失败后，恢复的 PW 调用次数仍为一次。
- [x] SC-003：旧数据经冻结来源、输入、日志、XML、波函数核验后复用；损坏或缺证据拒绝。
- [x] SC-004：脚本篡改仍拒绝，新尝试正常生成；回归及安装验证通过。
## 文档影响（Documentation Impact）

- [x] 更新 docs/overview/README.md、docs/overview/epwflow/README.md。
- [x] 更新模块手册和 MgB₂ 算例 README。
- [x] 新增本目录 [Spec](spec.md)、设计、任务及验收记录。

根 README 的入口和安装命令未变，无需重复修改；具体恢复行为在模块手册和算例 README 中更新。

线上状态不由本地技术完成代替；证据见 [验收记录](acceptance.md)。


本地技术验收依据：207 项分组回归、显式真实旧 PW 只读验证、规范构建与安装、入口和实际配置检查，详见 acceptance.md。真实修复后科学任务未提交，线上 Issue 状态未修改。

## 完整原始需求及实施授权

为什么我刚刚在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run提交了epw_wannier --stage wannier -s然后跑了一下就结束了，是不是报错了, 你来看看/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/5.epw-wannier里面的报错内容？看看是不是需要修改代码，如果需要使用 issue 和 spec 来新建需求修改。

## 用户批准并要求实施的完整方案

PLEASE IMPLEMENT THIS PLAN:
# 修复 QE 能带后处理失败与续算

### 1. 已确认的原因

两次作业均失败，但原因不同：

- **1306688**：`pw.x` 能带计算已成功，随后 `bands.x` 报错：`there are processes with no planes`。32 个 MPI 进程在当前 FFT 分解下出现空进程，Wannier 尚未开始。详见 [bands.err](/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/3.qe-electronic/qe_eband/bands.err:3)。
- **1306749**：修改后的 `submit.sh` 与冻结记录的哈希不符，启动检查拒绝执行。详见 [作业错误日志](/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/5.epw-wannier/slurm-1306749.err)。

需要修改代码；这次 FFT 错误与 `set -euo pipefail` 无关。

### 2. 需求与实现

- 检查编号后，新建本地 **0020，type:bug** Issue／Spec，关联 0018 和已有的 0019；保存原始需求、设计、任务及验收标准。线上 Gitee 仍由用户管理。
- `bands.x` 启动时增加 QE 支持的 `-pd .true.`，保留当前 **32 MPI**。不把 PW／EPW 的 pool 参数直接传给它，不修改物理网格、能窗或截断能；CLI 和 TOML 接口保持不变。
- 将电子阶段拆分为可校验的原生任务完成记录：成功的 `pw.x`、`bands.x`、DOS 分别记录输入、上游身份和产物哈希。后处理失败时保留有效数据，只重试缺失任务。
- 对本次已有能带结果，结合冻结配置、原生输入、退出码、完成日志及 XML／波函数完整性验证后接入续算；验证不足时明确停止，不能仅凭 `JOB DONE` 宣布可复用。
- 保留提交脚本哈希保护，改善错误提示。修复后由公开命令生成新尝试，不手改历史脚本或清单。

### 3. 验证

- 验证 `bands.x` 收到 32 MPI 和 pencil 分解参数；其他程序不误收这些参数。
- 模拟“PW 成功、bands 失败”，再次恢复时确认 PW 调用次数仍为一次；参数不相容或产物损坏时拒绝错误复用。
- 验证修改冻结脚本仍被拒绝，新生成的脚本可正常通过启动检查。
- 运行相关 EPW／QE 回归和安装检查；与 0019 衔接，确认生成脚本没有严格选项且显式传播失败。
- 对现有成功 QE 数据做只读核验和哈希记录。协议测试与真实科学验收分别记录，本次不代提交科学作业。

### 4. 交付与恢复

更新 Issue／Spec、Overview、模块手册和算例 README；通过规范构建入口更新 `bin/epwflow`，区分源码验证与安装验证。

0019 与本缺陷分别验证、分别提交，只纳入各自修改，不推送。修复后的恢复路径应为：复用 SCF、NSCF、PH 及已成功的 PW 能带数据，从 `bands.x` 后处理继续，再进入 Wannier。


关联后续缺陷：[0021 DOS FFT 与旧尝试恢复](../0021-qe-dos-pencil-resume/issue-draft.md)。1307064 的 bands 已完成，DOS 空进程及旧脚本重提问题单独验证，不改写本需求历史验收结论。
