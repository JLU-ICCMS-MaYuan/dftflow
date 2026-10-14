# qe_scf：电子自洽计算

[返回 qeflow 索引](README.md)

## 功能与入口

读取结构与 `[qe_scf]`，生成并运行或提交 `pw.x` 的 SCF 输入。

等价入口：`qeflow qe-scf`。

## 使用方法

```bash
qe_scf -c input.toml -i POSCAR
# 在获得计算资源后运行；或改用 -s 提交
qe_scf -c input.toml -i POSCAR -r
```

`-c/--config` 默认 `input.toml`；`-i/--input` 指定结构（也支持 `-p/--poscar`）。不带执行选项时生成输入与脚本；`-r` 在当前资源中运行，`-s` 提交作业，二选一。

## 输入、输出与衔接

默认生成 `qe_scf/` 下的 `scf.in`、运行脚本；执行后得到 `scf.out` 和配置 outdir 中的 `prefix.save` 等数据，供 NSCF、能带、DOS 使用。

## 使用边界

只安排这一阶段；后续电子任务需另外调用。 配置中的 `[qe].executable_path` 提供 QE 执行命令，赝势目录和 `[slurm].header` 分别提供 UPF 与队列环境。提交成功只表示作业入队。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[电子输入生成](../../../crates/qf-input/src/electronic.rs)；[基础 CLI 覆盖](../../../qeflow/tests/basic_cli.rs)。本页依据当前代码整理，本次未运行测试。
