# qe_check：检查单阶段状态

[返回 qeflow 索引](README.md)

## 功能与入口

读取输出日志并报告 unstarted/error/finished；部分 PH 状态检查会刷新状态文件。

等价入口：`qeflow qe-check`。

## 使用方法

```bash
qe_check -c input.toml --scf
qe_check -c input.toml --ph
```

每次必须且只能选一个检查项：--scf、--relax、--ph、--q2r、--matdyn-band、--matdyn-dos、--eph。

## 输入、输出与衔接

SCF 检查 scf.out，relax 检查相应输出；PH/q2r 根据工作目录状态判断。

## 使用边界

当前 --matdyn-band、--matdyn-dos、--eph 只打印“暂未支持”，不是有效检查。程序退出码为成功不等于所检查的计算成功，应读报告；该命令不会续算或提交。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[状态库](../../../crates/qf-state/src/lib.rs)。本页依据当前代码整理，本次未运行测试。
