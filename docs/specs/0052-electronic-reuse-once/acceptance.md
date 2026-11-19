# 验收记录

日期：2026-11-16；关联 [GitHub #52](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/52)。隔离源码基线为 d57d89b9，仅叠加本次修复。

## 实现与验证映射

- FR-001／SC-001：首次主电子参考核验包含已有 PDOS；电子派生交付成功后集中打印 reuse: 3.qe-electronic，删除普通投影及原生投影检查点的独立复用提示。整个目标已完成的快捷返回保留总体完成提示。
- FR-002／SC-002：持锁准备／推进作用域内按完整配置共享主电子有效性、PDOS 产物确认及电子派生刷新；实际回调计数测试证明四次请求各只执行一次。
- FR-003：SCF／NSCF／Electronic 清理或运行、PDOS 重新生成前清除确认；作用域结束丢弃，配置不匹配绕过。错误、未完成投影和失败刷新不记成功。
- FR-004：公开 CLI 测试从完整粗网格结果进入复用，仅一次第 3 步主提示、零 projwfc 子提示；删除解析检查点及图像后复用原生投影修复，科学及投影调用数均不增加；下一条命令拒绝损坏的电子日志。
- 简化原则：复用原有检查及派生交付函数，只增加本次操作内的确认，不增加磁盘状态、配置或依赖，不扩大到其他阶段的科学核验。

## 隔离回归

命令：

`bash scripts/build-native.sh test --release -p epwflow --locked -j 4 --lib --test three_stage_execution --test native_interpolation --test native_outputs -- --skip numbered_dos_failure_public_resubmit_preserves_pw_and_bands`

| 目标 | 通过 | 失败 | 既有跳过 | 单独排除 |
| --- | ---: | ---: | ---: | ---: |
| 单元测试 | 81 | 0 | 4 | 0 |
| native_interpolation | 13 | 0 | 1 | 0 |
| native_outputs | 11 | 0 | 0 | 0 |
| three_stage_execution | 113 | 0 | 1 | 1 |
| 合计 | 218 | 0 | 6 | 1 |

其中新增 6 项确认作用域测试，覆盖实际执行次数、来源重算失效、后续阶段不重复、配置及命令边界、失败恢复和未进入作用域的实时核验。新增 1 项公开进程测试覆盖集中提示、原生投影恢复和新命令坏日志拒绝。原有声子／Wannier／超导、强制 SCF 重算、旧数据迁移和图表修复回归保留。

## 已有失败的独立复现

未排除时为 218 通过、1 失败、6 跳过。失败用例 numbered_dos_failure_public_resubmit_preserves_pw_and_bands 在 support/direct_submit.rs:421 要求 DOS 已失败后仍存在完整 electronic.png。

将隔离测试的三个公开入口临时指向本次修改前的主工作区固定命令，单独执行同一用例，在同一行和相同断言失败，退出码 101。随后恢复测试源码。本轮不修改该旧断言，不将排除项计为通过，也不声称所有既有测试已通过。此问题属于已有 DOS 失败与组合图交付行为，不属于电子复用合并。

原始日志留本地忽略目录 docs/local/0052-electronic-reuse-once/。

## 构建与发布

已执行 scripts/build-native.sh build --release -p epwflow --bins --locked -j 4，四个 EPW 固定入口成功覆盖。逐个比较安装入口与 release 制品 SHA-256 一致，--help 全部返回零。将同一公开进程专项的三个入口指向主工作区已安装命令再次执行，集中提示、原生投影修复及新命令坏日志拒绝全部通过，退出码零。编译保持固定命令覆盖，不保留旧命令备份。主工作区预先存在的 qf-plot 曲线样式、VASP 测试和 MgB2 配置修改均不纳入本次提交，发布还包含当时另一任务对声子比较／采样相关文件的未提交修改，未纳入本次补丁；本次完整回归使用隔离基线加本次补丁，安装入口专项验证使用实际已发布制品。

## 文档与协作

Overview 的 preflight-and-reuse.md 记录集中检查及失效边界。根 README 的操作方式不变，无需修改。Issue 保持开放：未推送，远程 Spec 链接尚待发布与验证。本轮没有真实 QE／EPW 科学计算或真实 Slurm 提交。
