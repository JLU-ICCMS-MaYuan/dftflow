# 调查证据

1313954：公开 --stage wannier -s 在 node14 完成，12 节点秒，拟合与后处理回执共 80 个输出身份均通过。但实际查看电子图发现本缺陷。

真实 1313954 的原生 MgB2.wpout 明确 Number of electrons per state=2；绘图仍把原生 DOS 乘 2，导致曲线及 CSV 重复放大。旧规范化说明错误地假定 postw90 每态只计一个电子。

非磁标量后处理显式设置 num_elec_per_state=2；重验原生日志电子数并拒绝缺失或不相容值。CSV／图直接使用原生总 DOS，不再乘二；记录原生日志与 DOS 身份。只升级后处理缓存版本，保留已成功拟合、QE、PH 与原始谱。

原生源文件及图已留本地，失败前截图与数值在 docs/local/0027-mgb2-epwflow-example-debug/。W90 parameters.F90 非 spinors 默认每态 2 电子；postw90 的 dos.F90 已在原生谱中计入该值。日志、输入与实际数值共同判断，不凭峰形猜测。
