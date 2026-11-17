# 0046 原生任务记录

> 2026-11-17 最新约定：预算账本功能已按用户要求彻底撤销。本文件涉及账本、campaign 身份、预算预留／结算及全局两个作业限制的旧条款均已失效；当前计算只依赖工作流记录、Slurm 作业身份及科学核验。历史消耗与验收记录不改写。详见 [0058 最新需求](../0058-epw-remove-resource-limits/spec.md)。

沿用 `epw-state/native-jobs`、history 和 native-active 三份记录，不新增实体或锁。

- 版本 1：原格式，保留 token 脚本与 Slurm 原生日志读取。
- 版本 2：字段不变，`script` 指向固定功能脚本；`stdout/stderr` 指向直接写入的科学 `.out/.err`。prepared、submission_pending、submitted、unknown 仍检查脚本身份；awaiting_validation、validated、failed 不以该可覆盖脚本作为复用依据。输入、科学来源、输出身份仍验证。
- 状态转移和预算预约／结算保持原语义。中断后可能仍存在未知提交；该状态继续提示核对回执，不因为移除锁而擅自重提。
