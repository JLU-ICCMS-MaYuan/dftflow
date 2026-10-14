# status：刷新工作流状态

[返回 qeflow 索引](README.md)

## 功能与入口

根据磁盘上的实际计算文件刷新状态，并打印续算阶段。

等价入口：`qeflow status`。

## 使用方法

```bash
qeflow status -c input.toml -w qe_phonon --prefix MgB2
```

-c 默认 input.toml；-w/--work-dir 默认当前目录；--prefix 默认 struct，实际使用应显式指定。

## 输入、输出与衔接

读取工作目录日志/结果，写 workflow_status.json，显示各任务状态与旧流程的续算位置。

## 使用边界

这是会更新状态文件的命令；它不提交作业，也不等同于 Slurm squeue。新 split 链的作业 ID 与最终完成还需看 chain_jobs.json、chain_complete.json。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[状态库](../../../crates/qf-state/src/lib.rs)。本页依据当前代码整理，本次未运行测试。
