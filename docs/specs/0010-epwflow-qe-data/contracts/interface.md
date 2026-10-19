# CLI 与数据契约：QE 三模式数据准备

**当前 QE 清单版本**：2（历史版本 1 仅只读兼容）；**需求**：[Spec](../spec.md)；**模型**：[数据模型](../data-model.md)。

## 公开接口

完整命令、参数、默认值、单位与行为以 Spec 的 FR-001 为规范；本接口不另改命名。所有 --help 均无科学执行副作用。参数使用 argv，显式路径按所属配置或命令规则解析，不 eval。

共同执行/错误/持久协议由 [0008 基础契约](../../0008-epwflow-foundation/contracts/interface.md) 唯一拥有；科学字段由各自 Spec 定义，全部进入同一 TOML schema_version=1。check/status 的 --json 返回 schema_version、operation_status、workflow_status、diagnostics；status 查询成功不代表科学完成。

## 输入输出与错误

读取经过绑定的 input-init.toml 及前置 manifest；每次读取确认版本和指纹。缺字段/类型错误/单位不符返回非零并报告字段路径；状态未知、预算拒绝和数据无效用独立错误类别，不自动重试。可预期的未定位或部分结果以结构化状态表示，不填伪造数值。

F1 返回配置/运行绑定与阶段协议，F2 返回 qe-data/manifest.json，F3 返回 interpolation/manifest.json，F4 返回 superconductivity/manifest.json 与 tc.json，F5 返回 results/manifest.json。每个 manifest 列出完整角色清单、尺寸、哈希、生产阶段、上游摘要与验证状态。文件记录采用相对 artifact_root 的路径；绝对机器路径只存在实际运行的忽略目录。拒绝路径穿越和指向来源的可写链接。

## 本项数据字段

- `QeManifest`：schema_version,run_id,fingerprints,scf,states,qpoints,phonon_files,validation；完整校验后发布；import 前重验。
- `QPoint`：global_index,local_index,directory,crystal,cartesian,alat,reciprocal_basis,source_dyn0；global_index 唯一1起始；物理 q 与响应基底配对。
- `ElectronicStates`：k_grid,kpoints,weights,nbnd,eigenvalues,wavefunctions,fermi,layout；完整均匀网格，不可用路径代替。
- `PhResponse`：q_id,dyn,patterns,dvscf,control_metadata,format,perturbations；所有文件对应同一物理 q；源不可写。

## 权限、兼容与副作用

仅使用当前用户已有权限。不得自动修改系统库、清除预算、覆盖来源、取消无关作业或推送仓库。默认 outdir 是按配置目录解析的基址；实际保存树为 <outdir>/<run_id>/<stage_id>。同一绑定内路径一经准备不变，用户自定义优先，工作目录与保存目录分别显示。

文件接口升级须新版本并提供显式兼容检查，不能静默读取未知格式。失败输出可保留用于诊断，但不能充作下游成功数据。查询和错误输出不得泄露无关配置或认证内容。

## 必需产物角色和交接

scf_xml、charge_density、wavefunction、dyn0、dyn_global_i、pattern_global_i、dvscf_global_i、phsave_control；额外赝势角色由已验证目标适配器声明，未支持组合拒绝。NSCF 消费完整 scf_snapshot，PH 消费各自副本；F3 只消费 published qe-data manifest。

Artifact={role,relative_path,byte_size,sha256,producer_stage,input_digest,format_version}；validation={status,checks,units,thresholds,evidence}。只有 status=valid 的必需角色完整才发布该阶段 manifest。format_version 未知或文件集合与已验证适配器不符即拒绝；不靠 glob 命中一个文件放行。

## 原生版本 2 保存树交接

QeManifest 新增 artifact_root 与 artifacts。新运行只发布版本 2，逐文件核对相对路径、完整文件集合、大小、SHA-256 及原生 q／NSCF 数值；F3 与统一入口拒绝仅有版本 1 valid 标签的历史文件。独立 check 可展示 legacy_unverified，不把旧清单当作可执行来源。

scf.snapshot_dir 为完整 outdir 快照，snapshot-files.json 固定全部内容；NSCF、PH 初始化和各 q 从该树独立复制。scf／nscf／ph 日志和错误日志均在对应阶段目录归档，阶段函数只在同一已登记分配内同步执行。nosplit 直接产生 dyn0 和完整响应，不额外 only_init。

PH 格式标识 qe7.4.1-ph-xml-c16，当前限定非磁性标量、portable 波函数和小端复数 REAL(8) 响应。save 包含 prefix.dyn0.xml、prefix.dyn_qN.xml、prefix.dvscf_qN 及 prefix.phsave/patterns.N.xml、control_ph.xml。coords 只修改派生 patterns 的 QPOINT_NUMBER 为全局 N，保留原始文件与复扰动基底，未知或截断布局拒绝。

import 只接受经过原生重验的版本 2 来源，按清单角色复制目录成员并重验，保存重新定位的独立 imported/qe-data/manifest.json；不自动运行科学程序。把来源根目录或其子目录用作导入目标时拒绝。导入报告本身不代表 F1 已完成后续阶段恢复。
