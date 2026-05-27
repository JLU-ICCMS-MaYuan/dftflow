# QEFLOW 使用说明

- CLI 总览流程图见 [qeflow-cli-flow.mmd](/home/mayuan/code/dftflow/qeflow/qeflow-cli-flow.mmd)。
- 分模块流程图：
  - [scf/scf-cli-flow.mmd](/home/mayuan/code/dftflow/qeflow/scf/scf-cli-flow.mmd)
  - [nscf/nscf-cli-flow.mmd](/home/mayuan/code/dftflow/qeflow/nscf/nscf-cli-flow.mmd)
  - [eband/eband-cli-flow.mmd](/home/mayuan/code/dftflow/qeflow/eband/eband-cli-flow.mmd)
  - [eledos/eledos-cli-flow.mmd](/home/mayuan/code/dftflow/qeflow/eledos/eledos-cli-flow.mmd)
  - [phonon/phonon-cli-flow.mmd](/home/mayuan/code/dftflow/qeflow/phonon/phonon-cli-flow.mmd)
  - [qe_check/qe-check-cli-flow.mmd](/home/mayuan/code/dftflow/qeflow/qe_check/qe-check-cli-flow.mmd)
  - [qe_input/qe-input-flow.mmd](/home/mayuan/code/dftflow/qeflow/qe_input/qe-input-flow.mmd)

## 1. `executable_path` 语义
- `[qe].executable_path` 填写的是“启动命令 + QE bin 目录”，不是某个单独的 `*.x` 可执行文件。
- 典型写法：

```toml
[qe]
executable_path = "mpirun -np 8 /home/mayuan/soft/qe-7.4.1/bin"
```

- 各 CLI 会自动拼接所需程序：
  - `qe_scf` / `qe_nscf` / `qe_eband` 的 `pw.x`
  - `qe_q2r` 的 `q2r.x`
  - `qe_ph_nosplit` / `qe_phonon_split_*` 的 `ph.x`
  - `qe_matdyn_phband` / `qe_matdyn_phdos` 的 `matdyn.x`
  - `qe_process_phonon` 复用 `q2r.x` 与 `matdyn.x`，并在 Python 内完成归档、CSV 整理和绘图

## 2. 模板与覆盖
- 默认模板定义在 [templates.py](/home/mayuan/code/dftflow/qeflow/templates.py)。
- 递归覆盖逻辑定义在 [config_templates.py](/home/mayuan/code/dftflow/public/config_templates.py)。
- `qe_phonon` 相关新 CLI 继续共用同一个 `[qe_phonon]` 配置块，不为每个命令单独拆 TOML 节。
- `qe_scf`、`qe_nscf`、`qe_eband`、`qe_eledos` 统一通过 [qe_input/calculation_setup.py](/home/mayuan/code/dftflow/qeflow/qe_input/calculation_setup.py) 的 **QE Calculation Setup** 准备目录；各 CLI 只保留命令行 Adapter。
- POSCAR 解析、化学式和 kmesh 派生集中在 [qe_input/structure.py](/home/mayuan/code/dftflow/qeflow/qe_input/structure.py)，避免各计算类型重复维护结构读取规则。
- `[qe].lattice_mode` 控制 `pw.x` 输入里的晶格写法，默认 `ibrav_angstrom`。允许值：
  - `ibrav_angstrom`：自动调用 QE `cell2ibrav.x` 判断 `ibrav`，写出 `ibrav + A/B/C/cos*`，不写 `CELL_PARAMETERS`。
  - `ibrav_celldm`：自动判断 `ibrav`，写出 `ibrav + celldm(n)`。
  - `cell_parameters`：兼容旧行为，写出 `ibrav = 0 + CELL_PARAMETERS {angstrom}`。
- 不允许在 `input.toml` 手动填写 `ibrav`、`A/B/C`、`cosAB/cosAC/cosBC` 或 `celldm(1..6)`；这些参数统一由结构文件自动生成。若 `[qe].executable_path` 指向的 QE bin 目录中找不到或无法运行 `cell2ibrav.x`，程序会使用内置 Python fallback。

## 3. 声子 CLI 体系
### 3.1 独立单步 CLI
- `qe_ph_nosplit`
- `qe_merge`
- `qe_q2r`
- `qe_matdyn_phband`
- `qe_matdyn_phdos`
- `qe_process_phonon`
- `qe_split_submit`
- `qe_check`

### 3.2 可恢复总工作流 CLI
- `qe_phonon_nosplit`
- `qe_phonon_split_index`
- `qe_phonon_split_coords`

