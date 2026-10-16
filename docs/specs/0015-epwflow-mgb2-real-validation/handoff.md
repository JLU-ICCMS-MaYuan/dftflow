# 0015 实现交接与复测入口

**当前职责（2026-10-16 更新）**：用户已要求本助手继续完善代码并测试；下文旧分工为历史记录，修复清单继续使用。

## 分工与基线

用户已确认：其他模型按缺陷清单补齐实现，本助手负责测试、复现、定位、debug 与复测。重大缺失能力由实现模型承担；不把分工答复记作完整 0015 草案的最终确认。

- 受审查源码：`62fda76621c17806f01b5831841fa44ed4239fb0`。
- 初查文档提交：`751784b`；详细证据见 [验收记录](acceptance.md)。
- 原需求：Epic 0007 和已确认的 0008、0010、0012、0013、0014；本项 [Spec](spec.md) 与 [任务](tasks.md) 组织复测。
- 初查的 41 项测试通过，但真实入口、数据格式和科学结果来源仍有阻断。未提交真实 EPW 任务。

本清单是实现模型的工作入口。按阶段提交可复测代码，无需等全部超导功能完成才交回第一批。

## 建议修复顺序

| 批次 | 问题及主要位置（相对 epwflow/src/） | 交回测试时必须具备的行为 |
| --- | --- | --- |
| 1：执行与预算 | B01 `foundation/cli.rs`、`snapshot.rs`；B05 `process.rs`；B06 `budget.rs`、`slurm.rs` | 公开 run/submit/resume 接入阶段，支持已约定的停止边界；stdout 落在阶段声明的日志位置；真实调度终态结算，未知状态保留预留，提交中断不重复投递 |
| 2：真实 QE 数据 | B09 `qe_data/nscf.rs`；B04 `reuse.rs`、`collect_ph.rs`；B07 QE 输入生成器 | 解析实际 QE 7.4.1 XML；正确复制目录树并核对保存指纹；三种旧来源经过明确适配及校验；统一 prefix 与实际 outdir，完整均匀 NSCF 及 PH 输入符合目标版本 |
| 3：真实 EPW 插值 | B07 `interpolation/epw_config.rs`；B02/B08 `scheduler.rs`、`adapter.rs`、`fine.rs` | 按目标 namelist 生成输入并读取原生数据；α²F/λ 来自实际 EPW 电子声子计算；缺质量参考、空清单、截断数据必须阻断；粗细网格和索引经实际输出核对 |
| 4：原生超导与总链 | B03 `superconductivity/{scheduler,iso,aniso,gap,attempt}.rs` | 调用目标 EPW 原生两种 Eliashberg 求解，保留完整虚频轴 Δ/Z 和映射；按 0013 的收敛、双初值、次数与 Tc 判据运行；both 独立记录两模式 |

先完成一个来源的真实 NSCF→Wannier→质量检查→细网格→α²F/λ，再覆盖另外两套，最后验证原生超导及从 SCF 起点的完整链路。初查 B01～B09 是已发现问题集合，不代表其他路径已正确。

## 修复时必须避免的误判

- 不能把频率公式生成的 α²F、Allen–Dynes 经验 Tc 或 BCS 能隙作为本需求的原生 EPW 求解结果。
- 不用空数据加 `valid` 标记通过清单验证；不能让模拟程序自行写出生产程序不会产生的文件来掩盖日志和适配错误。
- 实际 QE XML 的坐标在 `<k_point weight="...">x y z</k_point>`，本征值在相邻 `<eigenvalues>`；还需检查单位、权重、带数及完整网格。初查使用 SCF XML 验证 schema，不代表 NSCF 已算完。
- EPW 输入须核对目标 `readin.f90`，特别是 prefix、保存树、粗细网格参数和重读开关。此前写入的 `energy_reference` 等配置概念不能直接作为未经支持的 namelist 参数。
- coords 的局部 q=1 与全局序号分开管理；dvscf、patterns 和动力学矩阵按物理 q 点及扰动基底对应，不只批量改文件名。
- Wannier 收敛、插值质量、数据完整性和实际重读分别有证据；缺参考或未收敛明确阻断。

## 可直接执行的基线复现

在仓库根目录运行：

```bash
cargo test -p epwflow --locked --offline
cargo build -p epwflow --locked --offline
./target/debug/epwflow --help
./target/debug/epwflow submit -c /does/not/exist.toml
./target/debug/epwflow run -c /does/not/exist.toml
./target/debug/epwflow submit -c /does/not/exist.toml --until interpolation
```

基线预期：测试 41 项通过；submit/run 直接报阶段未实现，尚未读取不存在的配置；带 `--until` 返回参数错误。修复后，应加载并校验配置、正确识别停止边界，非法路径仍应失败。这些命令只复现接口问题，不提交有效计算。

