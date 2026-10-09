# 需求清单：qeflow Rust 重写

## 功能性需求
- [ ] 产出单一静态二进制，快速启动、低内存占用。
- [ ] 通过 shell 子进程调用外部 QE/VASP，不内嵌 Fortran。
- [ ] `clap` 子命令映射现有 `qe_*` 命令，参数 `-i/-c/-r/-s` 兼容。
- [ ] `input.toml` 递归覆盖语义与原 `config_templates` 一致。
- [ ] `workflow_status.json`/`task_status.json` 可恢复状态机与原实现一致。
- [ ] Slurm 两段提交（pre/post）脚本与现有 `qe_phonon_split_*` 1:1 对齐。
- [ ] 续算正确跳过 finished 任务（三态 + q 点 task 状态）。
- [ ] 全量覆盖：输入生成、QE 文件解析、任务状态机、Slurm 编排、后处理（含 scflow 超导 Tc 与绘图）。
- [ ] scflow 数值（λ/ω_log/Tc）与原 Python 实现对齐。
- [ ] 绘图用 Rust crate 替代 matplotlib，输出形态一致。

## 非功能性需求
- [ ] 发布目标为 Linux musl 静态二进制（`cargo build --target x86_64-unknown-linux-musl` + `strip`）。
- [ ] Darwin 仅作开发构建环境。
- [ ] 不引入数据库 / 常驻 daemon。
- [ ] 不修改外部 QE/VASP 源码。

## 兼容性约束
- [ ] `executable_path` 拼接 `pw.x/ph.x/q2r.x/matdyn.x` 语义不变。
- [ ] `[qe]/[slurm]/[qe_phonon]` 配置块与递归覆盖兼容。
