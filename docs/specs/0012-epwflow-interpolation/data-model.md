# 数据模型：Wannier 化与 EPW 插值

所有持久结构 schema_version=1；未知版本拒绝。依赖以角色和 SHA-256 绑定，不靠可变绝对路径判断同一科学数据。

## InterpolationManifest

字段：`f2_digest,wannier_digest,coarse,fine,spectra,quality,mode_readiness`。

约束与生命周期：粗细谱各自依赖图，不混用基底。缺失必需值拒绝，数值必须有限。

## QualityReport

字段：`points,energy_reference,matching,max_error,rms_error,coverage,thresholds,status`。

约束与生命周期：缺参考/覆盖不足即无效。缺失必需值拒绝，数值必须有限。

## FineData

字段：`kpoints,qpoints,energies,frequencies,weights,kqmap,ephmat,layout`。

约束与生命周期：矩阵维度和掩码与索引一致。缺失必需值拒绝，数值必须有限。

## Spectrum

字段：`smearing_id,energy_mev,a2f,lambda_integral,lambda_sum,cutoff`。

约束与生命周期：各展宽分列；零频规则明确。缺失必需值拒绝，数值必须有限。

## 关系、校验与所有权

RunBinding→StageAttempt→Artifact，后续 manifest 引用上游身份；基础账本按 campaign_id 跨运行共享。禁止以相同文件名合并不同来源。全局 q/局部 q、band/k/frequency 分别是显式键，不能用表格行位置替代物理映射。

原始科学数据不可被收集、绘图或导入改写；派生文件写入独立目录。失败记录和历史尝试保留；成功标记发布前先验证所有必需文件和数值。状态损坏、未知格式、未决提交和来源变化均阻止相应副作用，不自动删除重建。
