# 0053：日志数据契约

## 原生 Task

保持原 JSON 字段，新增可读写的 `schema_version=3`。

| 版本 | stdout／stderr 的路径 | 规则 |
| --- | --- | --- |
| 1 | `native-ID-JOB.out`／`native-ID-JOB.err` | 原路径、原验证及归集副本保持 |
| 2 | `stem.out`／`stem.err` | 必须保留两份独立日志 |
| 3 | 两身份均指向 `stem.out` | 捕获一次并复用身份，无 `.err` 依赖 |

状态、任务键、输入来源、预算、脚本身份和显式恢复规则不改变。未知版本拒绝。旧任务继续使用其记录版本，未开始的旧 prepared 任务也不静默改写。

## ExecutionReport 与科学产物

`stdout_path`／`stderr_path` 类型保持；相同路径表示合并日志。归集按不同路径返回 `.out`／`.err` 目标，新版本只登记 `.out`。旧版本及普通本地执行器仍归集两份。方法批次原有可选 `.err` 快照用于兼容，不要求新建该文件。
