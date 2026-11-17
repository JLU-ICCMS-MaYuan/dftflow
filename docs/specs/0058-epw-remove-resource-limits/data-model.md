# 数据变更

> 2026-11-17 追加需求已替代下文保留账本的设计，当前实现按本节执行；下文旧设计仅供历史追溯。

## 账本移除方案

删除配置、命令、存储模块与全部调用链，不新增包装器或替代登记系统。提交只持有现有运行锁，先持久化提交状态再调用 sbatch，回执中记录作业号；运行及恢复沿用 Slurm 身份和任务状态核验。结果交付直接读取科学产物，不查询账本。旧 `[budget]` 保持未知字段错误；旧账本和机器注册表留在原处但完全不访问，不自动修改带哈希的历史快照。

映射：FR-007／SC-008 对应配置与 CLI；FR-008／SC-006～007 对应工作区、原生任务和非编号执行链；FR-009 对应记录与交付；FR-010／SC-009 对应回执、防重复、失败恢复及完整进程回归。

## 当前配置

Resources 仅保留 partition、nodes、mpi。wall_seconds、storage_bytes 不再是合法配置字段，包括 Config::load_imported 的配置路径；错误必须带来源路径。

## 历史账本

保持账本版本 1。limits.node_seconds、limits.storage_bytes 可缺省；旧值若存在按原值保留，但不参与校验、预留或登记。新账本只写 limits.max_lanes。

预留和分配中的旧 node_seconds／storage_bytes 保留；新登记写零，表示未预估用量。实际 spent_node_seconds 仍按终态证据累计，结算幂等不变。legacy_source 完整保留。

## 状态接口

budget status 的有效 limits 仅含 max_lanes；历史实际消费与登记数值仍可查询，不提供时间／存储可用余额。结果包使用同一摘要接口，不额外执行额度检查。
