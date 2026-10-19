# epwflow

Rust 的 QE/EPW 工作流模块。当前功能与限制以 [Overview](../docs/overview/epwflow/README.md) 为入口；完整自动提交、原生两种 Eliashberg 与 Tc 尚未通过验收。

已有的配置检查和输入准备可使用：

```bash
epwflow check -c input.toml --json
epwflow prepare -c input.toml
```

对已完成的原生 EPW 目录，可只读检查，不启动科学程序或提交作业：

```bash
epwflow interpolation check -w /path/to/native-epw --json
```

若目录包含 `quality-reference.json`，同一命令检查独立 QE/PH 参考、指纹和电子/声子误差；质量超差输出诊断并非零退出。若目录只有原生 EPW 输入输出、退出码、wout 和谱，只能报告 `native_epc_only`，不会宣称能隙/Tc 或完整流程成功。格式见 [原生数据契约](../docs/specs/0012-epwflow-interpolation/contracts/interface.md)。

当前声子插值误差门为最大 **50 cm⁻¹**、RMS **2 cm⁻¹**（2026-10-19 用户调整）；报告打印实测值和采用上限，历史失败报告保留。元素／轨道投影按用户要求暂缓，总 DOS 与能带／声子叠图仍可使用。可靠电子投影还缺复原子投影算符及同一 Wannier 规范映射，可靠声子元素权重还缺逐原子本征矢适配，不能直接用键中心 Wannier 标签或模式序号代替。

MgB₂ 的 [真实测试手册](../test_auto/MgB2/epwflow/README.md) 区分旧 PBE 三模式冒烟和用户指定 Mori 讲义的新 LDA 输入。新输入已完成独立质量对照，但声子插值尚未满足严格阈值；没有越过该门生成超导结果。精确机器参数和原始输出留在忽略目录，未切换默认安装。

可使用更新后的受测构建入口生成比较诊断图。`DFTFLOW_BUILD_BIN` 指向本机平台构建目录，W 是包含 interpolation/comparison 的运行目录，R 是结果包：

```bash
"$DFTFLOW_BUILD_BIN/epwflow" interpolation export-comparison --source comparison-sources.json --output "$W/interpolation/comparison"
"$DFTFLOW_BUILD_BIN/epwflow" collect -w "$W"
"$DFTFLOW_BUILD_BIN/epwflow" plot --results "$W/results"
"$DFTFLOW_BUILD_BIN/epwflow" results check --results "$W/results" --json
"$DFTFLOW_BUILD_BIN/epwflow" plot --results "$R" --output "$D" --formats png,svg,pdf
```

导出命令只读取已准备的原生数据，当前原生适配限定 MgB₂／QE 7.4.1／EPW 5.9；来源 JSON 的完整字段见上述数据契约。后续三个命令不运行 QE／EPW／Slurm，不要求已有超导结果。默认生成四张独立叠图和 2×2 总览，各含 PNG／SVG／PDF：QE 为实线，EPW 为后绘制的虚线。比较表和图像清单分别保存来源哈希，结果包可复制后独立重绘。

当前真实适配只产生总 DOS；元素／轨道电子 PDOS 与元素声子 DOS 缺少严格投影时显示缺失面板，不能用总量代替。总览展示总量，元素／轨道分面位于独立 DOS 图。quality_failed 可以输出诊断图，但 workflow_complete=false；不能据图像生成成功继续细网格 EPC／超导。公开 submit/run/resume 科学链尚未接通，默认安装未更新。

声子叠图和总览用紫色标注直接 PH／EPW 的最大采样误差：主图突出路径内对应频率排序模式，附带区域列出整体采样最大；路径外 q 不冒充高对称点。标注包含晶体坐标、模式编号、两侧频率与误差。当前五点最大 49.45 cm⁻¹ 小于 50，但 RMS 8.08 cm⁻¹ 仍高于 2，质量门尚未通过。
