# qe_eledos：电子态密度计算

[返回 qeflow 索引](README.md)

## 功能与入口

读取 `[qe_eledos]`，安排 `pw.x`、`dos.x` 与 `projwfc.x`。

等价入口：`qeflow qe-el-edos`。

## 使用方法

```bash
qe_eledos -c input.toml -i POSCAR
# 在获得计算资源后运行；或改用 -s 提交
qe_eledos -c input.toml -i POSCAR -r
```

`-c/--config` 默认 `input.toml`；`-i/--input` 指定结构（也支持 `-p/--poscar`）。不带执行选项时生成输入与脚本；`-r` 在当前资源中运行，`-s` 提交作业，二选一。

## 输入、输出与衔接

默认目录 `qe_eledos/`；复用完整 SCF 保存目录，得到总 DOS 和分波投影原始数据；随后可运行 `qe_process_eledos`。

## 使用边界

DOS 网格与展宽需要自行收敛；程序生成默认输入不等于材料参数已收敛。 配置中的 `[qe].executable_path` 提供 QE 执行命令，赝势目录和 `[slurm].header` 分别提供 UPF 与队列环境。提交成功只表示作业入队。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[电子输入生成](../../../crates/qf-input/src/electronic.rs)；[基础 CLI 覆盖](../../../qeflow/tests/basic_cli.rs)。本页依据当前代码整理，本次未运行测试。