## 4. `work_dir` 规则
- 用户始终在“当前目录”执行命令。
- 如果显式设置 `[qe_phonon].work_dir`，三个总工作流都会统一使用该目录。
- 如果没有显式设置 `[qe_phonon].work_dir`，程序按当前总工作流命令自动选择默认目录：
  - `qe_phonon_nosplit` -> `qe_phonon_nosplit`
  - `qe_phonon_split_coords` -> `qe_phonon_split_coords`
  - `qe_phonon_split_index` -> `qe_phonon_split_index`
- 如果当前目录 basename 等于目标 `work_dir`，说明用户已经进入真实计算目录，程序直接在当前目录检查和续算。
- 如果当前目录 basename 不等于目标 `work_dir`，程序会进入或创建 `./<work_dir>` 作为真实计算目录。
- 单步 CLI 仍沿用共享声子目录语义；这里只有 3 个总工作流命令的默认目录发生拆分。

## 5. 目录结构
`work_dir` 内部固定使用以下语义：

```text
qe_phonon_nosplit/
  scf.in
  scf.out
  ph.in
  ph.out
  q2r.in
  q2r.out
  matdyn_phband.in
  matdyn_phband.out
  matdyn_phdos.in
  matdyn_phdos.out
  phband/
    Mg1B2.phband.freq
    Mg1B2_band.labelinfo.dat
    phonon_band.csv
    phonon_band_labels.csv
    phonon_band.png
  phdos/
    Mg1B2.phdos.dat
    phonon_dos_atoms.csv
    phonon_dos_elements.csv
    phonon_dos.png
  workflow_status.json
  Mg1B2.dyn0
  q_index_map.json
  01/
    scf.in
    split_ph.in
    run_split.sh
    task_status.json
  02/
  ...
```

- split 子目录固定使用 `01`、`02`、`03` 命名。
- `q_index_map.json` 顶层会显式记录 `mode = "split_assign_index"` 或 `mode = "split_assign_coords"`；`qe_merge` 与 `workflow_status.json.merge` 都以它为唯一模式来源。
- `qe_merge` 会按 `q_index_map.json.mode` 自动切换 merge 规则：
  - `split_assign_coords`：允许子目录只产出 `<prefix>.dyn`，也允许 `dvscf` 直接位于子目录 `_ph0/`。
  - `split_assign_index`：主语义仍是 `<prefix>.dyn<index>` 与 `_ph0/<prefix>.q_<index>`；但第 1 个 q 点允许 `_ph0/dvscf` 特例。
- 每个 `01/run_split.sh` 只包含两步：
  - `pw.x < scf.in > scf.out`
  - `ph.x < split_ph.in > split_ph.out`
- `qe_phonon_*` 工作目录只保留 `POSCAR` 副本，不再复制 `input.toml`；所有 CLI 都要求通过 `-c` 显式传入真实配置路径。
- 声子工作流状态统一由 `qeflow.phonon.status_machine` 维护；`qe_check.checks` 只负责低层输出文件检查，`workflow_status.json` 的刷新、split task 汇总和续算阶段判断集中在状态机 Module。
- `qe_phonon_split_index` 与 `qe_phonon_split_coords` 共用 `qeflow.phonon.split_workflow` 的对象式编排；两个 CLI 文件只保留 argparse 和 mode 选择，避免两条 split 链路行为分叉。
- `qeflow.phonon._shared` 已收敛为兼容 facade；新的内部调用应直接依赖 `context.py`、`runner.py`、`script_plan.py`、`q_index_map.py`、`dyn0.py`、`root_steps.py`。
- 声子后处理拆分为 `postprocess_workflow.py`、`archive.py`、`phonon_tables.py`、`phonon_plot.py`；`process_phonon.py` 仅保留 CLI 和兼容 re-export。
- split merge 规则集中在 `split_merge.py`，`qe_merge.py` 仅保留 CLI 输出与状态刷新。
- 声子输入 deck 写出集中在 `qeflow.qe_input.phonon_input`；`qeflow.qe_input.builders` 仅作为旧导入路径的兼容 facade。

## 6. `qe_check`
当前实现的检查开关：
- `qe_check -scf`
- `qe_check -relax`
- `qe_check -ph`
- `qe_check -q2r`

预留但本次不实现：
- `qe_check -matdyn-band`
- `qe_check -matdyn-dos`
- `qe_check -eph`

返回值统一收敛为三态：
- `unstarted`
- `finished`
- `error`

## 7. split 工作流行为
- `qe_phonon_split_index`
  - `-r` 本地顺序执行 `scf -> dyn0 -> split 01/02/... -> merge -> q2r -> matdyn -> collect -> plot`。
  - `-s` 第一次提交 `submit_pre_and_split_stage.sh`，脚本在 Slurm 内运行 `scf -> dyn0 -> qe_split_submit`。
  - `qe_split_submit --mode index` 负责生成全部 `01/02/.../` 输入和脚本，并逐个进入 q 点目录执行 `sbatch run_split.sh`；只提交 `error/unstarted`，跳过 `finished`。
  - q 点全部完成后，再次执行 `-s` 会提交 `submit_merge_and_process_phonon.sh`，脚本在 Slurm 内直接展开 `qe_merge -> q2r -> matdyn_phband -> collect phband -> matdyn_phdos -> collect phdos -> plot`。
