# 需求质量清单：Issue 0004 跨阶段约束

> 本清单检查需求是否可实施、可追踪；不把未实现项目勾成已完成。用户原文与本轮澄清分别见 `../spec.md`。

## 目标与范围

- [ ] 48 个待迁移 CLI 已按 `pyproject.toml` 分组登记：QE 17、VASP 19（含 `vasp_elf`）、Wannier 4、substitution 7、scflow 1；`mattersim_opt` 单列为 Python 例外。
- [ ] 五个家族二进制、48 个旧命令兼容入口与现有 `qf-gen-dyn0` 整合关系已明确；命令参数、默认值、退出码和脚本调用可核验。
- [ ] `mattersimflow` 仅为移除 `public` 依赖作等价调整；启用其可选链路时允许 Python，普通 Rust flow 不依赖 Python。
- [ ] 外部 QE/VASP/LOBSTER/vaspkit 的责任边界、`test_auto/` 长期保留及不得自动切换主分支已明确。

## 配置、状态与产物

- [ ] 配置真实块名、递归覆盖、允许键、QE/VASP 命令语义和失败行为已记录。
- [ ] `workflow_status.json`、`task_status.json`、substitution 状态与 Slurm pre/post、1–11 续算协议可对照。
- [ ] 文件结构精确项和浮点容差公式已事前锁定；零频、原始 γ、Eliashberg 下限状态不被浮点容差掩盖。
- [ ] PNG、phonon 联合 JPG、scflow PDF、CIF、COHPCAR 与 CSV/DAT/JSON 均在输出清单内；非像素级绘图等价标准明确。

## 阶段与验收

- [ ] P0 不把 `qeflow-rust/` 改成已占用的 `qeflow/`；P2/P3 不提前删除仍有反向依赖的 Python 包；P5 才集中切换。
- [ ] P0 静态 OpenBLAS、spglib、PDF/CIF 和 Linux musl 离线样例构成阻断门；失败不静默替换已确认求解器。
- [ ] 11 个被忽略的 pytest 文件须纳入 Git；不存在的 `validate_scflow_isif4.py` 不得作为通过证据；参考输入与环境须冻结。
- [ ] Python 黄金基线与 Rust 集成测试均通过，不以“或”替代；真实性能只记录、不设通过门槛。

## Issue 与文档治理

- [ ] 0004 为 Epic 的定位与线上实际状态分开表述；P0–P5 待创建、待编号，双向链接未建时不得标为完成。
- [ ] P1–P4 获得真实子 Issue 编号后才建立同编号 Feature Spec；现有 0004 文档仅承载过渡期跨阶段资料。
- [ ] 原始需求记录完整且未删改；Issue 草稿的概述没有伪装成逐字引文；已实现事实才进入 Overview。
