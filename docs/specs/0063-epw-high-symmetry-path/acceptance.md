# 0063 实施与验收记录

日期：2026-10-27。用户已明确开始执行，并确认只在相同坐标计算能量差、保留 DOS、不提示共同点覆盖率。

## 已实现

- 第 5 步从现有高对称端点文件生成真实路径边，MgB₂ 9 条连线对应 9 条路径声明；保留标签和断段，移除人工校准、逐点及网格覆盖段。
- 保留用户 bands_num_points 设置，取消旧的固定为 1 的约束；检查空路径、奇数端点、非有限坐标、非法标签、零长度边、行长及原生短段零间隔。
- 从原生 kpt、band、labelinfo 核验实际点数、端点与逐段坐标，以真实倒空间距离保存各自完整曲线。差值只计算双方原生共同坐标，没有共同点不生成最大差，不显示覆盖率。
- DOS 沿用同一拟合模型的 postw90 均匀网格，不把网格写入路径。第 6 步使用带哈希的第 5 步电子表格、统计和采样，声子保持原协议。
- 旧包缺少新标记时保持原有同点位验证；旧拟合、输入和回执不自动改写。Overview 与 MgB₂ 测试说明同步。

## 隔离验证

在基线 `7dfedb60ed338c8f223305ddfa9f81ed5b511f9c` 的临时归档中只应用本任务改动，目录为 `/tmp/dftflow-0063-isolated`。构建沿用本机原有原生依赖环境，未发布安装。

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow --lib --test comparison_plots --test native_stage_config
```

结果：库测试 51 通过、3 跳过；绘图测试 10 通过；配置测试 13 通过。日志：`/tmp/dftflow-0063-isolated-core-retry.log`。

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow --test three_stage_execution stage_results:: -- --test-threads=8 --nocapture
```

结果：7 通过。覆盖第 5 步交付、不同原生点数及断段、第 6 步电子 CSV 原样复用、DOS 单次调用、结果包检查、旧协议迁移、损坏坐标拒绝及后处理恢复。日志：`/tmp/dftflow-0063-isolated-stage.log`。

另用同一已编译测试程序执行 `decay::decay_is_published_before_comparison_failure_and_repairs_without_rerunning_science`，1 通过。日志：`/tmp/dftflow-0063-isolated-recovery.log`。合计 82 通过；3 个依赖显式真实数据的库测试未执行，不计为通过。

混合工作树的较大进程套件曾出现阶段恢复失败，未把该次未完成运行记为通过；同名恢复用例在本任务隔离树通过。隔离首轮构建因未带入本机 libpng 的 pkg-config 路径失败，带入原有本机构建环境后重新验证通过，没有为此修改依赖或系统配置。

## 需求证据

| 标准 | 证据 |
| --- | --- |
| SC-001 | MgB₂ 九段输入单测逐行检查真实端点及数量；真实生成入口的三段受控用例无 P0／Q0 |
| SC-002 | 两种密度路径块相同；重复 Γ、断段、非正交倒格矢及不等长路径测试 |
| SC-003 | 共同坐标统计和无共同点单测；实际原生点序／坐标错误拒绝；公开入口保留不同点数 |
| SC-004 | 阶段测试核对能带、DOS、postw90 单次调用和第 6 步 CSV 哈希复用 |
| SC-005 | 非法输入拒绝；旧来源、成功科学回执和模型哈希保护；损坏原生坐标拒绝 |
| SC-006 | 上述隔离回归及本目录、Overview、MgB₂ 说明的链接和原话一致性检查 |

## 文档影响与边界

根 README 的公开入口与导航没有变化，无需修改。采用原有 Rust/Bash 生产链及 Python 进程协议替身，不新增依赖或第二套求值器；路径声明、原生采样与比较发布分别保留既有职责。

本轮没有执行真实 QE／EPW／Wannier／postw90、提交 Slurm、修改用户科学现场、切换安装、推送或操作线上 Issue。测试替身不证明真实拟合质量；图像和数值仍供用户人工判断。线上 Gitee 及远程链接仍由用户管理；图片原件仅聊天预览，PDF 核查记录保留该限制。
