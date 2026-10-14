# 数据模型

- SCF snapshot：正常收敛 outdir 的独立树，波函数与 save 文件齐全；manifest 保存来源输入内容指纹，输入变化拒绝复用。
- q_index_map.json：mode、qpoints[]；每项 directory、index、q_cart、q_crystal，保留旧 coords 字段兼容读；序号和目录唯一，坐标有限。
- chain_jobs.json：pre、各 q 目录、post 的 job id；原子写入，每次提交即保存。调度记录不代替结果验证。
- workflow_status.json/task_status.json：保留既有三态，复用 SCF 的目录用 scf_reused.json 标记，PH 输出和 dyn 完整才 finished。
- chain_complete.json：所有步骤和输出验证完成后产生；配置变更不允许继续复用。
