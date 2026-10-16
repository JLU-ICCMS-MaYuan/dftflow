# 直接验证命令与当前限制

```bash
git pull --ff-only --no-recurse-submodules origin master
cargo test -p epwflow --locked --offline
./target/debug/epwflow --help
./target/debug/epwflow submit -c /does/not/exist.toml
./target/debug/epwflow submit -c /does/not/exist.toml --until interpolation
```

当前基线：41 项通过；submit 即使配置不存在仍直接 not_implemented，--until 被拒绝。这是失败复现命令，不是提交真实作业的教程。

修复后应能按原 Spec 使用 check/prepare、qe-data import、submit --until interpolation、status/resume 和完整 submit。真实配置应放隔离的 examples/MgB2/sugon-amd/epw/0015/ 模式目录，实际配置、命令及作业号在验收后回写本地手册。当前不提供声称已经可用的生产续算命令。

独立参考必须人工核查底层输入和原生输出，再运行规范化比较；不能复制 epwflow 生成的合成数据充当参考。计算使用预算登记的 Slurm 分配，不在登录节点启动科学求解器。

实现模型先阅读 [交接清单](handoff.md)，按批次提交修复和证据；本助手使用同一套基线命令及真实格式复测。当前命令的失败预期只适用于审查基线，修复后应更新证据而非继续依赖历史结论。

## 单独复现已知缺陷

```bash
cargo test -p epwflow --locked --offline --test real_contracts
cargo test -p epwflow --locked --offline --test real_contracts -- --ignored --test-threads=1 --nocapture
```

当前第一条为 1 通过、8 ignored；第二条退出 101，8 项失败，明确显示待修复缺陷。这里的失败是有意检查正确契约后发现实现不满足，并非完成验收。实现模型修复后逐项运行、取消 ignore，再执行完整回归。详细对应关系见 [验收记录](acceptance.md)。
