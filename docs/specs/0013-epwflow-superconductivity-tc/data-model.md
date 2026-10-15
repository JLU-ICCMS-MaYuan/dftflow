# 数据模型：两种超导求解与受限 Tc 搜索

所有持久结构 schema_version=1；未知版本拒绝。依赖以角色和 SHA-256 绑定，不靠可变绝对路径判断同一科学数据。

## SolveAttempt

字段：`mode,temp_centikelvin,type,seed,attempt_id,source_digest,job,niter,residual,status`。

约束与生命周期：每次仅一个模式和温度；严格计数。缺失必需值拒绝，数值必须有限。

## GapData

字段：`temperature,frequency,band,kpoint,weight,energy,delta_mev,z`。

约束与生命周期：最低正频与完整数组关联，禁止缺点补零。缺失必需值拒绝，数值必须有限。

## TcSearch

字段：`mode,method,range,precision,attempt_limit,interval,classified_points,pending,status,reason`。

约束与生命周期：pending/running/located/not_bracketed/classification_uncertain/inconsistent/solver_failed/attempt_limit/resource_limit/scheduler_unknown。缺失必需值拒绝，数值必须有限。

## Classification

字段：`solver_valid,seed_agreement,d_inf,d0_inf,eigenvalue,side,thresholds`。

约束与生命周期：有效性与物理分类分别表示，unknown 不能成为端点。缺失必需值拒绝，数值必须有限。

## 关系、校验与所有权

RunBinding→StageAttempt→Artifact，后续 manifest 引用上游身份；基础账本按 campaign_id 跨运行共享。禁止以相同文件名合并不同来源。全局 q/局部 q、band/k/frequency 分别是显式键，不能用表格行位置替代物理映射。

原始科学数据不可被收集、绘图或导入改写；派生文件写入独立目录。失败记录和历史尝试保留；成功标记发布前先验证所有必需文件和数值。状态损坏、未知格式、未决提交和来源变化均阻止相应副作用，不自动删除重建。
