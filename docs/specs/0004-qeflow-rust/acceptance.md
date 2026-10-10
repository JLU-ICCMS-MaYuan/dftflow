# 验收标准：dftflow 全量 Rust 重写

> 以下均为未来实施验收项，当前未勾选。对照规则见 `contracts/config-and-output.md`；任何一项未通过不得宣称全量迁移完成。

## P0 基线与可行性

- [ ] 11 个现有 `test_auto/MgB2/` pytest 脚本已纳入 Git；Python 参考提交、依赖环境、输入 SHA-256 与真实 ISIF4 数据来源已冻结。不存在的 `validate_scflow_isif4.py` 不再用作验收命令。
- [ ] 根 workspace 在 Python `qeflow/` 与 Rust `qeflow-rust/` 并存时构建通过，无路径碰撞。
- [ ] `ndarray-linalg` + 静态 OpenBLAS 的真实 Eliashberg 本征值路径、spglib、PNG/JPG/PDF 与 CIF 在 Linux musl 环境完成静态构建和离线样例验证。

## 五个二进制与旧 CLI（FR-001、FR-006、FR-007；SC-001、SC-002）

- [ ] `qeflow`、`vaspflow`、`scflow`、`wannier90`、`substitution` 通过 `cargo build --release --target x86_64-unknown-linux-musl`，经 `strip`；用 `file`、`readelf` 或等效工具确认没有意外动态库依赖。
- [ ] 48 个原命令名都可从交付包直接调用：17 个 QE、19 个 VASP（含 `vasp_elf`）、4 个 Wannier、7 个 substitution、1 个 scflow；参数、默认值、帮助、退出码及 Slurm 脚本调用与参考版本对照。`qf-gen-dyn0` 不作为第六个独立交付二进制。
- [ ] 普通 Rust 路径在离线机器不调用 Python；启用可选 MatterSim 步骤时 `mattersim_opt` Python CLI、生成脚本和现有运行环境仍可用。

## 配置、状态与输出（FR-002–FR-005；SC-001、SC-004）

- [ ] 配置允许键、模板、`None` 不覆盖、表×表递归、QE/VASP 可执行命令语义与参考一致。
- [ ] `workflow_status.json`、`task_status.json`、substitution 状态、q 点续算及 Slurm pre/post 顺序与参考一致；失败与恢复路径有真实集成验证。
- [ ] 文件名、路径、列、单位、顺序、非数值状态及输入卡语义精确匹配；CSV/DAT/JSON 浮点按契约的固定公式验收，并记录误差。不因失败临时扩大容差。
- [ ] scflow 的 λ/ω_log/Tc、精确零频、原始 γ 与 Eliashberg 下限状态在真实 QE EPC/ISIF4 输入上对照；搜索下限不被标为有效物理 Tc。
- [ ] COHPCAR.lobster 手写解析、CIF 结构/空间群、phonon 联合 JPG、其他 PNG、scflow PDF 的内容和文件形态对照通过；图片非像素级一致即可。

## 双侧测试、集中切换与交付（SC-003、SC-005）

- [ ] 在冻结 Python 参考工作树运行已跟踪的 pytest 黄金基线，**并且**在 Rust 工作树运行覆盖同一真实输入及故障边界的集成测试；两侧证据分别留档。
- [ ] P2–P4 期间旧 Python 包仍可用；P5 删除前，`qeflow.common`、`public` 等反向导入及脚本调用已经清零。
- [ ] 最终根目录没有 `xxx-rust/`；五个 flow 目录为最终名称；`mattersimflow/`、`test_auto/` 与 `mattersim_opt` 的最小 Python 打包内容保留。
- [ ] 完整安装包在无网 Linux 目标机运行；计算节点可通过 PATH 找到旧命令别名和外部 QE/VASP/LOBSTER/vaspkit；回滚包与参考提交可用。
- [ ] README、AGENTS、已实现 Overview、真实子 Issue 编号与本地 Spec 的关联已更新；线上 Epic 与子 Issue 的关闭仍由用户决定。
- [ ] 同一目标机记录冷启动时间和峰值内存；按用户选择，此项只要求记录，不设性能通过门槛。
