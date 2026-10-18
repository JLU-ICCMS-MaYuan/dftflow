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

## 2026-10-19：成对数据与投影实体

下列为 FR-014/015 的待实施实体；独立 comparison 清单使用自己的 schema_version=1，不把新必需字段静默塞入旧版本主清单。

| 实体 | 必需字段与校验 |
| --- | --- |
| 成对路径 | method、reference_kind、segment_id、point_id、晶体／QE 笛卡尔坐标及换算依据、弧长、band_or_mode、原始值／规范值；逐点对应，断点不跨线 |
| DOS 网格与表 | method、轴、density、total/element/orbital、元素／轨道标签、网格／权重身份、展宽核与宽度、单位、每晶胞归一化；区分总量与未完备投影和 |
| 电子原子投影 | atom_id、element、orbital、复投影矩阵、Bloch 规范、排除带映射、解纠缠／旋转身份及覆盖窗；禁止只凭 Wannier 中心猜元素 |
| 声子元素权重 | q、mode、atom_id、质量、归一化本征矢及 ASR；每模原子权重和为 1，元素权重按原子求和 |
| 成对比较清单 | 来源角色与文件哈希、坐标／E_F／单位／展宽／积分元数据、数据完整性、数值质量、诊断状态和缺项；不等同 F3/F4 成功清单 |

电子投影保留同一 k 点完整算符 P_α；令 W 为包含解纠缠和局域化的 Bloch→Wannier 变换，使用 P_α^W = W† P_α W。其倒／实空间转换沿用实际 Wannier 规范、相位与插值权重，在细网格本征态上求期望值。粗网格重构、厄米性、规范和带映射是必要检查，不能通过裁掉负权重或重新缩放掩盖错误。电子原子投影可能不完备，必须报告 spilling／未投影部分；不要求元素投影强行等于总 DOS。

声子质量加权本征矢满足全原子笛卡尔分量平方和为 1；元素权重是该元素所有原子分量平方和。共同累积后各元素声子 DOS 逐点和应等于总 DOS；完整频率范围内积分为 3N（MgB₂ 为 9），有限范围、虚频、截断与核尾部另报，不通过重新归一化隐藏缺失。
