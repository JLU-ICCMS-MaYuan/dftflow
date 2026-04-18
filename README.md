# vaspflow

VASP calculation workflow automation scripts.

## Installation

```bash
pip install .
```

## Usage

Available commands:

- `vasp_scf`
- `vasp_eband`
- `vasp_eledos`
- `vasp_fermisurface`
- `vasp_cohp_uniform`
- `vasp_cohp_fatband`
- `vasp_lobsterin`
- `vasp_plot_cohp`
- `vasp_plot_kcohp`
- `high_throughput_substitute_structure`
- `high_throughput_substitution`
- `high_throughput_continue`

## High Throughput Substitution

- `high_throughput_substitute_structure` 从 `[high_throughput_substitution]` 读取 `XA/XB` 原子序号范围，生成 `<Z>.XA_<元素>/period_n/<Z>.<formula>` 目录树。
- 每个 `period_n/` 目录保存一份静态 `workflow_manifest.json`，只记录输入结构、`XA` 元素、压力列表和该组 case 清单。
- 每个实际体系目录保存一份 `case_status.json`，记录各压力点在 `vasp_opt / vasp_scf / vasp_eledos / vasp_process_eledos` 的 `finished / running / error / unstarted` 状态。
- 输出根目录保存一份汇总 `workflow_status.json`，供 `high_throughput_continue` 扫描、统计和续算使用。
