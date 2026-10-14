# 合成 α²F 参考材料

## 来源与数据范围

从原 `qeflow/examples/` 迁入 `a2F.dat` 与 `[scflow]` 配置。数据是合成曲线，三列分别为频率、a2f_lambda、a2f_gamma，单位约定见文件注释；不代表 Si 或 MgB₂ 的实际计算。配置移除了与该数据无关的 `[qe]` 机器路径示意。

## 当前限制

这是早期单曲线演示的参考材料，当前 CLI 的 `scflow -c input.toml` 和 `qeflow scflow -c input.toml` 均进入完整 QE EPC 流程，需要动力学矩阵及电子声子输出。只有本目录的 `a2F.dat` 时会报告 `No QE EPC files found`，因此本目录不作为可运行的当前 CLI 算例。

本次整理仅迁移展示资产，不修改 scflow 的计算入口或实现。若需验证完整 scflow，请参阅 [scflow 手册](../../../scflow/README.md) 和仓库测试体系。
