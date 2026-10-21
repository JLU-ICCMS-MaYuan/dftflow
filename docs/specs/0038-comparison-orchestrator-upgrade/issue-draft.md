# 0038：代码升级后复用已完成的原生比较子任务

唯一类型：`type:bug`，本地编号已查重，线上 Gitee 用户管理。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0034](../0034-reference-orchestrator-upgrade/issue-draft.md)。

## 原始需求

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。


## 已确认的问题

1314626 已完成比较 PW，matdyn 产物也完整；自动比较 request.json 却将可覆盖的 CLI 身份当成永久数据。修复 0037 并安装新程序后，旧摘要与验证将拒绝恢复，造成不必要的原生重算或阻塞。与 0034 同类，但该请求与独立质量参考是两个不同数据所有者。

## 处理范围与文档影响

编号工作区显式续算核对原始请求摘要、相同科学配置及除第七项原编排器外的全部来源。原 CLI 仅历史追溯，当前版本由冻结提交记录追溯；复用原摘要驱动子任务恢复，不改旧 request 和成功子记录。旧非编号工作区仍严格核对全部来源，不忽略科学程序、矩阵或物理数据。

更新模块手册、Overview、本例 README 和本目录验收；源码／安装／真实数值分别记录。复用原预算，不放宽质量门，不推送、不操作线上 Issue。
