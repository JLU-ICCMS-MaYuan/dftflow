# 原始需求记录

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
