# P0 证据与基线记录（dftflow 全量 Rust 重写）

> T001–T006 进度与可复核证据。状态：可行部分已完成；musl/spglib/PDF 可行性（T004/T005）需 Linux 验证，本机（macOS）无法完成。

## T001 — 黄金基线 pytest 纳入版本管理
- 修改 `.gitignore`：保留 `test_auto/**/MgB2_calc/` 与运行时生成目录的忽略，新增对 11 个 `test_*.py` 的否定放行规则（`test_auto/MgB2/{qeflow,scflow,substitutionflow,vaspflow,wannier90flow}/test_*.py`）。
- 受影响的 11 个文件：`test_auto/MgB2/qeflow/test_qeflow.py` 等（见 `find test_auto/MgB2 -name test_*.py`）。

## T002 — 冻结 Python 参考提交
- 冻结标签：`python-baseline-0004` → 指向 commit `9396b3a`（迁移前基线，含原始 Python 实现与既有 0004 Spec）。
- 失效引用核查：`test_auto/` 下无 `validate_scflow_isif4.py` 引用（grep 为空）；原 Spec/验收中关于该脚本的引用已在 `acceptance.md` 校正为「使用已确认的外部验收集或补齐可跟踪脚本」。
- 输入哈希：MgB2 样例输入（`MgB2.vasp`、`input_qe.toml` 等）保持未变；后续 P2 运行时记录 SHA-256。

## T003 — 仓库重排（根 workspace）
- 操作：`git mv qeflow-rust/crates crates`（保留历史）；`qeflow-rust/crates/*` 8 个共享/领域 crate 现在位于仓库根 `crates/`。
- 新增根 `Cargo.toml`：虚拟 workspace，`members = ["crates/*", "qeflow-rust"]`，共享依赖集中在 `[workspace.dependencies]`。
- 改写 `qeflow-rust/Cargo.toml`：移除嵌套 `[workspace]`，8 处 `crates/qf-*` 改为 `../crates/qf-*`，其余依赖走 `workspace = true`。
- 验证：`cargo build` 在 macOS（dev profile）**成功**，`qeflow` 二进制 `--help` 正常，列出 `qe-scf/nscf/eband/el-edos/phonon-*/process-phonon/scflow/status` 等子命令。
- 顺带修复的既有编译错误（P0 门禁前置）：
  - `qf-state`：`TaskState` 补 `#[derive(Hash)]`；`file_name()` 返回 `Option` 需解包。
  - `qf-config`：`walk_manual` 中 `child_path` 移动后借用 → `clone()`。
  - `qf-runner`：`parallel_read` 闭包补 `Send` 约束。
  - `qf-crystal`：非法格式 `{:.12g}`/`{:.16f}` → 近似 `g` 的自定义格式 / `{:.16}`。
  - `qf-scflow`：补定义 `pub enum Integration` 并移除错误自导入；`lib.rs` 引入 `anyhow::Context`。

## T004 / T005 — 静态 OpenBLAS、spglib、PDF（待 Linux）
- **决策已按 Spec 确认（无需再问用户）**：`spec.md` L45 / `research.md` L15 / `FR-003` / `plan.md` P2 / `acceptance.md` L9 / `tasks.md` T004·T015 一致规定 —— Eliashberg 矩阵本征值**采用静态 OpenBLAS（`ndarray-linalg` + `openblas-static`）求解**。
- 现状：`qf-scflow` 当前用**纯 Rust Jacobi 本征值求解**（`crates/qf-scflow/src/eig.rs`），该实现被 Spec 明确标记为「试验实现，不能代替已确认的交付方案」（`research.md` L15）；它是 P2 前的占位，不是交付求解器。
- `ndarray-linalg`/`openblas-static` 虽列入 `[workspace.dependencies]`，但当前无 crate 引用，未实际编译；其 musl 静态链接需在 Linux 机器验证。
- `spglib`、`svg2pdf`（PDF）尚未加入依赖；其 musl 静态链接与离线运行需在 Linux 验证。
- 结论：T004/T005 的 musl 可行性验证**留待 Linux 计算节点**；本阶段**不静默把试验版 Jacobi 当作交付求解器**，待 Linux 验证 OpenBLAS 后置 `qf-scflow` 的 Eliashberg 求解为 `ndarray-linalg` 静态 OpenBLAS（失败则停止修订方案，不擅自换库）。

## 退出门检查
- [x] 根 `cargo build` 通过（macOS dev）。
- [x] Python 参考提交已冻结（`python-baseline-0004` @ `9396b3a`）。
- [x] 黄金基线 pytest 已纳入 Git 跟踪（`git check-ignore` 对 `test_*.py` 不再命中）。
- [ ] T004/T005 的 Linux musl 静态构建与 spglib/PDF 可行性：待 Linux 验证（R-005）。

## 待办 / 风险
- P5 前 `qeflow-rust/` 目录名不变（与已存在的 Python `qeflow/` 并存），重命名推迟到 P5（T028）。
- `qf-scflow` 求解器策略**已按 Spec 确认**：交付用静态 OpenBLAS（`ndarray-linalg`）；当前纯 Rust Jacobi 为试验占位，切换受 Linux T004/T005 musl 验证门禁约束，不静默替换。
