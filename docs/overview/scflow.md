# scflow 超导后处理

入口：`scflow -c input.toml`；实现：`scflow/`。

- 输入：QE EPC 输出目录中的 `*.dyn*.elph*`，以及可选的 `*.dyn*` 和 `*.ph.out`。
- 配置：`input.toml` 的 `[scflow]` 段，路径相对配置文件目录解析。
- 输出：direct/a2f CSV、`result.json`、Tc 曲线和文章视图 PDF，以及从 `ph.out` 提取的 CIF/VASP 结构。
- 运行边界：只做后处理，不调用 QE，不修改原始计算文件。

功能规格：[IKHZT0](../specs/IKHZT0-scflow-qe-superconductivity/spec.md)。
