# 0066 验收

## 当前结论

原始 Mg 旧赝势模型可用。当前失败由二进制 Vanderbilt 数据误入 QE 文本读取分支导致；没有更换赝势模型，只转换编码。Slurm 未启用 accounting 的状态、计费与退出码查询已采用真实完成记录恢复，无系统配置修改。

正式 CLI 已通过 --resume -s 重新生成 12 路脚本并提交作业 4；9 次 SCF 迭代收敛，QE WALL 53.75 秒、最终自由能 −138.11356494 Ry，原生日志 JOB DONE。Slurm 为 COMPLETED、12 CPU、1 节点、60 秒、退出码 0:0。再次执行公开 SCF 命令返回 stage_complete=true、workflow_complete=false。未执行 NSCF／PH／EPW；此项不证明超导或数值网格收敛。

## FR／SC 证据

- FR-001／SC-001：根据同版本 QE read_uspp.f90 的 !unf 注释适配未格式化读取及三个合并记录；原生 write_upf 导出、read_ps_new 重读。PBE、10 价电子、821 径向点、3 原子波函数、7 beta 保持；r/rab/vloc/rho_at/beta/dion/qqq/qfunc/qfcoef/chi 逐数组满足 1e-12×(1+最大绝对值) 标准。最大绝对差来自大系数 qfcoef，约 4.55e-12；其他数组最大约 4.27e-14。原始文件不变，B 本地副本逐字节一致。
- FR-002～003／SC-002：只在 sacct 明确禁用 accounting 时降级。控制器确认运行中；配置中明确为 jobcomp/filetxt 时，私有 0600 副本供原生 sacct 查询，查询后删除。使用 NNodes，规避完成插件对 AllocNodes 返回 4294967294 哨兵的实际行为。终态／退出码统一后端，缺失、重复、查询失败和一般数据库错误不伪造完成。
- FR-004／SC-003：作业 3 的真实 FAILED、3 秒、1:0 从完成日志核验并结算后，经公开恢复提交作业 4。旧失败原生日志保存在忽略的 repair/failed-scf-evidence/。脚本 ntasks=12、mpirun -np 12；公开阶段日志状态与科学状态均已完成。
- SC-004：EPW 四入口重新编译并直接覆盖固定 bin/epwflow，未创建版本安装；当前模块手册、Overview、Ubuntu 输入／手册已同步。线上 Issue 仍由用户管理，未推送。

## 回归与执行顺序

分批回归去重后 361 通过、15 既有 ignored，无未解决失败。新进程回归覆盖 completed、running、missing、duplicate、query_failed、other_error 六种情形，并验证原日志权限／内容不变、临时副本清理、真实后端退出码和 best-effort 状态查询。

首次全量覆盖是在退出码回退补齐前启动的；并行构建又替换了长链测试当时正在执行的缓存入口，导致一个 current_exe 含 deleted 的失败。停止交叉编译／测试后，对最终代码补跑 65 项库回归、新完成日志测试及 foundation／real_contracts／slurm_log_filenames／three_programs，单独复测该完整公开链通过，最后补跑 10 项能窗回归。没有将失败全量命令冒充单次全量通过。所有测试结束后再次构建覆盖正式入口。

## 真实记录与边界

账本 spent_node_seconds=63，allocations=0、reservations=0；由旧失败作业 3 秒和新成功作业 60 秒构成。没有重置账本或手改任务号／状态。系统原完成日志仅由 Slurm 自身追加新作业记录。

转换程序、转换日志、原始文件校验与科学原始输出只存 examples/MgB2/ubuntu/repair 和 run。完整构建／回归输出只存忽略的 docs/local/0066-mgb2-legacy-pseudo-slurm，不纳入 Git。Ubuntu 目录仍只跟踪 input.toml 与 readme.md。删除本地转换文件后须重建同模型转换；通用二进制赝势自动识别不在本项范围。
