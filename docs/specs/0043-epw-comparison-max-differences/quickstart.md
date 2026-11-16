# 验证说明

构建及回归：`bash "scripts/build-native.sh" test -p epwflow --offline -- --test-threads=4`（使用本机已有原生依赖配置）。测试使用 Rust 与隔离原生程序夹具，不提交 Slurm 科学作业，不切换安装。

输入使用新配置，删除 interpolation.quality 小节。三个公共程序及常规出图入口保持。新比较包 metrics.json 提供四项最大差值，图和命令同时显示最大值与位置；不含质量阈值或评级。已有旧科学目录不自动迁移，不手动删除其中的 quality-sources 来冒充完整升级。

数据损坏仍需返回非零；仅改变数值差值不能导致阻断。移动结果包并在不含 QE／EPW 的 PATH 下重绘，确认三格式和总览均可生成。

## 2026-11-16 独立采样验证

运行 comparison_plots、native_stage_config 及库测试；检查不同密度保留原始点数、只统计共同点、非均匀内部点合法、错误端点及偏离路径仍拒绝。使用 epw_wannier --plot --results <结果目录> 直接重绘，核对三格式无紫色误差点及图例，不启动科学程序。

## 已有 215 点缓存入口

回归使用 `three_stage_execution` 中 `stage_results::completed_matdyn_keeps_215_native_points_when_current_qe_plan_is_935` 和 `stage_results::legacy_electronic_native_points_are_not_replaced_by_current_density`。测试名需包含模块前缀，避免 `--exact` 选中零项。

本机真实只读验证：设置 `DFTFLOW_REAL_COMPARISON_INPUT` 为 Ubuntu 算例输入，显式执行库测试 `interpolation::native_sampling::tests::actual_cached_matdyn_uses_its_own_request_instead_of_current_density -- --exact --ignored --nocapture`。仅读取原生请求、XML 和频率，不提交科学作业；其他机器没有该现场时保持忽略。

修改后通过发布构建覆盖固定命令。`epw_wannier -c "input.toml" --stage coarse` 仅准备，实际提交仍由用户加 `-s` 执行。
