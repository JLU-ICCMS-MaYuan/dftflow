# 0034：修复编号独立参考在原路径升级编排程序后的恢复

唯一类型：`type:bug`；本地编号已查重，线上 Gitee 由用户管理。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0033](../0033-wannier-velocity-handoff/issue-draft.md)、[0018](../0018-epwflow-stage-workspaces/issue-draft.md)。

## 原始需求

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。

## 故障与修复范围

1314095 已完成独立参考，request.json 第六来源是 bin/epwflow/epw_wannier。编号恢复本来允许修改编排程序，却仍验证旧可覆盖路径哈希，并把旧／新身份重复加入参考来源；原路径安装新版本后必然无法复用。五项科学来源与原生子任务均有独立哈希，不应以历史编排器的可变路径否定科学结果。

编号恢复严格核对原请求、参数、五项科学身份和全部原生任务证据；原编排器身份保留在不改写的请求／参考记录中，用作历史追溯，当前版本由本次冻结提交记录追溯。只对自动生成的本地编号参考按精确绑定识别历史编排器，其他参考／外部文件及科学来源仍严格核验。旧模式契约保持。

## 文档影响

更新 Spec／验收、Overview 与手册，明确追溯身份和活动数据依赖的区别；公开恢复不重算已完成参考，损坏科学数据仍拒绝，预算及原始来源不变。验证后范围自动提交，不推送。
