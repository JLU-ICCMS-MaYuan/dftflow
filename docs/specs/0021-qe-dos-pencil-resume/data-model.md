# 数据与状态

复用 schema_version=1 的 Attempt 及 version=1 的 NativeReceipt，无字段改动。新公开提交创建全新 id、冻结输入、reservation 和 job。旧尝试、旧作业及收据只读保留；bands／PW 的输入、产物与上游仍严格验证。DOS 的 args 更新自然改变其本任务键，不改变 PW 或 bands 键。作业冲突提示只改变错误文字。
