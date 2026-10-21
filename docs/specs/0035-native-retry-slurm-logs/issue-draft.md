# 0035：修复原生尝试错误冻结 Slurm 日志与重试归档入口

唯一类型：`type:bug`，本地编号已查重，线上 Gitee 由用户管理。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0029](../0029-active-slurm-log-archive/issue-draft.md)、[0033](../0033-wannier-velocity-handoff/issue-draft.md)。

## 原始需求

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。

## 已确认的错误

真实 1314095 的 coarse 原生尝试 inputs 含 slurm-1314095.out/err；退出后 err 追加 EPW 失败信息，哈希已不同。下一次恢复会把正常日志增长当作科学输入损坏。原生 attempt 自己的归档还会移走当前新作业日志，绕开 0029 工作区归档修复。

修复日志归类与两种归档路径，已记录的旧日志项只作为日志跳过输入一致性检查；已绑定科学文件照常严格校验，不自动修复损坏。不修改历史记录、冻结脚本、科学参数或质量门。

## 文档影响

更新 Spec、Overview、手册与本例验收。覆盖旧／新尝试日志追加、保持打开句柄的原生重试、科学数据损坏拒绝及真实粗网格恢复，保留预算和历史，范围自动提交、不推送。
