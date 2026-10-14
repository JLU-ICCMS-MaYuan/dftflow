# qe_eband：电子能带计算

[返回 qeflow 索引](README.md)

## 功能与入口

读取 `[qe_eband]`，生成沿路径的电子计算、`bands.x` 和 `projwfc.x` 步骤。

等价入口：`qeflow qe-eband`。

## 使用方法

```bash
qe_eband -c input.toml -i POSCAR
# 在获得计算资源后运行；或改用 -s 提交
qe_eband -c input.toml -i POSCAR -r
```

`-c/--config` 默认 `input.toml`；`-i/--input` 指定结构（也支持 `-p/--poscar`）。不带执行选项时生成输入与脚本；`-r` 在当前资源中运行，`-s` 提交作业，二选一。

## 输入、输出与衔接

默认目录 `qe_eband/`；输入结构、SCF 保存数据和 `KPATH.in`，输出 QE 能带及投影原始文件，可再交给 `qe_process_eband`。路径缺失时尝试调用 VASPKIT。

## 使用边界

必须已有可用 SCF 数据；VASPKIT 路径生成依赖外部程序，宜预先检查 KPATH.in。 配置中的 `[qe].executable_path` 提供 QE 执行命令，赝势目录和 `[slurm].header` 分别提供 UPF 与队列环境。提交成功只表示作业入队。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[电子输入生成](../../../crates/qf-input/src/electronic.rs)；[基础 CLI 覆盖](../../../qeflow/tests/basic_cli.rs)。本页依据当前代码整理，本次未运行测试。
