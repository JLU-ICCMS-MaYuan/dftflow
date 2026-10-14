# qe_split_submit：分 q 子任务提交工具

[返回 qeflow 索引](README.md)

## 功能与入口

独立提交分 q 任务；--chain 进入新链路的 q 点与 post 调度阶段。

等价入口：`qeflow qe-split-submit`。

## 使用方法

```bash
qe_split_submit -c input.toml --mode index --chain
# 旧式独立提交（语义不同）
qe_split_submit -c input.toml --mode coords
```

--mode 必填，只能 index 或 coords；-c 默认 input.toml；-i/--input 可指定结构；--chain 可选。命令本身就会提交，不加 -s。

## 输入、输出与衔接

--chain 需要前置阶段已经完成初始化与输入物化，提交 q 点作业及依赖全部成功的 post；自动全链路内部会调用它。原独立模式根据 dyn0/映射生成数字目录与 run_split.sh。

## 使用边界

通常用户直接运行 qe_phonon_split_index/coords -s。不要把 --chain 当成从空目录执行 SCF 的入口。旧式不带 --chain 的模式仍可能逐 q SCF，且不自动提交最终后处理，与新链路一次 SCF 的语义不同。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[链路提交](../../../qeflow/src/phonon_chain.rs)；[旧拆分实现](../../../crates/qf-phonon/src/split.rs)。本页依据当前代码整理，本次未运行测试。
