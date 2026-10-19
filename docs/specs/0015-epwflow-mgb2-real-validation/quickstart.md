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

第二批补充后，上述默认命令为 1 通过、11 ignored；显式命令为 11 项已知失败（退出 101）。首批“8 项”是历史批次记录，当前数量以本段及 acceptance.md 的最新结果为准。程序路径存在也不能绕过尚未接通的提交入口。

## 2026-10-19：叠图快速验收步骤（待实现后执行）

本段为已确认的直接命令契约，不宣称当前 collect／plot／results check 或完整 submit 已实现。先完成 [任务](tasks.md) 的比较数据与交付适配，再用固定配置执行到插值边界；质量失败时保留原失败状态和完整诊断数据，不继续细网格 EPC／超导。

```bash
epwflow submit -c "input.toml" --until interpolation
epwflow status -w "run" --refresh --json
epwflow collect -w "run" --output "comparison-results"
epwflow plot --results "comparison-results" --formats png,svg,pdf
epwflow results check --results "comparison-results" --json
```

W、配置与输出目录应换成已准备的隔离运行路径；这里不提交真实作业。仅需叠图时，不要求先生成 Eliashberg／Tc 清单。collect、plot、results check 不触发科学程序或调度器；缺参考数据由上游按预算受控准备。

验收按以下顺序记录：

1. 对比共同能窗、路径／坐标、DOS 网格／权重、投影、E_F、核／展宽和单位，复查来源哈希；缺投影不得用总 DOS 补齐。
2. 检查四张独立图和 comparison_overview 的 PNG／SVG／PDF，确认 QE 实线在前、EPW 虚线在后，声子 QE matdyn 曲线和直接 ph.x 点分别标注，电子与声子元素／轨道面板可读。
3. 将完整结果包复制到另一个临时目录后，只运行 plot 重新绘图；核对表格和原始数据哈希不变，QE／EPW／sbatch 调用为零。
4. 核查数值报告、缺项和质量状态，质量失败的诊断图必须显示整体未完成；三 PH 来源分别留证，图像成功不替代原科学门。

新真实数据仅放忽略的 `test_auto/MgB2/local_run/001-epw-validation/`；保留旧计算，沿用唯一预算。不能把旧少量误差点或原生冒烟当成本次四图与严格 PDOS 已通过。
