# 技术依据与决策

- A2F 原入口读取 `*.dyn*.elph*`，优先从同名 `*.dyn*` 取得权重，失败时从 `*.ph.out` 读取，最后使用均匀权重。
- A2F 当前源码使用 `scipy.integrate.simps`；当前 Conda 环境提供 `simpson`，因此增加兼容函数而不改变积分公式。
- 当前 pymatgen 的 `CifWriter.write_file` 需要文本模式 `wt`；迁移层显式传入该模式。
- A2F 使用 NumPy `float16` 中间数组，现代 NumPy 下参与物理量计算会下溢；写结果前将关键物理量提升为 Python/float64 数值。
- 依赖放入 `scflow` extra，避免基础 dftflow 安装强制引入绘图和科学计算依赖。
