# 数据模型：图表、真实对照与安装交付

所有持久结构 schema_version=1；未知版本拒绝。依赖以角色和 SHA-256 绑定，不靠可变绝对路径判断同一科学数据。

## ResultManifest

字段：`schema_version,run_id,source_digests,tables,figures,status,source_verification`。

约束与生命周期：移动可重绘；原始不可访问不伪造重新验证。缺失必需值拒绝，数值必须有限。

## ComparisonCell

字段：`ph_mode,solver_mode,reference,input_hashes,metrics,tolerances,status,evidence`。

约束与生命周期：六个唯一组合，缺任一项不得整体成功。缺失必需值拒绝，数值必须有限。

## InstallManifest

字段：`source_commit,diff_digest,platform,source_hashes,packaged_hashes,entries,node_checks`。

约束与生命周期：逐文件状态；无默认历史二进制。缺失必需值拒绝，数值必须有限。

## Figure

字段：`kind,mode,temp,smearing,units,data_hash,format,path,status`。

约束与生命周期：只画有效量；状态图不冒充科学曲线。缺失必需值拒绝，数值必须有限。

## 关系、校验与所有权

RunBinding→StageAttempt→Artifact，后续 manifest 引用上游身份；基础账本按 campaign_id 跨运行共享。禁止以相同文件名合并不同来源。全局 q/局部 q、band/k/frequency 分别是显式键，不能用表格行位置替代物理映射。

原始科学数据不可被收集、绘图或导入改写；派生文件写入独立目录。失败记录和历史尝试保留；成功标记发布前先验证所有必需文件和数值。状态损坏、未知格式、未决提交和来源变化均阻止相应副作用，不自动删除重建。
