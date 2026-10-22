# 0046 原生任务记录

沿用 `epw-state/native-jobs`、history 和 native-active 三份记录，不新增实体或锁。

- 版本 1：原格式，保留 token 脚本与 Slurm 原生日志读取。
- 版本 2：字段不变，`script` 指向固定功能脚本；`stdout/stderr` 指向直接写入的科学 `.out/.err`。prepared、submission_pending、submitted、unknown 仍检查脚本身份；awaiting_validation、validated、failed 不以该可覆盖脚本作为复用依据。输入、科学来源、输出身份仍验证。
- 状态转移和预算预约／结算保持原语义。中断后可能仍存在未知提交；该状态继续提示核对回执，不因为移除锁而擅自重提。
