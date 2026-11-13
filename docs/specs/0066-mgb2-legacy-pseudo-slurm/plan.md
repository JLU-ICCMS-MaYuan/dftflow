# 修复计划

1. FR-001／SC-001：私有转换器按同版本 QE 注释恢复无格式读取和三个合并记录；连接已有 QE 库，用原生写入／读取往返核对，原数据只读。
2. FR-002～003／SC-002：仅在 sacct 明确禁用 accounting 时查询控制器／原生完成日志。由 scontrol show config 获取 filetxt 路径，只读复制到 0600 临时文件；通过 sacct --completion --file 查询，使用 NNodes 避免 AllocNodes 的未初始化哨兵。
3. FR-004／SC-003～004：当前输入指向同一原势转换文件；编译覆盖入口，恢复旧失败任务、提交一次 12 MPI SCF；完成原生和阶段校验，更新文档并提交。
