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

## 2026-10-19：成对参考、投影与 DOS 设计

本段落实 FR-014/015；是待实施设计，不是已验证功能。数据由 F3 唯一拥有，0014 只收集／绘图，0015 核验真实结果。复用 qf-input 的 QE 输入、qf-crystal 的晶胞／倒空间、qf-phonon 的 q2r/matdyn 和现有绘图数据能力；EPW 版本适配、投影算符及配对校验集中在 interpolation，避免维护另一套 QE 工作流。

1. 准备时冻结电子／声子完整路径、分段与坐标单位、DOS 均匀网格、共同能量／频率轴、参考 E_F、展宽及投影定义。路径和 DOS 网格是不同数据集；原质量验证点另存，不能用少量验证点连接出完整色散。
2. 电子曲线使用直接 QE 能量与同点 Wannier 插值。共同有效窗至少覆盖 E_F±0.5 eV 和全部 Fermi 窗口；检查 DOS 展宽贡献需要的边界态。只有 Wannier 子空间覆盖该窗全部参考态时，才可标为该窗的总 DOS；不增加轨道、不自由平移或裁掉错误能带。
3. QE 电子 PDOS 读取同一 NSCF 规范下的复原子投影与原子／轨道标签。以 Wannier 旋转及解纠缠矩阵将完整投影算符变换到同一 Wannier 基底，再插值到 DOS 网格。保留非对角项和相位；只有 PDOS 平方权重、键中心 Wannier 标签或不同波函数规范的数据不足以实现原子投影。具体原生格式必须经目标版本适配和粗网格重构验证，失败时报告投影不可用。
4. 声子连续 QE 参考来自 q2r/matdyn；直接 ph.x 点独立保存、标记并继续执行原质量门。元素声子权重从同质量／原子顺序／ASR 的质量加权动力学矩阵归一化本征矢获得。禁止把 EPW 按模式输出的列按元素改名。
5. 共同 DOS 累积器处理两侧相同网格、权重、核函数与展宽，复用单位转换和表格写出。标准高斯约定明确为 exp[-(x/σ)²]/(σ√π)，若原输入为其他约定则显式换算并记录；不得无记录改变已有物理展宽。冻结覆盖范围及尾部处理，不对两条 DOS 分别缩放以制造一致。
6. 校验通过的数据写独立 comparison 清单；质量失败但数据完整时只发布诊断状态，不发布 F4 就绪。必要参考计算与存储使用 F1 既有账本；预算不足保留缺项，不能为画图启动受质量门阻断的细网格 EPC 或超导。

| 来源 | 任务 | 设计／事实来源 | 验证 |
| --- | --- | --- | --- |
| FR-014、SC-009 | T039/T040/T043 | QE 原始路径／均匀网格、EPW 原生插值、统一坐标和固定输入 | 路径／单位／零点／覆盖／来源负向测试，真实参考核查 |
| FR-015、SC-010 | T041/T042/T043 | 原子投影算符、Wannier 变换、质量加权本征矢与共同 DOS 累积 | 相位／模式误标／缺投影／积分测试，0015 真实对照 |

新数据文件与字段见 [契约](contracts/interface.md)；表格及投影实体见 [数据模型](data-model.md)。实现路径为计划触点，不以文件存在证明功能完成。
