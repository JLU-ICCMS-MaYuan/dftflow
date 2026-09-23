# 验收记录

## 已完成

- 当前 Conda 环境下 `test_scflow.py` 的配置、积分兼容和公开 `La2H20` QE EPC 归档测试通过。
- `result.json` 包含 Eliashberg、McMillan 和 Allen-Dynes Tc；direct/a2f CSV 已生成。
- 原始 `external/a2f` 子模块未修改。

## 2026-09-23：SiP2 真实归档验收

用户提供的 Windows 路径对应 `/mnt/e/4.my_coorperation/2026.3_SiP2_Gouhuiyang`。
对照对象固定为 DFTFlow `03c9994`、原版 A2F `22ece668` 和本地 QE 7.4.1 `lambda.x`。
仅做已有输出的后处理，没有重跑 SCF/phonon/EPC，没有修改程序源码或原始文件。

报告：`/mnt/e/4.my_coorperation/2026.3_SiP2_Gouhuiyang/docs/scflow-comparison-2026-09-23/README.md`。
数据项目对照报告提交：`4d41b944edc558d4891321131c5249bb45867e81`。远端 Issue 已回写评论 `51335842`，保持开放。

同目录 `comparison.csv`、`validation.json`、`manifest.json`、`parity.json` 和运行日志记录数值、哈希与命令。

- 覆盖 10 组 EPC 归档 × 10 个电子展宽，统一 μ*=0.10，并另外保留现存 lambda.in 的 μ* 重跑 QE。
- ISIF2、ISIF3 缺 lambda.in，但各有完整 16 个 EPC 与动力学矩阵；本次生成独立后处理输入并实际运行。仅 SC-07（33.6 GPa）无完整 EPC 数据。
- 324 条来源记录的原始文件和复制件 SHA-256 均通过校验。
- 原样包含 ph.out 的 CLI 未通过；省略不可解析的可选结构日志后，Pa-3 三组及 Pbam 未优化、ISIF2、ISIF3 的 CLI 完成。后三组 Pbam 有虚频，数值不作为可靠物理 Tc。
- 原版 A2F 的计算核心使用进程内 SciPy simps→simpson 兼容映射，不修改其源码。197 组配对记录中 102 组成功，记录数值最大差为 0；包括 60 组原始核心计算及 42 组另行标注的诊断，不能把诊断结果冒充原始 CLI 验收。
- ISIF4：QE μ*=0.10 得 8.343–11.113 K；现存输入 μ*=0.16 得 4.831–7.131 K。当前 scflow 原始零频输入失败。
- 21.3 GPa：除零频外，round(...,5) 坐标比较漏掉 4/16 个 q 点；现存 μ*=0.13 输入与归档输出来源不一致，归档输出符合 μ*=0.10。
- Pnma 原始 scflow 失败；QE Tc 公式分母非正/ωlog 非有限，不将星号或 NaN 当 Tc=0。
- Pa-3 的 A2F Eliashberg 0.001 K 是源码强制下限，不是已验证的物理 Tc。

## 未通过项

SC-004 仅部分通过，**不得关闭 Issue**。需要后续独立修复并明确科学处理规则：

1. 可选 ph.out 缺结构字段时的处理。
2. 零频、无耦合及 γ 非有限值处理。
3. q 点容差匹配和权重完整性校验。
4. 虚频、公式适用范围、Eliashberg 下限和低精度舍入的结果标记。

本次对照和报告任务已完成，不宣称上述程序缺陷已修复。
