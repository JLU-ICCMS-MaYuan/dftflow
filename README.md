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
- `mattersim_opt`
- `substitution_structure`
- `substitution`
- `substitution_continue`
- `substitution_prepare_potcar`
- `substitution_plot_eledos`
- `qe_scf`
- `qe_nscf`
- `qe_eband`
- `qe_eledos`
- `qe_process_eledos`
- `qe_process_eband`
- `qe_ph_nosplit`
- `qe_ph_split_assign_index`
- `qe_ph_split_assign_coords`
- `qe_q2r`
- `qe_matdyn_phband`
- `qe_matdyn_phdos`
- `qe_check`
- `qe_phonon_nosplit`
- `qe_phonon_split_index`
- `qe_phonon_split_coords`

## High Throughput Substitution

- `mattersim_opt` 通过外部 MatterSim Python 环境运行两阶段预优化：先只优化原子坐标，再在目标压力下联合优化晶格和原子坐标，输出 `mattersim_opt/CONTCAR` 供后续 `vasp_opt` 使用。
- `substitution_structure` 从 `[substitutionflow]` 读取 `XA/XB` 原子序号范围，生成 `<Z>.XA_<元素>/period_n/<Z>.<formula>` 目录树。
- 每个 `period_n/` 目录保存一份静态 `workflow_manifest.json`，只记录输入结构、`XA` 元素、压力列表和该组 case 清单。
- 每个实际体系目录保存一份 `case_status.json`，记录各压力点在 `mattersim_opt / vasp_opt / vasp_scf / vasp_eledos / vasp_process_eledos / qe_phonon` 的 `finished / running / error / unstarted` 状态。
- 输出根目录保存一份汇总 `workflow_status.json`，供 `substitution_continue` 扫描、统计和续算使用；旧版不含 `qe_phonon` 的状态文件会在下一次刷新时自动补齐或重写，不需要手工删除。
- `[substitutionflow].submit_groups` 控制 `-s/--submit` 时把未完成任务分成多少组；一个 `case + pressure` 视为一个不可拆分的完整提交单元，组内保持当前遍历顺序，`substitution` 和 `substitution_continue` 会分别对 `submit_group_01.sh`、`submit_group_02.sh` 等脚本逐组执行 `sbatch`。
- `[substitutionflow].phonon_mode` 控制末尾 QE 声子总工作流，允许值只有 `nosplit / split_index / split_coords`，默认 `split_index`；对应执行 `qe_phonon_* -i vasp_opt/CONTCAR -c ../input.toml -r`。
- `substitution` 负责从头生成并启动整条工作流，`substitution_continue` 负责基于最新状态从最后一个已完成步骤之后续算；如果某个压强点当前目标步骤处于 `running`，则会跳过不重复启动。
- `substitution_prepare_potcar` 从 `[substitutionflow]` 推导需要的元素集合，按 `--pot` 指定的原始赝势库筛选并复制所需 `POTCAR` 到当前目录的 `potcar_lib/`，默认读取已有 `potcar_lib/potcar_lib.log` 续选，并在每次完成单个元素选择后立即更新该记录文件。
- `substitution_plot_eledos` 是独立的批量汇总绘图 CLI；省略 `-d` 时默认扫描当前目录下全部 `[Z].XA_*` 目录，也支持传入一个批量根目录或一个/多个指定的 `[Z].XA_*` 目录，并在每个 XA 目录下按 pressure 输出 `eledos_summary_<pressure>.png`，每个格子固定绘制 `XA / XB / H / TDOS` 四柱。
