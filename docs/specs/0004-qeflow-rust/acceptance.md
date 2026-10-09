# 验收标准：qeflow Rust 重写

## 构建与交付
- [ ] `cargo build --release --target x86_64-unknown-linux-musl` 成功，产物 `qeflow` 为静态链接二进制（`file` 显示 statically linked），并经 `strip`。
- [ ] Darwin 开发构建（`cargo build`）可编译通过，作为开发验证环境。

## 配置与状态契约
- [ ] `qf-config` 递归覆盖对 `input.toml` 的处理与原 `config_templates._merge_dict_in_place` 行为一致：`dict×dict` 递归、非 dict 覆盖、`None` 不覆盖。
- [ ] `workflow_status.json`/`task_status.json` 读写与原 Python 状态机一致，续算可正确跳过 finished 任务。

## 编排等价性
- [ ] `qe-scf`/`qe-nscf` 垂直切片输出（输入文件、运行命令、状态文件）与原 `qe_scf`/`qe_nscf` CLI 等价。
- [ ] `qe_phonon_split_*` 的 Slurm 两段提交脚本（pre/post）与现有 Python 生成内容逐字段一致。

## scflow 数值等价（最高风险）
- [ ] `scflow` 垂直切片在同一组 QE EPC 输入下，λ、ω_log、Tc（Allen-Dynes 与 Eliashberg 下限）与原 Python `scflow` 数值一致（误差在浮点容差内）。
- [ ] `plotters` 输出 PNG/CSV 与原 matplotlib 输出形态、坐标轴、共享频率轴一致。

## 端到端
- [ ] 在 Linux musl 环境用静态二进制跑通 MgB2 样例全链路（scf→nscf→eband→eledos→phonon），产物与现有工作流对照通过。
- [ ] 根 README 与 `docs/overview/` 已更新，`AGENTS.md` 已登记本需求。
