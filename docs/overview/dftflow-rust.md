# dftflow 全量 Rust 迁移（Issue 0004）实现总览

> 状态：**P0–P5 已在本机（macOS）完成代码迁移与验收**；Linux musl 静态交付验证（T004/T005）留待 Linux 节点，见「待 Linux 验证项」。
> Spec：[0004-dftflow-rust](../specs/0004-dftflow-rust/spec.md)；远端 Issue：[Gitee IKKGLP（编号 0004）](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKKGLP)。

## 1. 交付物（五个家族二进制 + 48 个旧命令入口）

| 二进制 | 位置 | 覆盖旧命令 |
|---|---|---|
| `qeflow` | `qeflow/src/bin/qeflow.rs` | 17 个 `qe_*` + `qf-gen-dyn0` 别名（T013 已并入，无第六个二进制） |
| `vaspflow` | `vaspflow/src/bin/vaspflow.rs` | 19 个 `vasp_*`（含 `vasp_idos_computed` / `vasp_plot_cohp` / `vasp_plot_kcohp` / `vasp_charge_diff`） |
| `scflow` | `scflow/src/bin/scflow.rs` | `scflow` |
| `wannier90` | `wannier90flow/src/bin/wannier90.rs` | 4 个 `wannier_*` |
| `substitution` | `substitutionflow/src/bin/substitution.rs` | 7 个 `substitution*` |

- 旧命令入口：`scripts/install-compat.sh <bin_dir> [target_dir]` 为每个旧名创建硬链接（回退软链），二进制按 `argv[0]` 分派（`LEGACY_ENTRY_NAMES` 表）。
- `mattersim_opt` 保留 Python CLI（登记的 Python 例外）；其生成脚本仍由目标机 Python 运行。
- Python `qeflow/scflow/vaspflow/wannier90flow/substitutionflow/public` 已全部删除；`qeflow-python-backup/` 已删除；Python 参考冻结于提交 `9396b3a`（标签 `python-reference-0004`）。

## 2. workspace 结构

```
Cargo.toml          # 根虚拟 workspace：vaspflow, wannier90flow, substitutionflow, scflow, crates/*, qeflow
crates/
├── qf-config/      # TOML 递归深合并（表×表递归、None 不覆盖）+ 默认模板
├── qf-crystal/     # POSCAR 解析、化学式、kmesh、晶格参数、CIF 写出
├── qf-input/       # QE 输入卡（pw/ph/q2r/matdyn/KPATH vaspkit 生成）
├── qf-io/          # QE/VASP/LOBSTER 输出解析 + qe_proj（projwfc/pdos 后处理）
├── qf-state/       # 三态状态机、q 点 task、1–11 续算
├── qf-runner/      # spawn 外部程序、Slurm 两段提交脚本
├── qf-plot/        # plotters 绘图封装
├── qf-phonon/      # split/merge/dyn0(含 qf-gen-dyn0 bin)/归档/表格/绘图
├── qf-scflow/      # scflow 完整链路：dyn*.elph 解析/q 插值/direct+interp/λ·wlog·w2/Tc/分展宽 CSV/result.json/spglib CIF/三类 PDF
├── qf-compare/     # 产物对照器（契约容差）
└── df-vasp/ df-substitution/   # vasp / substitution 领域逻辑
qeflow/  vaspflow/  scflow/  wannier90flow/  substitutionflow/   # 薄 bin
mattersimflow/      # [保留 Python] 仅去除 public 依赖（_public_compat.py 等价内联）
test_auto/          # [保留] pytest 黄金基线（需在冻结参考工作树运行）
scripts/install-compat.sh
```

## 3. 本轮验收证据（2026-10-13）

