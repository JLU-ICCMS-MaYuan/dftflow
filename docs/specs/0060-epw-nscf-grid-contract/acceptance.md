# 0060 验收记录

日期：2026-10-26。用户明确“开始执行落地”，本轮按已记录范围完成工作树实现及相关验证，不操作线上 Issue、推送、安装或科学队列。

## 已实现与证据映射

| 范围 | 实现事实 | 验证 |
| --- | --- | --- |
| FR-001～003、SC-001～002 | NSCF 三整数数组、SCF 同维度零位移、Rust 官方格式展开及溢出检查 | native_stage_config 11 项、nscf_grid_contract 配置用例、qf-input 11 项 |
| FR-004～007、SC-003～004 | 实际输入／输出声明和记录数、点序／坐标／重复／权重；pw.x 两种模式 | 8 项 NSCF 契约、3 项公开进程；SCF 16／NSCF 36 通过，40／35 点拒绝 |
| FR-008～010、SC-005～006 | 原生 FFT 完整三元组、密度和平滑实际输出核查；交付绑定实际输入；PH 既有来源和响应核查保留 | 数量失败后无 NSCF／Wannier 完成回执且仅两次 pw 调用；密度与平滑不匹配均失败；qe_data 三种 PH 来源及导入、恢复回归 |
| FR-011、SC-007 | 手册、操作说明、0059 现行契约补充和 Overview 已同步 | 本文区分源码、提交、安装、受控和历史证据；相对链接核查 |

## 验证命令与结果

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p qf-input
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow --test native_stage_config --test nscf_grid_contract --test qe_data --test native_outputs --test native_interpolation --test foundation --no-fail-fast
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow --test three_stage_execution nscf_grid_contract -- --nocapture
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow --lib --no-fail-fast
```

- qf-input：11 通过。官方脚本六组为 1×1×1、2×3×6、6³、4×4×2、2×3×7、11×3×5，带权重输出逐字相同。冻结脚本 SHA256：c47e454b934e05c5107d8d7687f9fef478ec775c87f3f830285557f88b3c9e91。
- 六个集成套件：75 通过；分项 foundation 15、native_interpolation 13、native_outputs 10、native_stage_config 11、nscf_grid_contract 8、qe_data 18。默认忽略 3 项：两项 LDA 本机证据不属于本轮；SiP₂ 只读项已单独显式运行通过，不能将默认忽略算作通过。
- 公开入口：3 通过，实际执行生成的 PW 输入；输出 40／35 点、同点数但点序改变、密度／平滑 FFT 不匹配都非零退出，不登记完成、不进入 Wannier。合法模式在回执复用后不再调用科学替身。第一轮正向复用测试误将旧作业 201 保持为活动，调度器正确拒绝；修正测试调度状态后上述最终三项全通过。
- 早期 qe_data 的带位移 SCF 用例与新零位移要求冲突；当前 EPW 用例改为零位移，共享 qf-input 的位移功能测试仍通过。
- 相关生产代码已通过编译。包含 --lib 的更广命令未通过：0059 未提交的 electronic_sampling::wdata 从一个参数改为三个，但三处单元测试调用尚未更新，产生 E0061。随后并行工作树已更新三处调用，本助手未改写该无关改造；补跑库测试最终 46 通过、2 忽略。两项忽略分别需要真实粗网格回执和真实 MgB₂ 配置，未执行且不计通过。初次失败仍保留，不将补跑记录伪装成初次全过。
- 最终范围合计 135 项相关受控回归通过，另 1 项 SiP₂ 历史只读核验通过；并未执行整个仓库的全部测试。

机器日志仅保留在 /tmp/dftflow-0060-qf-input.log、/tmp/dftflow-0060-integration.log、/tmp/dftflow-0060-public-final.log、/tmp/dftflow-0060-sip2-readonly.log 和 /tmp/dftflow-0060-regression.log、/tmp/dftflow-0060-lib-final.log；不提交机器原始证据。

## 历史 SiP₂ 只读核验

显式运行 nscf_grid_contract 的 actual_sip2_fft_evidence_is_read_only_and_rejected，通过生产 fft_grids／verify_fft 读取既有 XML，拒绝不匹配。SCF 密度 [216,128,60]、NSCF [216,125,60]；平滑均 [120,72,32]。读取前后两文件哈希分别保持 b4bf6505290aa0964bdd9113c6625e1c5e19fd3d1527a8c29b3425e88152e67c 与 c18ed542be265eca455918e489130ea59c65258054d666ce68e1f833db8a7b6d。

历史 XML 的 nosym／对称操作与 QE 源码支持对称约束变化导致第二维可取 125／128 的解释；不能据此宣称历史堆损坏仅有此根因。此核验只读，不是新 SCF、PH、NSCF 或超导验收。

## 提交与未完成边界

- f83a641 已独立提交共享 Rust 生成器及测试，3 文件、66 插入／8 删除；不含其他未提交改造。
- 其余 0060 配置、接续检查和回归在当前工作树通过上述验证。关键依赖 native_config.rs、native_stage_config.rs 和 0059 文档在任务开始前已存在但未被 Git 跟踪；把它们整体暂存会纳入 0059，不能安全构造仅含 0060 的完整独立源码提交。保留任务前备份及任务差异到 docs/local/0060-epw-nscf-grid-contract/remaining-task.patch（已反向预检通过）、baseline/ 和 manifest.json，等待依赖基线可用，不追认 0059 为本任务成果。
- 本轮仅按简写要求同步当前忽略的 MgB₂ 可编辑示例配置，不修改任何历史科学结果；该配置不强制纳入 Git。
- 外部 SiP₂ save 导入、迁移 examples/SiP2、安装和用户实际续算步骤仍属于后续范围；SCF 可以重新计算，声子数据保持原样。
- 主入口命令没有改变，根 README 无须调整。线上状态／类型／远程 Spec 双向链接由用户管理，本轮不宣称 Issue 已关闭。

## 工程原则

复用单份共享网格生成器及单份 NSCF 文件核验器；新执行、阶段复用和清单交付复用同一规则，避免三套检查漂移。FFT 只增加明确三元组与实际 XML 对比，不增加自动猜测、重试或外部 Perl 生产依赖。


## 2026-10-27 后续整合

0010 外部声子导入补充按用户明确授权整合待提交的 0060 适配与已提交的 0061 平层格式，相关源代码及回归随本次任务提交，不再受此前 0059 基线未落档的限制。当前公开字段为 `stages."2.qe-nscf".K_POINTS` 和 `.calculation`；错误提示同步推荐平层 `calculation='bands'`，仍使用 pw.x。正式 release 发布和真实 SiP₂ 导入的证据见 [0010 验收](../0010-epwflow-qe-data/acceptance.md)，前文“未安装、尚无原始导入”的限制仅描述 0060 初次实施时的状态。
