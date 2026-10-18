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

## 2026-10-19：独立比较包和图表状态

新增独立 `comparison/manifest.json`（schema_version=1），由 [0012 成对清单](../0012-epwflow-interpolation/data-model.md) 转入绘图所需表格与元数据，不依赖外部配置重解释。保留来源哈希和上游清单身份；移动后可重绘，原始来源不可访问时仅标为未重新核验来源。

| 实体 | 字段与约束 |
| --- | --- |
| 比较数据包 | 成对曲线／DOS／参考点表、metadata、metrics、文件哈希；完整性、质量、缺项、diagnostic_only 和 workflow_complete 分开 |
| 叠图序列 | kind、method、projection、segment、点序、单位、颜色、线型、绘制顺序；QE 实线在前，EPW 虚线在后 |
| 参考点 | qe_ph_direct、实际坐标／模式及哈希；与 qe_matdyn 连续曲线分别索引 |
| 叠图产物 | kind、png/svg/pdf、数据摘要、图层顺序、面板状态、文件哈希；四个独立 kind 加 comparison_overview |

缺失值沿用空值／null，质量不合格与缺数据是不同状态。未取得元素或严格轨道投影时，完整叠图交付保持未完成；仍可导出有效总量曲线作部分诊断，但不得将其登记为元素／轨道曲线。2×2 总览各面板引用同一独立图数据，不允许另取来源或不同能窗。