- `qe_phonon_split_coords`
  - 与上面相同，但 `split_ph.in` 写入单个 q 点坐标而不是 `start_q/last_q`，内部使用 `qe_split_submit --mode coords`。

split 工作流现在固定为两段 Slurm 提交：
- pre 准备段：`scf -> dyn0 -> qe_split_submit`，提交完成后等待所有 q 点 Slurm 任务结束。
- post 回收段：`qe_merge -> q2r -> matdyn_phband -> 归档 phband -> matdyn_phdos -> 归档 phdos -> plot`。

## 8. `qe_process_phonon`
- `qe_process_phonon -r` 在当前声子工作目录内顺序执行 `q2r -> matdyn_phband -> collect phband -> matdyn_phdos -> collect phdos -> plot`。
- `qe_process_phonon -s` 提交 `submit_process_phonon.sh`，脚本在 Slurm 内顺序执行 `q2r -> matdyn_phband -> collect phband -> matdyn_phdos -> collect phdos -> plot`，每步开始前都会输出当前步骤。
- `qe_process_phonon --collect phband` / `--collect phdos` 只把根目录已有的 `<prefix>.phband.freq`、`<prefix>_band.labelinfo.dat`、`<prefix>.phdos.dat` 分别归档到 `phband/` 与 `phdos/`；省略目标时同时归档二者。
- `qe_process_phonon --plot` 只读取 `phband/` 与 `phdos/` 中已归档的数据并生成 CSV/PNG，不自动回头收集根目录文件。
- `phband/phonon_band.csv` 保存 `point_index,distance,kx,ky,kz,mode_1_cm-1,...`；`phband/phonon_band_labels.csv` 保存高对称点标签、路径坐标和二维路径距离。
- `phdos/phonon_dos_atoms.csv` 保存频率、`TDOS` 和按结构原子顺序排列的原子级 DOS；`phdos/phonon_dos_elements.csv` 保存频率、`TDOS` 和元素级求和 DOS。DOS 图只绘制 `TDOS + 元素级曲线`，横轴为 DOS，纵轴为 `Frequency (cm^-1)`。
- `workflow_status.json` 复用 `matdyn_phband`、`matdyn_phdos` 记录归档后的计算状态，并用 `phonon_plot` 记录绘图状态。

## 9. 最小配置示例
```toml
pp_dir = "/path/to/pseudo"

[qe]
executable_path = "mpirun -np 8 /path/to/qe/bin"
lattice_mode = "ibrav_angstrom"

[slurm]
header = """
#!/bin/bash
#SBATCH -J qe_phonon
"""

[qe_phonon]
work_dir = "custom_ph"

[qe_phonon.scf]
restart_mode = "from_scratch"
verbosity = "high"
forc_conv_thr = "1.0d-6"
etot_conv_thr = "1.0d-7"
tstress = true
tprnfor = true
outdir = "./out"
kmesh = 0.04
ecutwfc = 60.0
ecutrho = 480.0

[qe_phonon.ph]
qpoints = [4, 4, 4]
fildvscf = "dvscf"

[qe_phonon.q2r]
zasr = "simple"

[qe_phonon.phband]
kpath_file = "KPATH.in"
kpath_points = 100

[qe_phonon.phdos]
qpoints = [12, 12, 12]
ndos = 200
dos = true
```

## 10. 说明
- `pp_dir` 只扫描目录直下文件，并按“文件名开头连续字母串”自动匹配元素赝势，例如 `Mg...`、`b_...`、`li_...`、`Hf-...`。
- `pp_dir` 支持相对路径；程序会优先检查当前工作目录，再从配置文件所在目录向上逐级搜索同名目录，便于在批量工作流里统一写成 `qe_pseudo_lib`。
- 若某元素在 `pp_dir` 中没有匹配文件，或匹配到多个候选，程序会直接报错，不做自动择优。
- `fildyn`、`flfrc`、`flfrq`、`fldos` 由程序按 `prefix` 自动派生，不建议手写。
- split q 点目录由 `qe_phonon_split_index` 或 `qe_phonon_split_coords` 内部物化，不再暴露独立 `qe_ph_split_assign_*` 命令。
- `qe_merge` 调用即直接执行 split 结果回收；`-s` 总工作流会把它写入 `submit_merge_and_process_phonon.sh`，避免在登录节点直接执行 merge。
