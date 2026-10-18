# CLI 与数据契约：图表、真实对照与安装交付

**版本**：1；**需求**：[Spec](../spec.md)；**模型**：[数据模型](../data-model.md)。

## 公开接口

完整命令、参数、默认值、单位与行为以 Spec 的 FR-001 为规范；本接口不另改命名。所有 --help 均无科学执行副作用。参数使用 argv，显式路径按所属配置或命令规则解析，不 eval。

共同执行/错误/持久协议由 [0008 基础契约](../../0008-epwflow-foundation/contracts/interface.md) 唯一拥有；科学字段由各自 Spec 定义，全部进入同一 TOML schema_version=1。check/status 的 --json 返回 schema_version、operation_status、workflow_status、diagnostics；status 查询成功不代表科学完成。

## 输入输出与错误

读取经过绑定的 input-init.toml 及前置 manifest；每次读取确认版本和指纹。缺字段/类型错误/单位不符返回非零并报告字段路径；状态未知、预算拒绝和数据无效用独立错误类别，不自动重试。可预期的未定位或部分结果以结构化状态表示，不填伪造数值。

F1 返回配置/运行绑定与阶段协议，F2 返回 qe-data/manifest.json，F3 返回 interpolation/manifest.json，F4 返回 superconductivity/manifest.json 与 tc.json，F5 返回 results/manifest.json。每个 manifest 列出完整角色清单、尺寸、哈希、生产阶段、上游摘要与验证状态。文件记录采用相对 artifact_root 的路径；绝对机器路径只存在实际运行的忽略目录。拒绝路径穿越和指向来源的可写链接。

## 本项数据字段

- `ResultManifest`：schema_version,run_id,source_digests,tables,figures,status,source_verification；移动可重绘；原始不可访问不伪造重新验证。
- `ComparisonCell`：ph_mode,solver_mode,reference,input_hashes,metrics,tolerances,status,evidence；六个唯一组合，缺任一项不得整体成功。
- `InstallManifest`：source_commit,diff_digest,platform,source_hashes,packaged_hashes,entries,node_checks；逐文件状态；无默认历史二进制。
- `Figure`：kind,mode,temp,smearing,units,data_hash,format,path,status；只画有效量；状态图不冒充科学曲线。

## 权限、兼容与副作用

仅使用当前用户已有权限。不得自动修改系统库、清除预算、覆盖来源、取消无关作业或推送仓库。默认 outdir 是按配置目录解析的基址；实际保存树为 <outdir>/<run_id>/<stage_id>。同一绑定内路径一经准备不变，用户自定义优先，工作目录与保存目录分别显示。

文件接口升级须新版本并提供显式兼容检查，不能静默读取未知格式。失败输出可保留用于诊断，但不能充作下游成功数据。查询和错误输出不得泄露无关配置或认证内容。

## 必需产物角色和交接

summary、a2f、coupling、gap_summary、gap_states、tc、report、figure_png、figure_svg、figure_pdf；各图由对应规范化表产生，figure_manifest 哈希不反馈修改原始科学文件。

Artifact={role,relative_path,byte_size,sha256,producer_stage,input_digest,format_version}；validation={status,checks,units,thresholds,evidence}。只有 status=valid 的必需角色完整才发布该阶段 manifest。format_version 未知或文件集合与已验证适配器不符即拒绝；不靠 glob 命中一个文件放行。

## 2026-10-19：比较结果包与图表契约（待实现）

沿用已规划的 `epwflow collect -w W [--output R]`、`epwflow plot --results R [--output D] [--formats png,svg,pdf]` 和 `epwflow results check --results R [--json]`，不新增平行命令。当前落档不表示这些公开入口已经接通；实际实现／验收以 acceptance.md 为准。

collect 允许没有 F4 清单的有效 F3 诊断输入；输出 R/comparison，保留 [0012 契约](../../0012-epwflow-interpolation/contracts/interface.md) 的五类 CSV、metadata.json、metrics.json 和独立 manifest.json。上游原始数据只读，图表所需数据全部随包保存。原结果主清单兼容性保持原约定，新比较包用独立版本；旧包缺此数据不能通过新增叠图验收。

| 图种基名 | 内容 | 图层顺序 |
| --- | --- | --- |
| electronic_bands_comparison | 共同有效窗内 QE／Wannier 能带 | QE 实线→EPW 虚线 |
| electronic_dos_comparison | 总量、Mg/B 元素及严格轨道电子 DOS 分面／图例 | 同投影 QE 实线→EPW 虚线 |
| phonon_dispersion_comparison | QE q2r/matdyn 与 EPW 连续曲线，独立 ph.x 点 | QE 实线→EPW 虚线；直接点独立标识 |
| phonon_dos_comparison | 总量与 Mg/B 元素声子 DOS | 同元素 QE 实线→EPW 虚线 |
| comparison_overview | 上述四类的 2×2 总览 | 复用相同图层语义 |

图种各输出 PNG／SVG／PDF，登记在绘图产物清单中；默认 D=R/figures。metadata 的能窗、投影标签、E_F、单位、核／宽度与归一化必须同时进入图例或可定位的说明；不画跨路径断点连线。

缺可靠投影或参考时，plot 只能生成标明缺项的部分面板；结果检查明确 comparison_complete=false。篡改、未知版本、非法单位／非有限值或未知文件覆盖冲突返回非零。完整但质量失败的结果允许操作成功并输出诊断图，同时 quality_status=failed、workflow_complete=false；操作成功绝不等同科学验收通过。

collect／plot／results check 不运行 QE、EPW 或 sbatch，不创建科学任务；缺参考计算应由受预算控制的 F3 明确执行。重绘仅更新本工具拥有的图像和绘图清单，comparison 表格和科学源文件内容必须保持不变。
