# 0039：四类比较图的路径标签归一化与避让

唯一类型：type:bug，本地编号已查重；线上 Gitee 用户管理。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0032](../0032-disconnected-electronic-band-paths/issue-draft.md)。

## 原始需求

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。


## 真实问题

实际查看作业 1314972 产生的第 6 步四类比较总览，GAMMA 原样输出，断开的 A/L、M/H 两端只隔一小段绘图距离，标签重叠。第 5 步专用绘图已由 0032 修复，此处是 F5 共用绘图路径的遗漏。科学曲线与坐标不应为修复标签而改变。

## 处理与文档影响

电子能带、声子色散及总览共用显示规则：G/GAMMA 显示为 Γ；屏幕上相距不超过 24 像素的一组相邻标签合并为按路径顺序的 A | L 等，去掉重复符号，显示在该组两端的中间。各条高对称点竖线、路径坐标、分段和曲线保持原样，DOS 坐标不变；该合并是显示分组，不把两个物理端点改成同一点。

更新 Spec、模块手册、Overview 和算例 README；保存旧图，只用公开离线绘图重绘，核验所有科学表和原始数据哈希，不新增科学作业。验证后范围提交，不推送。
