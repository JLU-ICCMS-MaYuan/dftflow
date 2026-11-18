# 技术任务：统一绘图格式配置

协作状态以 [Issue #74](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/74) 为准。下列只记录技术拆解。

## 阶段 1：配置

- [x] T001 [US1] 验证 `crates/qf-config/src/lib.rs` 已有配置与九项测试，增加真实加载校验和默认兼容覆盖。

## 阶段 2：统一输出

- [x] T002 [US1] 在 `crates/qf-plot/tests/formats.rs` 验证四格式、组合、DPI、尺寸、字体、失败隔离与无临时文件。
- [x] T003 [US1] 在 `crates/qf-plot/src/formats.rs` 实现 `save_plot` 和公共 PDF 转换，更新 crate 依赖及锁文件。

## 阶段 3：QE 声子

- [x] T004 [US2] 在 `qeflow/tests/phonon_plot.rs` 覆盖默认 SVG、只 PNG、多格式、非法配置和原始 CSV 不变。
- [x] T005 [US1] 接入 `crates/qf-phonon/src/plot.rs` 三图、`qeflow/src/lib.rs` 全部调用方及续算清单；更新 EPW 历史重绘调用和测试期望。
- [x] T006 [US1] 使用已有真实 MgB₂ 的频率、DOS、标签及结构副本，验证三类图和 DPI，不提交科学任务。

## 阶段 4：scflow

- [x] T007 [US1] 在 `scflow/tests/scflow_parity.rs` 验证完整链三类图的默认 SVG+PDF、配置额外格式和数值黄金对照。
- [x] T008 [US1] 在 `crates/qf-scflow/src/config.rs`、`lib.rs`、`plots.rs` 接入格式，保留原 PDF 兼容接口和文件名。

## 阶段 5：文档

- [x] T009 [US3] 更新 `README.md`、`CLAUDE.md`、现有 QE 测试模板和 `docs/overview/`，补齐本目录设计、快速验证、需求核对和验收；明确范围与 scflow PDF 例外。

## 阶段 6：验收与发布

- [x] T010 完成 99 项相关测试及扩展回归，对照 SC-001–016 记录证据；全量中止、配额不足及已复现的基线失败不计通过，见 `acceptance.md`。
- [x] T011 从仅包含本任务变更的源码构建，覆盖固定命令并核对制品；只提交本任务文件，不推送。

远端双向 Spec 链接需用户后续授权推送才能验证，不属于已完成的本地技术验证；Issue 保持现有状态。