- `cargo build --workspace` 通过；`cargo test --workspace` 75 项全绿。
- **scflow 完整链路逐字节对照**：合成 EPC fixture（2 q × 6 模式 × 3 展宽）端到端对照 Python 参考：分展宽 CSV、summary CSV、MgB2.vasp 全部逐字节一致；result.json 除 Eliashberg 求解器差分（py float32+LAPACK vs rust f64，~2%，留档豁免）外逐字段一致；CIF（空间群 35/Cmm2）、三类 PDF（plotters SVG + svg2pdf）文件形态等价。已固化为 `scflow/tests/scflow_parity.rs` 集成测试。
- **pytest 黄金基线**：冻结参考环境运行 212 通过 / 34 环境性失败（本机无 QE/VASP 二进制）/ 2 跳过。
- **真实数据逐字节对照（Python 参考 `python-reference-0004` vs Rust）**：
  - `qe_process_eband`（MgB2 真实 projwfc/bands 数据）：`Mg1B2_eleband_proj.dat` / `_proj_sum.dat` / `_band.dat` 三表逐字节一致（`qeflow/tests/postprocess_golden.rs`）。
  - `qe_process_eledos`（合成等网格数据）：四张 `_dos_proj*.dat`/`_idos_proj*.dat` 逐字节一致；能量网格不一致场景与 Python 同样报错。
  - `substitution structure` + `continue`（2 case × 2 压强）：目录树、`structure.vasp`、`case_status.json`、`workflow_manifest.json`、`workflow_status.json`、`submit_group_01.sh` 与 Python 输出仅差根路径前缀，键序与步骤顺序一致。
  - `vasp_charge_diff`（合成 CHGCAR）：差分逐字段一致。
- 替换的 Python 反向导入清零：`public`/`qeflow`/`scflow` 等导入方全部随 Python 删除；`mattersimflow` 已改用 `_public_compat.py`。

## 4. 待 Linux 验证项（本机 macOS 无法验证，不伪造通过证据）

以下各项的**可复现命令**（Linux x86_64 节点）：

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
strip target/x86_64-unknown-linux-musl/release/{qeflow,vaspflow,scflow,wannier90,substitution}
file target/x86_64-unknown-linux-musl/release/qeflow   # 确认 statically linked
./scripts/install-compat.sh target/x86_64-unknown-linux-musl/release ~/.local/bin
```

- [ ] T004：`ndarray-linalg` + 静态 OpenBLAS 的 musl 静态构建与离线运行。**注意**：`qf-scflow` 当前 Eliashberg 本征值用纯 Rust Jacobi（`crates/qf-scflow/src/eig.rs`，Spec 标记为试验实现）；若 musl 下静态 OpenBLAS 可用，按 Spec 切换求解路径（失败则停止并修订方案，不静默换库）。**另**：`spglib-sys` 需要系统 spglib C 库 + bindgen/clang（macOS 经 `.cargo/config.toml` 提供 `-I/opt/homebrew/include`；Linux 需 libspglib-dev），其 musl 静态链接是本项核心验证内容。
- [ ] T005：`spglib` 对称性、PNG/JPG（plotters）、PDF（SVG 后端 + 纯 Rust `svg2pdf`）在 musl 下的静态链接。功能层面 macOS 已全部验证（CIF 空间群 35/Cmm2 正确、PDF 有效输出）。
- [ ] T030：五个二进制 strip + 离线运行 + PATH 验证；性能（冷启动/峰值内存）记录。
- [ ] `pytest` 黄金基线在冻结参考工作树（`git worktree add ../dftflow-py python-reference-0004`）运行通过（本仓库工作树已删 Python，无法直接跑）。

## 5. 功能规格与契约

- Spec：[spec.md](../specs/0004-dftflow-rust/spec.md)（FR-001～FR-007、SC-001～SC-005）
- 契约：[contracts/config-and-output.md](../specs/0004-dftflow-rust/contracts/config-and-output.md)
  - 配置块名 `[qe]`/`[qe_phonon.*]`/`[slurm]`/`[scflow]`/`[vasp]`/`[wannier90]`/`[wannier_init]`/`[substitutionflow]`/`[mattersim_opt]`
  - 浮点容差 `|new-ref| <= A + 1e-6×|ref|`，禁止失败后放宽（对照器：`crates/qf-compare`）
- 验收清单：[acceptance.md](../specs/0004-dftflow-rust/acceptance.md)；任务勾选：[tasks.md](../specs/0004-dftflow-rust/tasks.md)
