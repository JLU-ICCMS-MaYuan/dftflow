# 验证方法

在独立工作树和独立构建缓存执行 release 回归，避免共享调试程序被其他构建替换。先运行电子确认作用域的单元测试及公开 CLI 集中复用测试，再运行 three_stage_execution 中电子、PDOS、源重算与派生图修复相关回归。

构建使用 scripts/build-native.sh build --release -p epwflow --bins --locked，成功后覆盖固定入口；不提交真实 QE／EPW 或 Slurm 作业。
