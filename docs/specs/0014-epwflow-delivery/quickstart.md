# 快速验证：图表、真实对照与安装交付

本文为阶段实施蓝图；已有部分 epwflow 输入、解析及只读检查，完整自动链与新增叠图尚未验收。以下规划命令不是已运行证据，实际状态见 [验收记录](acceptance.md)。

## 前提

Rust/Cargo 和 workspace 依赖就绪；使用 contracts 的有效 input.toml，结构、赝势和目标程序真实存在。生产提交前完成 F1 版本证据与唯一预算账本登记；运行资源内 run 必须登记当前 allocation。真实路径保存在忽略目录，不将本文占位示例当成实际机器路径。

## 命令与预期

```bash
cargo test -p epwflow --test delivery
epwflow check -c input.toml --json
epwflow prepare -c input.toml
epwflow submit -c input.toml
epwflow status -w run --refresh --json
epwflow collect -w run
epwflow plot --results run/results
epwflow results check --results run/results --json
```

自动测试须使用隔离临时目录和可执行 fixture，不占用真实队列。真实 submit 只能在自动验证通过后、预算预留内执行。状态可区分已提交/未知/失败/成功，不能根据命令退出零推断科学成功。

## 失败与恢复核验

分别触发 Spec 所列缺文件、错误版本、损坏/篡改、非有限值、预算不足和未知作业；应报告具体字段/阶段并阻断依赖。不改物理参数，修复外部条件后仅用显式 resume 续算，有效完成项执行次数不增加。再次运行与从其他目录启动必须读取相同输入绑定。

## 验收输出

按 tasks 覆盖全部 FR/SC，实际证据存 docs/local/epwflow-acceptance/；先记录未运行，不能通过脚本存在或帮助输出完成数值验收。共享预算耗尽时停止追加并保留现场。

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
