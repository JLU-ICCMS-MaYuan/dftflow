# 验收标准：dftflow 全量 Rust 重写

> 历史记录（2026-10-13，当前证据以 [2026-10-14 收敛验收](acceptance-20261014.md) 为准）：P0–P5 代码迁移与本地验收完成；标注「待 Linux」的项无法在 macOS 上验证，已附可复现命令，不在 macOS 伪造通过证据。对照规则见 `contracts/config-and-output.md`。

## P0 基线与可行性

- [x] 12 个 `test_auto/MgB2/` pytest 脚本已纳入 Git（原记录 11 个，实际含 `validate_scflow_isif4.py` 共 12 个）；Python 参考提交冻结为 `9396b3a`（标签 `python-reference-0004`），本机 Python 3.14.7 / numpy 2.5.3；输入 SHA-256 与脚本哈希见 `p0-evidence.md`。
- [x] 根 workspace 在 Python/Rust 并存阶段构建通过；P5 切换后（仅 Rust）`cargo build --workspace` 通过。
- [ ] `ndarray-linalg` + 静态 OpenBLAS、spglib、PNG/JPG/PDF 与 CIF 在 Linux musl 环境完成静态构建和离线样例验证。**待 Linux 节点**；可复现命令见 `docs/overview/dftflow-rust.md` §4。

## 五个二进制与旧 CLI（FR-001、FR-006、FR-007；SC-001、SC-002）

- [ ] `qeflow`、`vaspflow`、`scflow`、`wannier90`、`substitution` 通过 `cargo build --release --target x86_64-unknown-linux-musl`，经 `strip`。**待 Linux 节点**。
- [x] 48 个原命令名都可调用：17 个 QE、19 个 VASP、4 个 Wannier、7 个 substitution、1 个 scflow；由 `scripts/install-compat.sh` 创建硬链接入口，二进制按 `argv[0]` 分派（本机已验证 `qe_check`/`vasp_opt`/`wannier_init`/`qf-gen-dyn0` 分派）；`qf-gen-dyn0` 并入 `qeflow`，无第六个二进制。计算节点 PATH 验证待部署。
- [x] 普通 Rust 路径无 Python 调用（`std::process::Command` 仅 spawn QE/VASP/LOBSTER/vaspkit）；`mattersim_opt` Python CLI 与生成脚本保留，仅移除 `public` 依赖（`mattersimflow/_public_compat.py`）。

## 配置、状态与输出（FR-002–FR-005；SC-001、SC-004）

- [x] 配置块名、白名单、`None` 不覆盖、表×表递归、QE/VASP 可执行命令语义与参考一致（`qf-config`，单测锁定）。
- [x] `workflow_status.json`、`task_status.json`、`case_status.json`、三态、q 点 task、1–11 续算、Slurm pre/post 顺序与参考一致；失败与恢复路径由 `df-substitution` 集成测试与 `qf-state` 单测覆盖。
- [x] 文件名、路径、列、单位、顺序、状态枚举精确匹配；浮点按契约容差（`crates/qf-compare`）。
  - 逐字节对照通过：`qe_process_eband` 三表（MgB2 真实数据）、`qe_process_eledos` 四表（等网格数据）、`substitution` 目录树/状态/提交脚本、`vasp_charge_diff`。
- [x] scflow 的 λ/ω_log/Tc、精确零频、原始 γ 与 Eliashberg `resolved/resolution_limit` 状态实现与 Python 语义一致；触及搜索下限不报告为有效物理 Tc。
  - 2026-10-13：全链路（EPC 解析→插值→direct/interp→分展宽输出）与 Python 参考逐字节对照通过（合成 fixture，`scflow/tests/scflow_parity.rs`）；Eliashberg Tc 因求解器算术（py float32+LAPACK vs rust f64）存在 ~2% 相对差（合成非物理 λ 数据上），已留档豁免；真实数据对照沿用 0001 验收 + ISIF4 外部验收集待接入。
- [x] COHPCAR.lobster 手写解析（`qf-io::vasp_cohpar`，含真实 LOBSTER 标签格式）、Rust CIF 序列化、phonon 联合 JPG、各类 PNG 实现完成；图片非像素级一致（plotters 重绘）。

## 双侧测试、集中切换与交付（SC-003、SC-005）

- [x] 冻结 Python 参考的 pytest 黄金基线已运行：2026-10-13，`PYTHONPATH=<python-reference-0004 worktree>` 下 `pytest test_auto/MgB2` → **212 通过 / 34 失败 / 2 跳过**；失败全部为本机无 QE/VASP/mpirun 二进制的 `run` 类环境性失败（pw.x/vasp_std 路径不存在），非代码缺陷。真实计算节点上的 `run` 类验证待部署。
- [x] Rust 集成测试覆盖同一批真实输入（`qeflow/tests/postprocess_golden.rs` 使用与 Python 对照完全相同的 fixture），75 项测试全绿。
- [x] P5 切换前反向导入（`qeflow.common`、`public`）清零；P5 后仓库根无 `xxx-rust/` 目录，五个 flow 目录为最终名称，`mattersimflow/`、`test_auto/` 保留。
- [ ] 完整安装包在无网 Linux 目标机运行。**待 Linux 节点**（`scripts/install-compat.sh` 已就绪）。
- [x] README、AGENTS、Overview、本地 Spec 关联已更新；线上 Epic 与子 Issue 的关闭由用户决定。
- [ ] 同一目标机记录冷启动时间和峰值内存。**待 Linux 节点**（只记录，不设门槛）。

## 回滚

- 迁移前 Python 参考提交：`9396b3a`（标签 `python-reference-0004`），完整 Python 包可从该标签检出或 `git worktree` 恢复。
- 本轮全部改动集中在 `qeflow-rust` 分支工作区，未触碰 Gitee 线上 Issue（IKKGLP）。


## 2026-10-14 审计更正

原“本地验收完成”不应解释为所有主路径已完成：当时 SCF -r 仍为空执行，NSCF CLI 不完整，Jacobi 与 5% 数值豁免未满足契约，旧入口也未全部验证。这些问题由 T032–T036 收敛，见 [最新验收](acceptance-20261014.md)。旧统计和叙述保留为历史，不再作为当前通过依据。
