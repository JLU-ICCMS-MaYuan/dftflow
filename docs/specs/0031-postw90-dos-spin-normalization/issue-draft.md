# 0031：修复第 5 步 DOS 重复自旋归一化

唯一类型：`type:bug`。本地编号已查重，线上 Gitee 用户管理。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0018](../0018-epwflow-stage-workspaces/issue-draft.md)、[0023](../0023-epw-wannier-native-start/issue-draft.md)。

## 原始需求

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。


## 证据与修复范围

真实 1313954 的原生 MgB2.wpout 明确 Number of electrons per state=2；绘图仍把原生 DOS 乘 2，导致曲线及 CSV 重复放大。旧规范化说明错误地假定 postw90 每态只计一个电子。

非磁标量后处理显式设置 num_elec_per_state=2；重验原生日志电子数并拒绝缺失或不相容值。CSV／图直接使用原生总 DOS，不再乘二；记录原生日志与 DOS 身份。只升级后处理缓存版本，保留已成功拟合、QE、PH 与原始谱。

## 文档影响

更新 Spec／验收、Overview、模块与本例 README；纠正旧错误说明。源码、安装与真实图像／数值验收分开记录，不以任务零退出代替；保留旧现场、共享预算，不推送、不操作线上 Issue。
