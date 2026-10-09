# 实施计划：qeflow Rust 重写

## 阶段 0：调研与大纲（已完成）
- 六维度调研 AiiDA / FireWorks / atomate2·jobflow / pyiron，产出借鉴结论（见 `research.md`）。
- 用 `big-project-planer` 产出总体架构大纲与阶段路线（见 `spec.md`）。

## 阶段 1：cargo workspace 骨架（已完成）
- 在 `qeflow-rust` 分支建立 cargo workspace，落地 `qeflow-rust/` 根 `Cargo.toml` 与 `crates/` 各 crate 骨架。
- 引入共享依赖：`ndarray`、`openblas-static`、`plotters`、`clap`、`serde`、`toml`、`serde_json`。

## 阶段 2：配置与 IO（已完成）
- `qf-config`：TOML 加载 + 递归深合并覆盖，复刻 `config_templates._merge_dict_in_place`。
- `qf-io`：集中读取 `scf.out/nscf.out/eband.out/ph.out/dyn*/dos/elph`，对齐 `io_reader`。

## 阶段 3：状态机与调度（已完成）
- `qf-state`：`workflow_status`/`task_status` 状态机、`split_resume_step` 三态判定、续算跳过 finished。
- `qf-runner`：`Command` spawn、Slurm 脚本生成、两段提交、并行读取（`ThreadPool`）。

## 阶段 4：垂直切片（已完成）
- 跑通 `qe_scf → qe_nscf` 与 `scflow` 单点垂直切片，并打包 musl 静态二进制。
- 验证最难路径：scflow Eliashberg 求解 + 编排链与原 Python 数值一致。

## 阶段 5：全量迁移（待做）
- `qf-crystal`：POSCAR/化学式/kmesh/ibrav 三模式晶格写出。
- `qf-input`：pw/ph/q2r/matdyn 输入 deck 写出，1:1 对齐 `input_deck`。
- `qf-phonon`：phonon split/merge 工作流编排，1:1 对齐 `split_workflow`/`split_merge`。
- `qf-scflow`：a2f、Allen-Dynes、Eliashberg、Tc、plotters（替代 numpy/scipy/matplotlib）。
- `qf-cli`：clap 子命令定义与参数适配，薄层映射现有 `qe_*` 命令。

## 阶段 6：Issue 与 Spec（进行中）
- 用 `big-project-issue-manager` 建立本需求 Issue 与 Spec 文档（本目录）。
- 用 `big-project-overview-maintainer` 回写 `docs/overview/`。

## 阶段 7：验收与收尾（待做）
- 在 Linux musl 环境验证静态二进制全量工作流与存档对照。
- 更新根 README 使用说明，关闭 Issue。
