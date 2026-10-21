# 调查证据

部署的 W90 3.1.0 parameters.F90 支持 guiding_centres，缺省 false，原生日志 Use guiding centre to control phases 报告 T/F。1313788 的指导中心和短试探步长两份独立副本均有 Wannierisation convergence criteria satisfied；不是仅凭进程零退出判断。原失败 chk/mmn/win/wout 保留，探针不产生公开工作区完成记录。

500→5000 仍振荡、固定步长 0.1 加最陡下降仍未收敛；本例优先采用一个原生布尔开关，避免新增多种优化器配置。该选择只适用于本次已测配置，不宣称普适收敛保证。证据在忽略的 docs/local/0027-mgb2-epwflow-example-debug/ 及工作区 diagnostics/。
