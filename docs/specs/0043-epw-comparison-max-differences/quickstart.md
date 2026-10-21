# 验证说明

构建及回归：`bash "scripts/build-native.sh" test -p epwflow --offline -- --test-threads=4`（使用本机已有原生依赖配置）。测试使用 Rust 与隔离原生程序夹具，不提交 Slurm 科学作业，不切换安装。

输入使用新配置，删除 interpolation.quality 小节。三个公共程序及常规出图入口保持。新比较包 metrics.json 提供四项最大差值，图和命令同时显示最大值与位置；不含质量阈值或评级。已有旧科学目录不自动迁移，不手动删除其中的 quality-sources 来冒充完整升级。

数据损坏仍需返回非零；仅改变数值差值不能导致阻断。移动结果包并在不含 QE／EPW 的 PATH 下重绘，确认三格式和总览均可生成。
