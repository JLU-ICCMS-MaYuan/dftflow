# 数据模型

- SCF snapshot：正常收敛 outdir 的独立树，波函数与 save 文件齐全；manifest 保存来源输入内容指纹，输入变化拒绝复用。
- q_index_map.json：mode、qpoints[]；每项 directory、index、q_cart、q_crystal，保留旧 coords 字段兼容读；序号和目录唯一，坐标有限。
- chain_jobs.json：pre、各 q 目录、post 的 job id；原子写入，每次提交即保存。调度记录不代替结果验证。
- workflow_status.json/task_status.json：保留既有三态，复用 SCF 的目录用 scf_reused.json 标记，PH 输出和 dyn 完整才 finished。
- chain_complete.json：所有步骤和输出验证完成后产生；配置变更不允许继续复用。

## US3 初始化与独立工具

- ph_init.in/ph_init.out：QE 真实初始化输入与日志，单步与自动链路共用。
- ph_init_complete.json：prefix、q_count、stage=initialization；在正常退出和 dyn0 校验后原子写入。只表示初始化成功，重新执行前删除，不影响完整声子状态。
- prefix.dyn0.before-init-时间戳：单次重初始化之前归档的旧列表，防止本次缺失输出时误读。
- 独立工具 q_points.json：保留原工具 schema，含 q_cart/q_crystal/权重等字段；不作为新主流程输入。主项目仍能读取旧计算目录对应数据。

## US4 保存目录缺省值

省略 qe_phonon.scf.outdir 时保存树为工作目录下 tmp/prefix.save；各 q 的树为数字目录/tmp/prefix.save，PH 数据合并至工作目录/tmp/_ph0。显式目录覆盖此默认值；scf_snapshot 的名称和结构不变。
