# qe_nscf：电子非自洽计算

[返回 qeflow 索引](README.md)

## 功能与入口

读取 `[qe_nscf]`，从已有 SCF 数据构建 NSCF 输入并调用 `pw.x`。

等价入口：`qeflow qe-nscf`。

## 使用方法

```bash
qe_nscf -c input.toml -i POSCAR
# 在获得计算资源后运行；或改用 -s 提交
qe_nscf -c input.toml -i POSCAR -r
```

`-c/--config` 默认 `input.toml`；`-i/--input` 指定结构（也支持 `-p/--poscar`）。不带执行选项时生成输入与脚本；`-r` 在当前资源中运行，`-s` 提交作业，二选一。

## 输入、输出与衔接

默认目录 `qe_nscf/`。继承 SCF 的 XML、电荷密度及存在的 PAW 数据，再按 NSCF 网格计算；生成 `nscf.in/out`。

## 使用边界

需先完成匹配结构和 prefix 的 SCF；此入口不会替用户自动补跑 SCF。继承逻辑不等于复制所有波函数。 配置中的 `[qe].executable_path` 提供 QE 执行命令，赝势目录和 `[slurm].header` 分别提供 UPF 与队列环境。提交成功只表示作业入队。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[电子输入生成](../../../crates/qf-input/src/electronic.rs)；[基础 CLI 覆盖](../../../qeflow/tests/basic_cli.rs)。本页依据当前代码整理，本次未运行测试。
