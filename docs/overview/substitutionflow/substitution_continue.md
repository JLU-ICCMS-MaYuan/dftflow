# substitution_continue：更新状态与续算

[返回家族索引](README.md)

## 功能与入口

扫描已有案例的日志/输出，更新状态并按剩余阶段准备或继续任务。

等价入口：`substitution continue`。

## 使用方法

```bash
substitution_continue -o cases -c input.toml --refresh-status
# 只重写状态
substitution_continue -o cases -c input.toml --rewrite-states
```

-o 与 -c 都必填；--refresh-status 兼容 -rfs；--rewrite-states 兼容 -rws；两状态选项互斥；-r/-s 分别运行/提交且互斥。

## 输入、输出与衔接

读取 manifest、case_status.json 与日志；--rewrite-states 且无 -r/-s 时只重建状态，其他组合进入续算流程。

## 使用边界

状态判断包含 OUTCAR 标志等启发式，完成标记不等于参数收敛。当前生成的部分子命令参数与被调用 CLI 不匹配：vasp_scf/vasp_eledos 使用 -i，而当前入口要求 -p；DOS 后处理使用当前不支持的 --elements-total；nosplit 路径还传入其不支持的 -i。完整自动链不能据此宣称可直接跑通，需修复接口并验收。本次仅记录代码事实，不修改实现。

## 实现依据与关联

[CLI 定义](../../../substitutionflow/src/bin/substitution.rs)；[工作流实现](../../../crates/df-substitution/src/workflow.rs)；[状态实现](../../../crates/df-substitution/src/status.rs)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
