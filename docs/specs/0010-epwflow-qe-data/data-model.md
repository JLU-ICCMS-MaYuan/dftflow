# 数据模型：QE 三模式数据准备

所有持久结构 schema_version=1；未知版本拒绝。依赖以角色和 SHA-256 绑定，不靠可变绝对路径判断同一科学数据。

## QeManifest

字段：`schema_version,run_id,fingerprints,scf,states,qpoints,phonon_files,validation`。

约束与生命周期：完整校验后发布；import 前重验。缺失必需值拒绝，数值必须有限。

## QPoint

字段：`global_index,local_index,directory,crystal,cartesian,alat,reciprocal_basis,source_dyn0`。

约束与生命周期：global_index 唯一1起始；物理 q 与响应基底配对。缺失必需值拒绝，数值必须有限。

## ElectronicStates

字段：`k_grid,kpoints,weights,nbnd,eigenvalues,wavefunctions,fermi,layout`。

约束与生命周期：完整均匀网格，不可用路径代替。缺失必需值拒绝，数值必须有限。

## PhResponse

字段：`q_id,dyn,patterns,dvscf,control_metadata,format,perturbations`。

约束与生命周期：所有文件对应同一物理 q；源不可写。缺失必需值拒绝，数值必须有限。

## 关系、校验与所有权

RunBinding→StageAttempt→Artifact，后续 manifest 引用上游身份；基础账本按 campaign_id 跨运行共享。禁止以相同文件名合并不同来源。全局 q/局部 q、band/k/frequency 分别是显式键，不能用表格行位置替代物理映射。

原始科学数据不可被收集、绘图或导入改写；派生文件写入独立目录。失败记录和历史尝试保留；成功标记发布前先验证所有必需文件和数值。状态损坏、未知格式、未决提交和来源变化均阻止相应副作用，不自动删除重建。
