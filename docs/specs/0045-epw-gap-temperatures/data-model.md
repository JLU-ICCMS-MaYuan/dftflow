# 0045 数据与状态

## 运行计划

版本 3 的运行清单保存 task=gap、run_id、interpolation_digest、binding_digest、plan、attempts、execution_complete 和 data_complete。plan 保存展开后的单模式 Request：温度用 0.01 K 整数，kind=gap，包含单初值、μ*、截断及迭代／混合设置；绑定涵盖配置、EPW 和 F3 来源身份。不能通过编辑记录改变实际来源。

## 温度点与原生尝试

点身份为模式＋温度，隶属固定计划。每次尝试另有编号和冻结输入，保留提交、程序、退出码、日志及产物哈希。有效成功点重验后复用；重试新建尝试并保留旧失败。活动／未知作业拒绝重复提交。

## 执行与数据状态

- attempts[].status：running 为处理中，failed 为失败，valid 为正常结束且能隙有效，completed 为正常结束但无有效能隙。valid／completed 都必须有零退出和完整原生结束证据。
- 计划中尚无尝试的点为待运行；活动／未知调度状态由原生作业回执判定，不伪造已完成尝试。
- attempt.json 的 gap_status 保存 execution_complete、converged、data_status 和 reason；converged 为 true／false／null，来自原生日志，不能由退出码推导。
- 能隙有效性：valid／not_converged／missing／invalid，包含有限数、频率、温度和实际状态身份检查；原因与原始文件保留。
- execution_complete：计划中所有点正常结束；data_complete：所有点均有有效能隙。这两个字段不能互相代替。

points.csv 列为 mode、temperature_k、initial_gap_mev、execution_status、execution_complete、converged、data_status、log_path、reason。汇总 JSON 的 gap_run 另保存尝试目录、计划完成和数据完整状态；可用 Δ 单位 meV，Z 无量纲，温度 K。F4 保存完整正 Matsubara 频率数据，F5 的 gap_summary.csv／gap_states.csv 使用最低正频率，aniso 保留真实 k／band 及权重。零能隙可以有效，不要求归类为超导态。

## 清单版本与 Tc 兼容

新清单显式标 task=gap，不填 Tc 区间、估值或伪搜索阈值；旧 Tc 清单按已有 schema 1／2 读取和验证。最终交付以任务类型分派，不能让两种完成门混用。结果包把原始输出和有效数值数据分开索引，移动重绘不启动科学程序。
