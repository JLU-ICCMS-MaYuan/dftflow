# scflow 超导后处理

入口：`scflow -c input.toml`；实现：`scflow/`。

- 输入：旧式 `*.dyn*.elph*` 或原生 `elph_dir/elph.inp_lambda.N`；可选 `*.dynN` 与 `split_ph.out.N / ph_no_split.out / *.ph.out`。
- 配置：`input.toml` 的 `[scflow]` 段，路径相对配置文件目录解析。
- 输出：direct/a2f CSV、`result.json`、Tc 曲线和文章视图 PDF，以及从 `ph.out` 提取的 CIF/VASP 结构。
- 运行边界：只做后处理，不调用 QE，不修改原始计算文件。

功能规格：[0001](../specs/0001-scflow-qe-superconductivity/spec.md)。

- 精确零频：只允许对 λ=0 的模按零贡献处理，并在 result.json 记录原始 γ；λ 非零则报错。
- 各展宽状态隔离；McMillan / Allen–Dynes 公式使用 float64 运算。
- 真实归档验收入口：`test_auto/MgB2/validate_scflow_isif4.py`，显式运行两组 μ* 的完整检查。
- Eliashberg 附 `resolved/resolution_limit` 搜索状态；ISIF4 的两套 μ* 共 20 份输出已通过完整链路测试，其余材料仍以验收记录为准。
