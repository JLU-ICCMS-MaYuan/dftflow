# 0020：QE 电子后处理并行与续算

关联 [本地 Issue](issue-draft.md)；线上链接由用户管理。原始需求完整保存在 [原文](original-request.md)，本页通过引用复用，不以转述替代原文。

## 用户故事与验收场景

US1：保持当前分配，用 bands.x 的 pencil 分解完成能带后处理。32 MPI 参数保留，不复制 PW 的 pool 参数。

US2：电子后处理失败后从失败任务继续。PW 成功、bands 失败，再次提交只执行 bands、DOS 及后续；改变科学输入时更新受影响数据，证据损坏不能静默复用。

US3：旧失败现场有可信的冻结来源时复用原 PW 结果。缺来源、退出码非零、日志失败、XML 点表不符或波函数缺失均停止。

## 功能要求

- FR-001：仅 bands.x 默认加入 `-pd .true.`；不引入新 CLI／TOML 开关，不降低 MPI 或修改物理参数。
- FR-002：分别保存 PW 能带、bands、DOS 原生任务完成记录，绑定科学参数、真实上游产物、程序和输入／输出哈希；MPI 资源数量不进入科学复用键。
- FR-003：电子阶段失败保留同科学请求的数据，已成功任务重验后跳过，失败日志归档；不覆盖相容 PW 输入与保存树。
- FR-004：旧 PW 结果核验冻结配置及来源身份、原生输入一致性、零退出码、完成日志、几何、点表、带数、与 SCF 快照一致的电荷密度和 portable 波函数；来源不足明确阻断。原始来源及上游数据不修改。
- FR-005：保持活动任务、预算和冻结脚本身份保护，给出重新生成新尝试的可操作报错；不得通过编辑旧清单绕过。
- FR-006：更新操作文档，经规范构建入口安装三个 EPW 程序，记录源码、协议、真实只读和安装验证边界。

## 成功标准

SC-001～004 见 Issue 验收项；每项必须在 acceptance.md 提供实际依据。协议替身通过不表示真实 bands.x 或 Wannier 科学验收通过；本轮不提交真实计算。

## 边界与默认

沿用 0019 的无自动严格选项脚本；CLI、TOML、MPI 分配和 EPW pool 约束不变。保留旧现场及失败证据；不运行真实科学程序、不推送、不操作线上 Issue。KISS／DRY：复用共享执行、XML、快照和哈希工具，仅新增电子原生任务记录，不建设第二套调度器。

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
