# 分支状态

唯一状态载体仍为 epw-state/3.qe-electronic.log；Slurm 编号仍在第 3 步 .jobs。

- Electronic branch · qe_eband / qe_eledos：Submission ID、Status、Configuration、Script，必要时 Failure。
- Stage：批次与科学配置；子任务完成检查点继续绑定批次及各自配置版本。
- Stage allocation：收尾作业，继续使用既有字段与入口。

状态顺序：prepared → submitting → submitted → running → completed / failed；提交回执无法确定时为 submission unknown，禁止自动重提。
进程启动可在 sbatch 返回前登记 running；父进程不得用 submitted 覆盖已经推进的状态。
