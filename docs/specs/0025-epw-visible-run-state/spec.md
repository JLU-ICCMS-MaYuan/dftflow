# 0025：EPW 可见运行状态与阶段重建

关联：[本地维护 Issue](issue-draft.md)。用户在需求落档后回复“开始”，授权按该目标实施；不操作线上 Gitee，不切换默认安装。本文件细化已授权维护范围，不另行扩大 Feature。

## 原始需求记录

```text
我不希望搞什么隐藏目录来记录运行状态。

(my_scripts) [mayuan@login02 run]$ rm -fr 5.epw-wannier/
(my_scripts) [mayuan@login02 run]$ epw_wannier --stage wannier -s
input_conflict: input_conflict: 无法定位 /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/5.epw-wannier/MgB2.chk: No such file or directory (os error 2)
(my_scripts) [mayuan@login02 run]$ ls
1.qe-scf             6.epw-coarse             input.toml  qeflow         substitutionflow
2.qe-nscf            7.epw-fine               KPATH.in    README.md      vaspflow
3.qe-electronic      8.epw-superconductivity  POSCAR      scflow         wannierflow
4.qe-phonon-nosplit  epwflow                  pp          submit_all.sh
(my_scripts) [mayuan@login02 run]$  这是为刚刚的操作，这是为什么啊？

你说是因为：删掉了第 5 步的计算文件，但 .epwflow/ 中可能还保留着它的成功记录。程序随后尝试校验旧的 MgB2.chk，因文件不存在而停止。 

我不希望搞什么隐藏目录来记录运行状态。

你来新建一个 issue完成这个目标。
```

> 开始

## 背景与需求结论

删除编号阶段目录应当使该阶段失效；运行状态必须可见。保留当前科学 DAG、文件身份、排队快照和预算保护。具体可见文件名属于本次可逆实现选择，不改变公开科学入口。

## 用户场景与验收

- **US1（P1）**：普通 ls 可找到运行状态；新工作区经三个入口生成、提交和恢复，均不产生隐藏状态目录。
- **US2（P1）**：删除第 5 阶段或必要产物后，重新提交可重建该阶段，完整相容上游复用，下游不复用过期交付。
- **US3（P1）**：旧工作区经明确的一次性迁移保留历史、可复用上游和原账本；活动或未知作业拒绝迁移及重复提交。

## 功能需求

- **FR-001**：阶段完成与电子原生任务回执归各自编号阶段；共享运行记录位于可见目录，旧非编号主路径也不创建隐藏状态。
- **FR-002**：阶段目录或必要产物缺失返回不可复用；递归传播给真实下游。篡改、不可读、记录损坏仍明确报错。
- **FR-003**：重建使用当前配置，排队任务使用原冻结配置；活动、未知提交及并发锁继续阻止重复执行。
- **FR-004**：明确识别旧隐藏状态，提供显式迁移；迁移保留原目录与账本，不静默复制活动任务或把旧脚本当成新提交。
- **FR-005**：状态查询只读；记录迁移边界、目录与恢复命令，保持科学结果验收独立。

## 成功标准

- **SC-001**：三个公开入口的生成与恢复进程回归通过；工作树无 EPW 自建隐藏状态目录。
- **SC-002**：完成拟合后删除目录或 chk，公开提交成功，执行只重建需要更新的科学阶段；粗、细和超导均不得显示旧依赖仍可复用。
- **SC-003**：完整复用、内容篡改、损坏回执、排队/运行/未知状态各有明确回归证据。
- **SC-004**：迁移保持原隐藏文件、科学数据与预算原文；成功迁移后即使旧目录移走仍可生成、查询和复用已迁入的完整上游。

## 依赖、边界与确认

关联 0018、0008、0023。仅 EPW 管理工作区；不扩展其他 flow，不改变数值标准，不运行真实科学计算。不声称已安装程序同步生效。无附件，images: []。原需求与当前“开始”构成维护实施授权；没有另行宣称独立 Feature Spec 已获逐字审查。远程 Issue/Spec 双向链接待用户上线核验。
