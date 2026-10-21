# 0033：补齐并校验 Wannier 到粗网格的速度矩阵数据交付

唯一类型：`type:bug`。本地编号已查重；线上 Gitee 由用户管理。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0018](../0018-epwflow-stage-workspaces/issue-draft.md)、[0030](../0030-wannier-guiding-centres/issue-draft.md)。

## 原始需求

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。

## 已确认的故障

公开粗网格作业 1314095（node14／32 MPI，FAILED/1:0，128 节点秒）已复用五个上游阶段并完成独立参考；原生 vmebloch2wan 报缺 MgB2.bvec。源程序同一路径还必须读取 .mmn，生成器只复制了 .ukk/.wout/.win，且第 5 步收据没有绑定 .bvec/.mmn。不是科学质量超限。

## 修复与兼容

第 5 步校验两项原生文件的格式、维度、完整性及有限数，纳入哈希收据；第 6 步只复制同一收据绑定且校验通过的配套文件。保留原生 vme，不改物理网格或能窗。旧收据缺绑定时判为需要重建第 5 步，不自动补认未绑定数据，不手改历史收据。保留相容 QE／PH 和已完成独立参考。

## 文档影响

更新 Spec、Overview、模块／本例 README，区分旧收据升级重建、源码／安装与真实质量。保留历史、预算和原始错误；验证后范围自动提交，不推送、不操作线上 Issue。
