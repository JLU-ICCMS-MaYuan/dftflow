# 0029：阶段归档后保留当前作业 Slurm 日志入口

唯一类型：`type:bug`。本地编号已查重；线上 Gitee 由用户管理。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0018](../0018-epwflow-stage-workspaces/issue-draft.md) 与 [0025](../0025-epw-visible-run-state/issue-draft.md)。

## 原始需求记录

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。


## 复现与处理

真实作业 1313468 的 Slurm 日志随阶段目录移入历史，阶段目录中看不到当前日志。Slurm 仍持有旧 inode，简单复制不能保留后续输出。

阶段归档后，仅对当前合法 SLURM_JOB_ID 对应的普通日志文件建立同 inode 硬链接，保留历史现场及阶段目录入口；不触碰旧作业日志，不改冻结脚本，不放宽脚本哈希保护。文件系统不支持时明确失败。

## 文档影响与关闭门槛

更新模块手册、指定算例 README、Overview 及本 Spec 验收；源码协议、安装与真实科学结果分别记录。完成 [任务](tasks.md) 与 [验收](acceptance.md) 后才能标记修复；不推送、不操作线上 Issue。
