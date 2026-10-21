# 0030：原生 Wannier 局域化中心相位引导配置

唯一类型：`type:feature`。本地编号已查重，线上 Gitee 用户管理。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0012](../0012-epwflow-interpolation/issue-draft.md) 和 [0018](../0018-epwflow-stage-workspaces/issue-draft.md)。默认优化器未收敛属于数值问题，本项分类为补齐参数入口，不伪称外部程序 Bug。

## 原始需求记录

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。


## 已核实的问题与范围

真实 1313621／1313723 在 500／5000 次内未局域化收敛。副本原生探针 1313765 固定小步长仍未收敛；1313788 对照中 guiding_centres=true 与短试探步长分别在原 1e-8 Å² 容差和 5 步窗口下收敛，spread 约 6.606811 Å²。当前 TOML 不能配置该原生开关。

只增加可选布尔值 wannier.guiding_centres，缺省不输出、不改变旧科学键；显式配置时传给 EPW 内嵌 W90 与独立 W90，并从原生日志核对实际采用值。不会自动切换用户参数，不增加任意 wdata 注入、外部升级或第二套执行链。

## 文档影响

更新模块手册、Overview、指定算例 README、Spec 与验收。源码、安装、诊断探针及完整公开链分开记录；真实验收不能由探针替代。沿用原预算，不推送、不操作线上 Issue。
