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
- `substitution_prepare_qe_pseudo`
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
- `substitution_structure` 是高通量替换工作流的 prepare 步骤：从 `[substitutionflow]` 读取 `XA/XB` 原子序号范围，生成 `<Z>.XA_<元素>/period_n/<Z>.<formula>` 目录树，并同时写出替换结构、case `input.toml` 与初始状态文件。
- 每个 `period_n/` 目录保存一份静态 `workflow_manifest.json`，只记录输入结构、`XA` 元素、压力列表和该组 case 清单。
- 每个 case 下生成的 `input.toml` 是后续 `vasp_*`、`qe_phonon_*` 和 `substitution_continue` 的统一配置真源；它默认完整继承根 `input_substitution.toml` 中的共享块和子工作流参数块，只对 `chgcar_path`、`wavecar_path` 做 case 级相对路径重写。
- 如果根配置中包含 QE 声子相关设置，`substitution_structure` 会原样带入每个 case `input.toml`，包括 `pp_dir`、`[qe]`、`[qe_phonon]` 及其子块。
- 每个实际体系目录保存一份 `case_status.json`，记录各压力点在 `mattersim_opt / vasp_opt / vasp_scf / vasp_eledos / vasp_process_eledos / qe_phonon` 的 `finished / error / unstarted` 状态。
- 输出根目录保存一份汇总 `workflow_status.json`，供 `substitution_continue` 扫描、统计和续算使用；旧版不含 `qe_phonon` 的状态文件会在下一次刷新时自动补齐或重写，不需要手工删除。
- `[substitutionflow].submit_groups` 控制 `-s/--submit` 时把未完成任务分成多少组；一个 `case + pressure` 视为一个不可拆分的完整提交单元，组内保持当前遍历顺序，`substitution` 和 `substitution_continue` 会分别对 `submit_group_01.sh`、`submit_group_02.sh` 等脚本逐组执行 `sbatch`。
- `[substitutionflow].phonon_mode` 控制末尾 QE 声子总工作流，允许值只有 `nosplit / split_index / split_coords`，默认 `split_index`；建议在根配置里显式写出该字段，避免依赖默认值。对应执行 `qe_phonon_* -i vasp_opt/CONTCAR -c ../input.toml -r`。
- `qe_phonon` 的整体状态不再靠目录残留猜测，而是读取对应 `qe_phonon_* / workflow_status.json` 归约得到：`nosplit` 以 `scf -> ph -> q2r -> matdyn_phband -> matdyn_phdos` 为准，`split_index / split_coords` 以 `scf -> dyn0 -> split tasks -> merge -> q2r -> matdyn_phband -> matdyn_phdos` 为准；只有相关阶段全部 `finished` 才会把 `qe_phonon` 判成 `finished`。
- `substitution_continue -rws` 的语义固定为“先真实刷新，再把全部 `error` 重置为 `unstarted`，并修复已有 `qe_phonon_*` 残留目录的基础输入”。它会用 case 根 `input.toml` 覆盖子目录 `input.toml`，并用当前 pressure 下 `vasp_opt/CONTCAR` 覆盖子目录 `POSCAR`；不会提前生成 `run_scf.sh / ph.in / q2r.in` 等中间文件。
- `substitution` 负责从头生成并启动整条工作流，`substitution_continue` 负责基于最新状态从最后一个已完成步骤之后续算。状态刷新不再依赖 Slurm 或本地进程探测；只有 `finished` 会跳过，`error` 会阻断，`unstarted` 会继续推进。执行过 `substitution_continue -rws` 后，再运行 `substitution_continue -r` 会从新的第一个非 `finished` 步骤继续推进。
- `substitution_prepare_potcar` 从 `[substitutionflow]` 推导需要的元素集合，按 `--pot` 指定的原始赝势库筛选并复制所需 `POTCAR` 到当前目录的 `potcar_lib/`，默认读取已有 `potcar_lib/potcar_lib.log` 续选，并在每次完成单个元素选择后立即更新该记录文件。
- `substitution_prepare_qe_pseudo` 从 `[substitutionflow]` 推导需要的元素集合，按 `--pp` 指定的原始 QE 赝势目录筛选并复制所需文件到当前目录的 `qe_pseudo_lib/`。它不会再回写各 case `input.toml`；推荐在根配置中直接写 `pp_dir = "qe_pseudo_lib"`，并让 `substitution_structure` 原样继承。
- `substitution_plot_eledos` 是独立的批量汇总绘图 CLI；省略 `-d` 时默认扫描当前目录下全部 `[Z].XA_*` 目录，也支持传入一个批量根目录或一个/多个指定的 `[Z].XA_*` 目录，并在每个 XA 目录下按 pressure 输出 `eledos_summary_<pressure>.png`，每个格子固定绘制 `XA / XB / H / TDOS` 四柱。

示例 `input_substitution.toml` 片段：

```toml
pp_dir = "qe_pseudo_lib"

[qe]
executable_path = "/opt/qe/bin"

[qe_phonon]
prefix = "ht_ph"

[qe_phonon.scf]
ecutwfc = 80.0
ecutrho = 640.0
outdir = "./qe_out"
kpoints = [3, 3, 2]

[qe_phonon.ph]
qpoints = [2, 2, 1]
recover = true

[substitutionflow]
XA = "3,4"
XB = "3-5"
pressures = [20, 50]
structure_filename = "XAXBH7-P63mmc.vasp"
phonon_mode = "split_index"
```
