# 0037：显式声子路径忽略 QE 未初始化的权重字段

唯一类型：`type:bug`，本地编号已查重，线上 Gitee 用户管理。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0034](../0034-reference-orchestrator-upgrade/issue-draft.md)。

## 原始需求

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。


## 已确认的问题

作业 1314626 原生 EPW 退出 0，粗 EPC 完成。比较 matdyn 也正常完成，但 215 个路径／均匀参考点中 12 个坐标行的第四列写成 **********，与第三坐标相连，split_whitespace 误报截断。QE 7.4.1 matdyn.f90 473～480 行在 q_in_band_form=false 路径只分配 wq、没有赋值，765 行仍输出。该字段不用于本项目路径绘图和 DOS 累积。

## 处理范围与文档影响

独立提供显式路径解析：按原生固定宽度保留前三坐标，第四字段只作未使用占位；严格校验所有坐标、频率、维度、点序与完整性。普通 parse_matdyn 的非负有限权重检查保持；EPW parse_bands 保持。自动比较及比较导出使用路径解析，不改 QE 和原始文件。

更新模块手册、Overview、本例 README 和本目录验收；源码／安装／真实数值分别记录。复用原预算，不放宽质量门，不推送、不操作线上 Issue。
