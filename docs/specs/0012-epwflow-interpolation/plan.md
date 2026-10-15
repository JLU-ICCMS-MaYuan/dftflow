# 实施计划：Wannier 化与 EPW 插值

**Issue**：[本地 0012](issue-draft.md)；**日期**：2026-10-15；**Spec**：[spec.md](spec.md)。

## 摘要与技术上下文

Rust 2021 edition，当前工具链 rustc 1.99.0；现有 Cargo workspace，新增 epwflow 家族 crate。使用 clap、serde、serde_json、toml、anyhow、regex；哈希/进程控制按基础契约实现。科学计算调用现有 QE/EPW，绘图复用 plotters/svg2pdf。存储为受控文件与 JSON，不引入服务或数据库。目标首先 Linux x86_64 的 sugon-amd，macOS arm64 配置保持独立，未实测不写通过。规模是单用户的一次受限验收活动，可跨多个运行目录共享预算。

性能与容量遵循 Spec 的次数、并发、8 节点小时和 100 GiB；无未经测量的速度承诺。真实路径和输出仅存忽略目录。生产实现只用 Rust/Bash，测试可使用仓库既有 Python 测试习惯，但不形成 Python 运行依赖。

## 设计与数据所有权

目标适配器以 QE 7.4.1 配套 EPW 的真实运行版本为准。严格分开首次 Wannier/粗矩阵、epwread 重读、质量验证点、均匀细网格 ephwrite、谱函数重读。epbread/epbwrite 与 epwread/epwwrite 不是同一缓存。生成的 wdata 只来自类型化允许字段，不透传任意文本。

Wannier 输出核对解纠缠和局域化两种迭代；解析实际 .wout 的原生收敛信息、最终展宽和旋转矩阵数量。投影按轨道展开，不按字符串条数计数。窗口统一转换为 QE 电子能量参考并保存原参考/Fermi 来源，冻结态数逐 k 核对。

质量参考固定在输入快照中；至少四个粗 k 网格外点与两个粗 q 网格外点，参考通过独立 NSCF/PH 任务从 SCF 副本产生，计入共同预算。比较覆盖全部声明能窗的态，不为达到阈值删点。电子/声子误差分别计算 max/RMS，固定单位和能带/简并组排序；失败不交付下游。

细网格检查 k/q 每方向整除、实际 k+q 映射、fsthick 内态集合和矩阵记录。目标 EPW 的 ephmat 数据包含 freq、egnv、ikmap 及按并行布局划分的矩阵文件；完整角色清单依据 io_supercond 中的读取例程建立，并以读取所有记录验收，不依赖单个文件名。格式、字节序/记录标记、pool 布局写入适配身份；未知格式拒绝。

谱函数使用目标 supercond_driver 中 liso=false/laniso=false 的实际数据读取与 find_a2f 路径，不调用温度能隙求解。经验 Tc 打印不进入 Eliashberg 结果。规范表保存 meV、每个展宽与累计 λ；按照同版本离散求积复核。虚频原值保留，按 Spec 的 -1 cm^-1 边界阻断，不自动改截断。

粗数据/细数据/谱/质量报告分别有依赖哈希；新配置创建新运行并 import 相容节点，温度与 μ* 不进入无关数据哈希。独立手工输入验证首次/重读；不以重排原始矩阵强求逐字节一致。

## 源代码结构

本项新增模块 epwflow/src/interpolation/，公开入口由 epwflow/src/main.rs 统一分派；共享状态、预算与路径只由 foundation 拥有，后续模块不得复制规则。测试位于 epwflow/tests/interpolation.rs。具体 FR 到文件见下表；尚未存在的文件为明确计划触点，不作为已实现证据。

## Feature 文档结构

spec.md、plan.md、research.md、data-model.md、contracts/interface.md、tasks.md、quickstart.md、checklists/requirements.md、acceptance.md；数据/CLI 均适用，因此不省略条件文档。

## 质量门

| 来源 | 约束 | 满足方式 | 状态 |
| --- | --- | --- | --- |
| 用户确认 | 完整草案与边界 | spec.md 原文/确认记录，保留 FR/SC | 已核查，见共同分析 |
| F1–F5 | 共享契约一致 | 下游链接基础契约；单一账本与阶段状态 | 已核查，见共同分析 |
| AGENTS | 已授权 Rust/Bash、隔离提交 | 不纳入无关 Si 删除；文档完成后自动提交 | 适用 |
| 用户本地约定 | 不操作线上 Issue | 本地 issue-draft 双向链接，不伪造远程验证 | 远程门按用户约定不适用 |
| 真实验收 | 不放宽/不造证据 | acceptance 保持未运行，真实通过后更新 | 尚未执行 |

## 需求到设计与验证的映射

| 来源 | 任务 | 设计组件/接口 | 验证方式 |
| --- | --- | --- | --- |
| FR-001 | T003/T004 | epwflow/src/interpolation/cli.rs | epwflow/tests/interpolation.rs |
| FR-002 | T005/T006 | epwflow/src/interpolation/epw_config.rs | epwflow/tests/interpolation.rs |
| FR-003 | T007/T008 | epwflow/src/interpolation/adapter.rs | epwflow/tests/interpolation.rs |
| FR-004 | T009/T010 | epwflow/src/interpolation/wannier.rs | epwflow/tests/interpolation.rs |
| FR-005 | T011/T012 | epwflow/src/interpolation/coarse.rs | epwflow/tests/interpolation.rs |
| FR-006 | T013/T014 | epwflow/src/interpolation/quality.rs | epwflow/tests/interpolation.rs |
| FR-007 | T015/T016 | epwflow/src/interpolation/fine.rs | epwflow/tests/interpolation.rs |
| FR-008 | T017/T018 | epwflow/src/interpolation/spectral.rs | epwflow/tests/interpolation.rs |
| FR-009 | T019/T020 | epwflow/src/interpolation/compare.rs | epwflow/tests/interpolation.rs |
| FR-010 | T021/T022 | epwflow/src/interpolation/reuse.rs | epwflow/tests/interpolation.rs |
| FR-011 | T023/T024 | epwflow/src/interpolation/scheduler.rs | epwflow/tests/interpolation.rs |
| FR-012 | T025/T026 | epwflow/src/interpolation/manifest.rs | epwflow/tests/interpolation.rs |
| FR-013 | T027/T028 | epwflow/README.md | epwflow/tests/interpolation.rs |
| SC-001 | T029 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-002 | T030 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-003 | T031 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-004 | T032 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-005 | T033 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-006 | T034 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-007 | T035 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-008 | T036 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |

## 阶段与依赖

准备和基础协议先完成，随后按 tasks 的测试→实现→验证顺序执行。当前 Feature 完成独立验收、quickstart、Overview 回写后才推进依赖阶段；不得用尚未实现的下游编排替代本项接口验收。全部文档门通过前禁止业务代码。

## 必要复杂度

| 复杂度 | 原因 | 拒绝的简化 |
| --- | --- | --- |
| 版本化清单和依赖指纹 | 固定路径不保证固定内容，复用有真实数据风险 | 仅检查文件存在或 JOB DONE |
| 两种状态：操作与科学结果 | 提交/绘图成功不等于科学成功 | 一个完成布尔值 |
| 独立参考与固定阈值 | 相同错误流程可产生自洽假结果 | 用自己输出生成唯一期望值 |
