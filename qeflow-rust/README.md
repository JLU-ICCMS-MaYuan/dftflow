# qeflow (Rust rewrite)

`qeflow` 的 Rust 全量重写：单一静态二进制，仅通过 shell 子进程调用外部 QE/VASP，
避免在弱机器上反复导入 Python 解释器与动态库。设计借鉴 FireWorks 的状态机 + 队列适配器
+ Workflow DAG，吸收 jobflow 的类型化数据流，用文件系统 JSON 状态替代 MongoDB/PostgreSQL。

## 当前进度（里程碑 M0–M1）

- **M0**：cargo workspace 骨架 + 8 个 crate；`qf-config`/`qf-state`/`qf-runner` 核心逻辑已落地。
- **M1**：`qf-config`（input.toml 递归覆盖加载）、`qf-crystal`（POSCAR/ibrav/晶格）、
  `qf-input`（pw/ph/q2r/matdyn 输入写出，含赝势自动匹配）已可用。
- 待做：**io_reader 读输出（M2）**、**phonon 状态机 + Slurm 两段提交（M3）**、
  **scflow 超导数值 + plotters 绘图（M4）**。

## 构建

```bash
cd qeflow-rust
cargo build --release
# 发布 Linux 静态二进制（可选）：
# cargo build --release --target x86_64-unknown-linux-musl && strip target/.../qeflow
```

## 快速验证（M1 垂直切片）

```bash
./target/release/qeflow qe_scf -c examples/input.toml -p examples/POSCAR
# 在当前目录生成 scf.in（pw.x 输入），并打印将执行的命令
```

`input.toml` 复用 Python 版契约：`[qe]`（`executable_path`、`lattice_mode`）、
`[qe_phonon.scf|ph|q2r|phband|phdos]` 各子段、`[k_points]`。递归覆盖语义与
`public/config_templates.py` 一致（嵌套表递归合并，None 不覆盖）。

## 模块

| crate | 职责 |
| --- | --- |
| `qf-config` | TOML 加载 + 递归深合并 + pp_dir 解析 |
| `qf-crystal` | POSCAR 解析、化学式、kmesh、ibrav 自动判断、三模式晶格写出 |
| `qf-input` | pw/ph/q2r/matdyn 输入 deck 写出、赝势匹配、文件名派生 |
| `qf-io` | QE 输出集中读取（M2） |
| `qf-state` | 状态机 + 续算（split_resume_step） |
| `qf-runner` | spawn 外部二进制 + Slurm 两段提交 + 并行读取 |
| `qf-phonon` | phonon split/merge 工作流编排（M3） |
| `qf-scflow` | a2f / Allen-Dynes / Eliashberg / Tc + 绘图（M4） |
