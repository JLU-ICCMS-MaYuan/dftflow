# 初始调查

指定 epwinput.toml 的 structure=POSCAR 解析到不存在的 epwflow/POSCAR，已由已安装 epw_phonon --check 实际复现。work_dir=. 正确；outdir 仍指旧隐藏目录，改为可见 scratch。KPATH 已在本目录，赝势绝对路径存在。README 当前引导上一级 run，与本次明确位置不符。

HEAD=5810e00 已包含 0025，但三份安装程序来自此前 0023 构建；需要重新构建部署。此前 0023 的未提交源码和文档均保留，不能混入本项提交。指定目录只有旧 generated/preparation-history，没有 layout、active 或编号 QE 成功目录；旧源码生成准备记录不是本次编号计算结果。

启动探针 1313411 在 node14 COMPLETED/0:0，脚本实际输出主机、uid 和 shared_workspace_access=ok；节点可读指定结构、可写工作目录。只出现 locale 警告，未影响退出码。预算实际记账 0 节点秒，已 reconcile；后续通过测试专用 sbatch 转发 --nodelist=node14，不改变根物理参数或 32 MPI。
旧 .epwflow 仅 16 份准备文件，无 layout/active/attempts/控制或执行锁，不是可迁移编号计算现场；按完整 SHA256 清单保留到本目录 historical-inputs/legacy-preparation-20261021，不自动清理未知锁，不删除原始内容。

## 首次真实链路

1313468：SCF、完整 NSCF、能带／DOS 参考及 PH 完成；q2r 失败，根因为 dyn0 自复制截断，另建 0028。516 文件身份已保存，归集清单零哈希差异。Slurm 日志被归档另建 0029。

1313621：使用公开 epw_wannier --stage wannier -s，复用前三阶段，EPW 正常退出，但 500 次局域化未收敛，被现有检查正确拒绝；解纠缠已收敛。此项属于迭代预算不足，不作代码缺陷。只将 wannier.localization.max_iterations 从 500 增为 5000，容差 1e-8 Å²、5 步窗口、物理网格／能窗／投影及质量门不变；不改外部科学程序。