实际 XML 的复现工具与结果保存在忽略的 `docs/local/0015-epwflow-smoke/parse-real-xml.rs` 和 `actual-xml-parser.json`；CLI 基线与来源哈希分别为同目录 `cli-baseline.json`、`phonon-source-inventory.json`。重新构建后再运行格式测试，不复用旧测试可执行文件充当新代码证据。

回归应覆盖公开 CLI、独立进程、真实格式的最小脱敏样本、提交边界中断、未知调度状态、缺文件/错映射/未收敛。科学输出适配测试必须说明样本的实际程序来源，合成样本单独标识。本助手负责复测并维护失败记录；生产功能补齐由实现模型负责。

## 数据与真实试算前置条件

已有来源位于 `examples/MgB2/sugon-amd/run/` 的 `qe_phonon_nosplit`、`qe_phonon_index`、`qe_phonon_coords`。来源只读；新增运行放隔离的 `examples/MgB2/sugon-amd/epw/0015/`。真实程序路径、机器环境、日志和计算输出留忽略目录。

三套来源有相同 2×2×1 网格、2 个不可约 q 点；SCF 与相关 PH 输出已初查。完整 EPW 可读性仍待验证，不能直接编造导入成功清单。保留原结构、赝势及物理参数；缺少数据时说明必要补算，兼容的 SCF/PH 不重跑。

真实提交前固定 NSCF、投影、能窗、验证点、细网格及独立参考输入，核验目标可执行版本，并核对历史预算。原授权总额不重置：每作业最多 1 节点、64 MPI、2 小时，同时最多 2 个；累计 8 节点小时、100 GiB，参考、探测与失败重试全部计入。历史剩余额度未知时不追加任务，不在登录节点跑科学计算。

数值标准完整继承 [0010](../0010-epwflow-qe-data/spec.md)、[0012](../0012-epwflow-interpolation/spec.md)、[0013](../0013-epwflow-superconductivity-tc/spec.md)。冒烟先验证能运行和真实产物，不能据此宣布数值正确；失败后不放宽标准或自动改物理参数。

## 每批交回内容与复测结论

实现模型每批提供：

1. 提交号、修复的 B 编号、未完成项，以及受影响的公开命令。
2. 从准备到目标阶段的直接命令和配置字段说明；跨工作目录行为一致。
3. 相关回归命令与实际退出结果；原生输入输出格式依据和日志位置。
4. 若有真实计算：登记的作业号、源码/程序/输入输出哈希、阶段执行次数、资源账本与独立参考位置，证据仅存忽略目录。

本助手重新构建，先复现缺陷关闭条件，再做真实小网格测试与数值对照。逐项记录通过、失败或未完成；一套成功写 1/3，原生两模式和从头链路单独统计。图像、41 项历史测试或文件存在均不能代替这些证据。当前不自动安装、推送或关闭 Epic。

## 已提供的首批失败回归

`epwflow/tests/real_contracts.rs` 已有 8 项已知失败用例与 1 项 stdout 对照，覆盖 B01/B04/B05/B06/B07/B08/B09。按 [直接验证命令](quickstart.md) 显式执行 ignored 项可复现；本轮全部 8 项失败已记录。修复相应缺陷后去掉该项 ignore 并复测，不能只看默认通过数。B02/B03、目录导入完整性与缺参考等其余路径继续按任务补测。

## 第二批补充与环境澄清

已新增 B02/B03/B08 三项回归：没有原生谱仍发布有效插值、没有原生求解仍发布 Tc/能隙，以及未收敛的 Wannier 日志被 Final State 误判为收敛。现在共 11 项已知失败，默认 ignored，显式执行全部复现，详情见 [验收记录](acceptance.md)。仍需补目录树和指纹等完整覆盖。

用户指定的 QE 7.4.1 配套三个程序已只读确认存在且可执行；旧 rc 的 7.1 声明不覆盖后续用户选择。真实路径只存本地证据。当前主要阻断是 epwflow 的入口和数据/求解实现，不能要求用户重复提供路径来替代这些修复；文件检查不等同计算节点运行验证。

## 2026-10-16 最新复用进展

输入层已调用 qf-crystal、qf-input，完成检查共用 qf-io；不得恢复已删除的第二套结构/输入生成器。后续优先提取 0006 当前 phonon_init/phonon_chain 的快照与物化能力，不使用旧的逐 q SCF 生成器。

本批 7 项失败回归转为通过，但入口/原生谱/原生超导 4 项暂挂回归仍显式失败。完整状态、未完成路径及只读真实 XML 证据见 [验收追加记录](acceptance.md)。现有三套声子数据保持原样；直接续算入口仍不可用。
