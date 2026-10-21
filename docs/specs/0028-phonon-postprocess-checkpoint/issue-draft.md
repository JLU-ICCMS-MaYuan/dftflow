# 0028：编号声子目录 dyn0 自复制截断与后处理续算

唯一类型：`type:bug`。本地编号已查重；线上 Gitee 由用户管理。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0018](../0018-epwflow-stage-workspaces/issue-draft.md) 与 [0025](../0025-epw-visible-run-state/issue-draft.md)。

## 原始需求记录

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。


## 复现与处理

真实作业 1313468 的 PH 已完成，run_phonon 将 source_dyn0 复制到同一路径，导致文件长度变为零；q2r 发生 Fortran EOF。save 副本及严格交付清单完整。运行时整阶段归档还会导致后处理重试重复 PH。

q2r 始终从清单中 dyn0 角色的归集副本物化输入。已归集且科学身份相容的 PH 清单须重验文件集合、哈希、原生 q 映射及 SCF/NSCF 数据；通过后保留现场、仅继续 q2r/matdyn。清单损坏拒绝执行，物理参数改变仍走阶段失效，不凭 JOB DONE 复用。

## 文档影响与关闭门槛

更新模块手册、指定算例 README、Overview 及本 Spec 验收；源码协议、安装与真实科学结果分别记录。完成 [任务](tasks.md) 与 [验收](acceptance.md) 后才能标记修复；不推送、不操作线上 Issue。
