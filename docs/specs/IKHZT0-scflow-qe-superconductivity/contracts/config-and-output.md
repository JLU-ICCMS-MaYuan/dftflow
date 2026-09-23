# 配置与输出契约

## 配置

- 入口：`scflow -c <input.toml>`。
- 配置段：`[scflow]`。
- `work_dir`：QE EPC 输出目录，默认是配置文件目录。
- `results_dir`：结果目录，默认是 `<work_dir>/results`。
- 数值参数：`smoothing_thz`、`resolution`、`gaussian_sigma`、`mu`、`structure_tolerance`。
- `integration` 只能是 `sum` 或 `simps`；`resolution = 0` 沿用 A2F 的自动分辨率逻辑。

## 输入

工作目录至少需要一个 `*.dyn*.elph*` 文件；`*.dyn*` 和 `*.ph.out` 为可选输入。

## 输出

结果目录保留 A2F 原有 direct/a2f CSV、`result.json`、PDF、summary、CIF 和 VASP 结构命名语义。`result.json` 中的 `Tc, K` 包含 McMillan、Allen-Dynes 和 Eliashberg 结果。
