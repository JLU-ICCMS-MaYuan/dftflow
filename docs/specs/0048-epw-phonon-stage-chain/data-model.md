# 数据模型：复用父尝试和原生子任务

## 父尝试

沿用 epw-state/attempts/<id>/attempt.json 与 active.json；增加可缺省的 phonon_chain 与 retry_failed 标志，历史记录缺省 false。保留 config／executable／sources／script 身份和 reservation／job／state／error。

状态为 prepared → submission_pending → submitted → running → succeeded／failed。提交端将作业号或未知错误写入同尝试目录的 submission.json；读取时合并，提交端不覆写 worker 的 running／终态。提交响应未知为 unknown，保留预留、禁止重提。worker 可早于 sbatch 返回启动，使用 Slurm 分配身份幂等绑定同一预算预留。

## 原生子任务

沿用 epw-state/native-progress/<step>.json 和 native-jobs/<key>.json；allocated 仅是进程内执行上下文，不增加磁盘格式。成功脚本的 stdout／stderr／exit_status 写入现有 Task，科学归集成功后验证回执；失败保持可重试证据。每个子任务不再关联单独 Slurm 分配或新预算。

## 科学与图像交付

继续使用阶段 completion.json、qe-data 清单、save/manifest.json、0047 plots/completion.json。图像缺失只影响图像交付，不让有效科学调用失效。状态查询保留父 Attempt 终态，不能用所有八阶段均已完成来代替声子目标完成。
