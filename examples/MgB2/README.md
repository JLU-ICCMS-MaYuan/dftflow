# MgB₂ 分机器测试

## 目录约定

每个体系按 `examples/<体系>/<机器>/` 管理手册；本体系包含 `sugon-amd/`、`inspur/`、`chem/`。运行输入、脚本、日志和结果统一放到各机器的 `run/`，不进入 Git。机器路径保存在 `docs/local/MgB2-<机器>.env`，不进入 Git。

## 当前状态

- [sugon-amd](sugon-amd/README.md)：QE 7.4.1、AMD Slurm 单节点声子流程。
- [inspur](inspur/README.md)：配置模板，等待实际机器信息。
- [chem](chem/README.md)：配置模板，等待实际机器信息。

## 计算范围

SCF → DFPT 声子网格 → q2r → matdyn 声子谱与 DOS → DFTFlow 表格和绘图。使用仓库 MgB₂ 原胞和小网格验证流程，不包含结构优化、电子声子耦合或 Tc；不代表数值已经收敛。公共准备脚本使用 Bash，沿用项目构建和计算脚本的语言例外。
