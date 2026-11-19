# EPW 5.9／6.0 原生运行证据与误拒绝恢复

> **协作入口（2026-11-17）**：[GitHub Issue #69](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/69)。Issue 已在 GitHub 登记，协作状态以 GitHub 为准；本文件保留设计与历史验收，旧的“本地草案／线上自管／未发布”说明是迁移前语境。Spec 远程链接待发布及验证。

关联 [GitHub Issue #69](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/69)。当前用户报错的完整原话在 Issue 中保存；现有测试／debug 授权覆盖该缺陷修复。

## 原始需求记录

> $ epw_wannier -c "input.toml" --stage wannier -s\
> 复用：1.qe-scf\
> 复用：2.qe-nscf\
> 复用：3.qe-electronic\
> reuse: electronic/projwfc\
> 诊断图：/home/mayuan/code/dftflow/examples/MgB2/ubuntu/run/5.epw-wannier/diagnostics\
> unknown: environment_incompatible: 缺少 EPW 5.9 运行证据


## 要求与验收

- FR-001／SC-001：从唯一、独立行的 Program EPW v. 运行头识别 5.9 或 6.0；缺失、多个运行头、未知版本拒绝，不接受输入标题或普通文本中的版本字样。
- FR-002／SC-002：退出码、原生错误、结束计时和各阶段科学语义检查继续保留，多温度入口复用同一版本识别，不放宽其他输出格式。
- FR-003／SC-003：仅对 Wannier 的旧“缺少 EPW 5.9 运行证据”误拒绝，且运行头为 6.0、退出码 0、Slurm COMPLETED、原生输入记录非空且实际输入和输出存在时，恢复到待验证；后续仍完整验证拟合、实际四界与模型，不手改用户文件、不重跑拟合。
- FR-004／SC-004：测试后覆盖 EPW 四入口，当前现场在不提交作业的准备模式中复核并推进；需要后续原生 DOS 作业时只准备脚本，由用户手动提交。

不处理已支持布局之外的二进制适配，不自动回收其他失败任务，不修改输入、旧原生日志、原模型或已有上游计算。
