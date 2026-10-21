# 0036：区分 EPW 波函数工作副本与不可变科学来源

唯一类型：`type:bug`，本地编号已查重，线上 Gitee 用户管理。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0033](../0033-wannier-velocity-handoff/issue-draft.md)、[0035](../0035-native-retry-slurm-logs/issue-draft.md)。

## 原始需求

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。

## 已确认的故障

逐项比对 1314095 的全部原生输入，除了正常增长的日志外，EPW 改写了本阶段 out/MgB2.wfc1～32。原始 NSCF、portable SAVE 与 PH 响应未变。旧原生尝试把可写工作副本作为永久不可变输入，在成功归集或失败重试时均会误报损坏。

只从已冻结且校验通过的 coarse 原生输入识别 prefix 和 ./out，允许该目录下 prefix.wfc正整数 的工作副本被原生程序改写；保留初始身份用于追溯，不改旧记录。原来源和 portable 波函数／响应／原生输入继续严格核验，失败副本不复用，而是归档并从已校验 F2 重新复制；成功 EPC 必须经过原生格式与产物验证。

## 文档影响

更新 Spec、Overview、模块与算例说明；覆盖原生改写、旧记录恢复、其他前缀／路径、portable SAVE 与 PH 篡改拒绝，并记录实际粗网格结果、源哈希与预算。验证后范围提交，不推送。
