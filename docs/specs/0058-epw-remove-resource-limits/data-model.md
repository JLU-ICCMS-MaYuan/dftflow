# 数据变更

## 当前配置

Resources 仅保留 partition、nodes、mpi。wall_seconds、storage_bytes 不再是合法配置字段，包括 Config::load_imported 的配置路径；错误必须带来源路径。

## 历史账本

保持账本版本 1。limits.node_seconds、limits.storage_bytes 可缺省；旧值若存在按原值保留，但不参与校验、预留或登记。新账本只写 limits.max_lanes。

预留和分配中的旧 node_seconds／storage_bytes 保留；新登记写零，表示未预估用量。实际 spent_node_seconds 仍按终态证据累计，结算幂等不变。legacy_source 完整保留。

## 状态接口

budget status 的有效 limits 仅含 max_lanes；历史实际消费与登记数值仍可查询，不提供时间／存储可用余额。结果包使用同一摘要接口，不额外执行额度检查。
