# 配置与输出契约

## 配置

- 入口：`scflow -c <input.toml>`。
- 配置段：`[scflow]`。
- `work_dir`：QE EPC 输出目录，默认是配置文件目录。
- `results_dir`：结果目录，默认是 `<work_dir>/results`。
- 数值参数：`smoothing_thz`、`resolution`、`gaussian_sigma`、`mu`、`structure_tolerance`。
- `integration` 只能是 `sum` 或 `simps`；`resolution = 0` 沿用 A2F 的自动分辨率逻辑。

## 输入

工作目录至少需要一组旧式 `*.dyn*.elph*` 或原生 `elph_dir/elph.inp_lambda.N`。两组同时存在则报错。动力学矩阵只识别 `*.dynN`，结构日志识别 `*.ph.out`、`ph_no_split.out` 和 `split_ph.out.N`。

## 输出

结果目录保留 A2F 原有 direct/a2f CSV、`result.json`、PDF、summary、CIF 和 VASP 结构命名语义。`result.json` 中的 `Tc, K` 包含 McMillan、Allen-Dynes 和 Eliashberg 结果。

新增 `zero_frequency_handling` 保存精确零模处理规则及原始数据定位；新增 `eliashberg_solver` 保存状态、0.001 K 分辨率/下限与矩阵大小。搜索下限保留旧 Tc 字段但明确标记 `resolution_limit`。
