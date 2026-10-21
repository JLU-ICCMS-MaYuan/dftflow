# 0023：修复 Wannier 独立阶段原生启动并实测提交

唯一类型：`type:bug`。本地编号已查重，线上 Gitee 由用户管理，未创建线上 Issue。

## 原始需求记录

> 为什么我刚刚在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run提交了epw_wannier --stage wannier -s然后跑了一下就结束了，又报错了！！！！ 你来看看/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/5.epw-wannier里面的报错内容？看看是不是需要修改代码，继续需要使用 issue 和 spec 来新建需求修改。你是不是压根就没测试提交epw_wannier --stage wannier -s能不能跑起来，你来亲自跑跑测试。

## 现象与目标

真实作业 1311577（32 MPI）进入 EPW 5.9，readin 报 `opening file kpoints.dat`。独立 Wannier 阶段继承了粗网格插值点文件声明，未交付对应文件；协议替身绕过原生无条件文件检查，导致遗漏。修复独立阶段输入，亲自通过公开命令提交并跟踪真实作业，保留成功 QE 结果。

## 范围与验收

见 [Spec](spec.md)、[计划](plan.md)、[任务](tasks.md) 和 [验收](acceptance.md)。关联 [0018](../0018-epwflow-stage-workspaces/issue-draft.md)、[0020](../0020-qe-electronic-postprocess-resume/issue-draft.md)、[0021](../0021-qe-dos-pencil-resume/issue-draft.md)；独立于尚未实施的 0022 日志命名需求。无父 Issue。

## 文档影响（Documentation Impact）

修复后更新 EPW Overview、模块手册与本例 README，记录真实执行、复用证据及科学验收边界。原生日志、哈希、作业回执和安装来源留 `docs/local/0023-wannier-native-start/`；远程双向链接待用户发布后核验。

## 授权

沿用用户“不要再征求我的意见”的直接实施授权，本轮明确要求实际提交测试；不推送、不代操作线上 Issue，不改变物理参数、并行资源或质量标准。用户未逐页审阅本文，不虚构草案确认。

## 后续原始需求记录

> 我刚刚想自己运行，但是我发现运行不起来，你似乎运行起来了，不让我运行：$ epw_wannier --stage wannier -s
> scheduler_unknown: scheduler_unknown: 锁不属于原分配，不能清除未知锁
>
> 这个锁是什么意思？你来看看是不是可以继续 debug 这个epw_wannier了

## 早期结果与阻塞

输入与后处理适配、错误检查和锁归属提示已修复，源码／安装回归通过。真实拟合和原生后处理副本探针通过；完整公开后处理被 node17/node18 批处理启动信号 53 和 node09 资源等待阻塞，未关闭本地需求。助手自己的排队测试已取消，锁与预算预留均释放，用户工作区无后台占用。证据与未完成验收见 acceptance.md；验证门未满足，按 AGENTS 暂不提交，线上仍由用户管理。


## 最新验证状态

0027 用户指定子目录的公开作业 1314551 已在 node14 完成原生 Wannier／postw90、真实图像、DOS CSV 与 78+4 份哈希校验，早期集群启动阻塞解除。本地源码／安装与最终公开验证通过，证据见 [验收](acceptance.md) 和 [0027](../0027-mgb2-epwflow-example-debug/acceptance.md)。粗网格的独立质量失败不放行细网格或 Tc；最终范围提交与文档交付由 0027 汇总，线上 Issue 由用户管理。
