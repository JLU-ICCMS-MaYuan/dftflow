# CLI 与数据契约：Wannier 化与 EPW 插值

**版本**：1；**需求**：[Spec](../spec.md)；**模型**：[数据模型](../data-model.md)。

## 公开接口

完整命令、参数、默认值、单位与行为以 Spec 的 FR-001 为规范；本接口不另改命名。所有 --help 均无科学执行副作用。参数使用 argv，显式路径按所属配置或命令规则解析，不 eval。

共同执行/错误/持久协议由 [0008 基础契约](../../0008-epwflow-foundation/contracts/interface.md) 唯一拥有；科学字段由各自 Spec 定义，全部进入同一 TOML schema_version=1。check/status 的 --json 返回 schema_version、operation_status、workflow_status、diagnostics；status 查询成功不代表科学完成。

## 输入输出与错误

读取经过绑定的 input-init.toml 及前置 manifest；每次读取确认版本和指纹。缺字段/类型错误/单位不符返回非零并报告字段路径；状态未知、预算拒绝和数据无效用独立错误类别，不自动重试。可预期的未定位或部分结果以结构化状态表示，不填伪造数值。

F1 返回配置/运行绑定与阶段协议，F2 返回 qe-data/manifest.json，F3 返回 interpolation/manifest.json，F4 返回 superconductivity/manifest.json 与 tc.json，F5 返回 results/manifest.json。每个 manifest 列出完整角色清单、尺寸、哈希、生产阶段、上游摘要与验证状态。文件记录采用相对 artifact_root 的路径；绝对机器路径只存在实际运行的忽略目录。拒绝路径穿越和指向来源的可写链接。

## 本项数据字段

- `InterpolationManifest`：f2_digest,wannier_digest,coarse,fine,spectra,quality,mode_readiness；粗细谱各自依赖图，不混用基底。
- `QualityReport`：points,energy_reference,matching,max_error,rms_error,coverage,thresholds,status；缺参考/覆盖不足即无效。
- `FineData`：kpoints,qpoints,energies,frequencies,weights,kqmap,ephmat,layout；矩阵维度和掩码与索引一致。
- `Spectrum`：smearing_id,energy_mev,a2f,lambda_integral,lambda_sum,cutoff；各展宽分列；零频规则明确。

## 权限、兼容与副作用

仅使用当前用户已有权限。不得自动修改系统库、清除预算、覆盖来源、取消无关作业或推送仓库。默认 outdir 是按配置目录解析的基址；实际保存树为 <outdir>/<run_id>/<stage_id>。同一绑定内路径一经准备不变，用户自定义优先，工作目录与保存目录分别显示。

文件接口升级须新版本并提供显式兼容检查，不能静默读取未知格式。失败输出可保留用于诊断，但不能充作下游成功数据。查询和错误输出不得泄露无关配置或认证内容。

## 必需产物角色和交接

wannier_input、wannier_log、rotation、disentanglement（适用时）、epw_coarse_data、coarse_ephmat、quality_reference、quality_report、fine_frequencies、fine_eigenvalues、fine_kqmap、fine_ephmat_shard、spectral_table；从目标 EPW 读取例程确定文件记录集合并生成 schema，其中缺任一角色即不发布。F4 消费其谱函数或 fine 集合。

Artifact={role,relative_path,byte_size,sha256,producer_stage,input_digest,format_version}；validation={status,checks,units,thresholds,evidence}。只有 status=valid 的必需角色完整才发布该阶段 manifest。format_version 未知或文件集合与已验证适配器不符即拒绝；不靠 glob 命中一个文件放行。
