# MgB₂ 旧赝势读取与无数据库 Slurm 续算修复

> **协作入口（2026-11-17）**：[GitHub Issue #66](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/66)。Issue 已在 GitHub 登记，协作状态以 GitHub 为准；本文件保留设计与历史验收，旧的“本地草案／线上自管／未发布”说明是迁移前语境。Spec 远程链接待发布及验证。

关联 [GitHub Issue #66](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/66)。用户明确要求修复当前真实失败，沿用既有测试／debug 授权。本项不改变物理参数或赝势模型，不修改系统 Slurm 配置。

## 原始需求记录

> 帮我解决一下，我觉得老赝势应该也是可以读取的

## 已核验原因与验收

- FR-001：保留原二进制 Mg 赝势，只转换编码格式；利用同版本 QE readvan 的上游 !unf 适配说明及原生 write_upf 导出，再用原生 read_ps_new 重读对照物理数组。
- FR-002：sacct 明确报告 accounting 未启用时，支持控制器确认正在运行的作业、jobcomp/filetxt 确认已结束作业；原始完成日志只读，查询副本临时且自动删除。其他查询错误不降级猜测。
- FR-003：未知／重复／损坏记录仍阻断；真实节点数、ElapsedRaw 和终态仍由 Slurm 返回，不清空账本或伪造结束。
- FR-004：修复当前输入和 README，经正式 CLI 恢复并用 12 MPI 完成一次 SCF。
- SC-001：原赝势未改变；转换前后元素、泛函、价电子、网格、beta 和全部相关数组按 1e-12 相对标准核对通过。
- SC-002：禁用 accounting 的终态、运行中、缺失／冲突记录、真实查询失败的进程回归通过。
- SC-003：公开 --resume -s 使用 12 MPI 完成真实 SCF，原生日志收敛且 JOB DONE；阶段复核成功，未执行 NSCF／PH／EPW。
- SC-004：源码重新编译覆盖既有 CLI，原始日志和历史证据保持；当前文档与本地 Issue 关联更新。

范围外：不升级或替换 QE，不安装 Slurm 数据库，不更换另一份 Mg 赝势，不推进完整超导链，不自动推送。
