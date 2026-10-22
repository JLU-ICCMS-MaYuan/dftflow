# 命令契约：声子阶段一次提交

```bash
epw_phonon -c "epwinput.toml" --stage phonon -s
```

默认声子目标同样适用；scf／nscf／electronic／Wannier／coarse 等其他选择沿用各自提交方式。成功启动输出 job_id、script、directory、workflow_status（通常 submitted；快速启动时可为 running／succeeded），明确后续自动执行，不提示逐项重复 -s。

一次 sbatch 提交声子目录 phonon.sh，日志为 slurm-phonon.log；脚本正文切换目录、直接 source 环境、执行现有程序内部工作入口。科学子脚本仍在原目录，输入／输出可读，不自动设置 shell 严格选项。

--status [--json] 只读显示阶段复用状态及父尝试；完整结果可在 worker 结束时直接查看。失败使用原命令加 --resume；已成功子项复用。活动／未知分配拒绝新提交；终态失败且未显式恢复则报错。请求完成再次执行仅核验复用与补图，不生成新科学作业。

用户手工执行旧脚本、修改固定输入或替换程序不自动成为正式交付。只更新根配置的合法新请求按现有科学身份处理，不篡改旧尝试的固定输入。

配置的 wall_seconds 覆盖本次全部未完成依赖、声子计算及出图，总时限不自动扩大；超时或取消后，调度终态确认前禁止恢复，确认后显式 --resume 复用可核验检查点。
