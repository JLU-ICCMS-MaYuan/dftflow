# 数据模型：配置、环境检查与受控执行基础

所有持久结构 schema_version=1；未知版本拒绝。依赖以角色和 SHA-256 绑定，不靠可变绝对路径判断同一科学数据。

## RunBinding

字段：`schema_version,run_id,config_sha256,structure,pseudos,programs,environment,work_dir,outdir_base,budget_id`。

约束与生命周期：prepare 唯一建立；input-init.toml 不可修改；新输入新绑定。缺失必需值拒绝，数值必须有限。

## StageAttempt

三程序工作流的实际尝试记录可选保存 `mpi_override: u32`，仅显式粗阶段恢复写入；原固定快照保持不变。后续尝试继承该字段，状态展示实际 `effective_mpi`。科学请求身份不因该受限执行参数变化而重建，任何其他固定字段或数据身份变化仍拒绝复用。

字段：`stage_id,attempt_id,dependencies,input_digest,state,job_id,cluster,validator,artifacts,error`。

约束与生命周期：ready→submission_pending→submitted→running→validating→succeeded；失败/unknown/blocked 不放行。缺失必需值拒绝，数值必须有限。

## BudgetLedger

字段：`schema_version,campaign_id,limits,spent_node_seconds,reservations,allocations,lanes,storage_roots,revision`。

约束与生命周期：同一 allocation 仅结算一次；未知保留预留；不提供 reset。缺失必需值拒绝，数值必须有限。

## EnvironmentEvidence

字段：`executable_sha256,launcher,env_script_sha256,host,observed_version,time,status`。

约束与生命周期：版本证据与文件身份绑定，变化即失效。缺失必需值拒绝，数值必须有限。

## 关系、校验与所有权

RunBinding→StageAttempt→Artifact，后续 manifest 引用上游身份；基础账本按 campaign_id 跨运行共享。禁止以相同文件名合并不同来源。全局 q/局部 q、band/k/frequency 分别是显式键，不能用表格行位置替代物理映射。

原始科学数据不可被收集、绘图或导入改写；派生文件写入独立目录。失败记录和历史尝试保留；成功标记发布前先验证所有必需文件和数值。状态损坏、未知格式、未决提交和来源变化均阻止相应副作用，不自动删除重建。
