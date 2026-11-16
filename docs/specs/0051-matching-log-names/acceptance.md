# 验收记录

日期：2026-11-16；Issue：[GitHub #51](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/51)。

## 实现与需求覆盖

- FR-001/002：EPW 执行主名取自输入文件；电子 bands、DOS 后处理分别使用 elebanddata、eletdos。输出、退出码、脚本及收集引用同步。
- FR-003：五类旧名由 scientific_logs 集中读取兼容；原生任务兼容匹配同时确认输入主名，新记录优先。旧任务续算不复制输出、不重新拟合；新名存在时错误不得被旧成功日志覆盖。
- FR-004：第八步与自动比较 epw.in/out 不改，原生科学数据命名不改。
- 简化原则：复用现有执行 stem 和任务记录，不增加配置、状态文件或依赖；兼容限定于五类已知历史名字。

## 构建与固定入口

执行 `bash scripts/build-native.sh build --release -p epwflow --bins --locked -j 2` 成功，直接覆盖 epw_phonon、epw_wannier、epw_superconductivity、epw_clear。当时核对四个固定入口与本轮 release 制品 SHA-256 全部相等，`--help` 全部返回零；PATH 指向仓库 bin/epwflow。没有创建版本目录、旧命令备份或编译版本记录。

构建使用当时的工作树，包含任务开始前已有的 qf-plot 曲线样式修改；该文件不属于本次提交。随后英文提示任务再次构建发布，制品包含并行工作树，不能归属为仅来自本次 Git 提交。隔离检验使用独立缓存，不覆盖共享入口。

## 回归边界

以 945b42ce 为基线，只应用本次 22 个文件补丁，在 detached 隔离工作树和独立构建缓存运行：

`bash scripts/build-native.sh test --release -p epwflow --locked -j 6 --lib --test three_stage_execution --test native_interpolation --test native_outputs`

| 目标 | 通过 | 失败 | 既有跳过 |
| --- | ---: | ---: | ---: |
| 单元回归 | 66 | 0 | 4 |
| native_interpolation | 13 | 0 | 1 |
| native_outputs | 11 | 0 | 0 |
| three_stage_execution | 110 | 0 | 1 |
| 合计 | 200 | 0 | 6 |

SC-001：五类实际输入／输出、退出码与脚本同步，旧日志不在新任务中生成。SC-002：旧 Wannier 输出内容不变、不重跑、不复制为新证据；新名错误不回退旧日志。SC-003：构建发布核对和上述回归通过。跳过项为原有真实数据或未启用场景，不计通过。

共享工作树先前一轮为 197 通过、3 失败、6 跳过：两项明确报调试二进制被并行构建删除或身份改变，另一项为并行比较路径行为变化导致旧断言失败。三项在本次隔离工作树均通过，不能将共享构建的混合状态当作本次补丁验收。原始日志保存在本地忽略的 docs/local/0051-matching-log-names/。

扩展回归发现既有 `comparison_plots::cli_collect_plot_move_and_replot_without_f4_or_external_calls` 失败：测试固定断言线宽 4／3，而任务开始前 qf-plot 已改为 2／2。保留用户原有改动，不修改测试绕过该失败；扩展回归余下无关旧入口用例主动停止，不宣称全仓库测试通过。

本轮只有隔离替身进程回归和构建核对，无真实 QE／EPW 科学计算或真实 Slurm 提交。历史计算目录未批量改名。

## 文档与协作边界

Overview 的 EPW 手册与目录图同步新规则。根 README 的操作入口不变，无需修改。未推送，远程 Spec 链接尚未验证，Issue 保持开放。
