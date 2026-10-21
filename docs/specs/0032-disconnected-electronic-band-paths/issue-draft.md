# 0032：修复第 5 步断开能带路径与重叠标签

唯一类型：`type:bug`。本地编号已查重，线上 Gitee 用户管理。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0018](../0018-epwflow-stage-workspaces/issue-draft.md)、[0023](../0023-epw-wannier-native-start/issue-draft.md)。

## 原始需求

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。


## 证据与修复范围

真实 MgB2_band.labelinfo.dat 的 A/L 与 M/H 分别位于同一横轴位置。旧绘图连续连接重复 x 的不同能量，产生假竖线，并将两标签画在同一位置。

在两侧原生曲线的重复 x 处拆开绘制，合并同位置不同端点名为 A | L、M | H，重复同名去重，GAMMA 仍显示 Γ；保持 QE 实线先绘、Wannier 虚线后绘、原数据和能窗不变。升级独立后处理缓存，不重算拟合或 QE。

## 文档影响

更新 Spec／验收、Overview、模块与本例 README；纠正旧错误说明。源码、安装与真实图像／数值验收分开记录，不以任务零退出代替；保留旧现场、共享预算，不推送、不操作线上 Issue。
