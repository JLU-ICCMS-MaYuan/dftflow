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

## 3. 声子 CLI 体系
### 3.1 独立单步 CLI
- `qe_ph_nosplit`
- `qe_merge`
- `qe_q2r`
- `qe_matdyn_phband`
- `qe_matdyn_phdos`
- `qe_process_phonon`
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

返回值统一收敛为四态：
- `unstarted`
- `running`
- `finished`
- `error`

`running` 统一通过 `squeue -u <user> -h --format='%12A %12P %5t %25R %8M %Z'` 的工作目录 `%Z` 判定，不靠文件猜测。

## 7. split 工作流行为
- `qe_phonon_split_index`
  - 根目录 `scf.out` 未完成时，只推进 SCF。
  - `scf.out` 完成但 `dyn0` 未完成时，只推进 `dyn0`。
  - `dyn0` 完成后，首次生成全部 `01/02/.../` 输入和脚本。
  - 若 q 子任务部分完成，只刷新并打印各目录状态，不自动回收任务。
  - 当全部 q 子任务完成后，先进入 `merge`，再执行 `q2r -> phband -> phdos`。
- `qe_phonon_split_coords`
  - 与上面相同，但 `split_ph.in` 写入单个 q 点坐标而不是 `start_q/last_q`。

split 工作流现在固定为四阶段：
- 前计算：`scf -> dyn0`
- split：全部 `01/02/.../run_split.sh`
- merge：程序内部直接执行 `qe_merge` 回收 split 结果
- 后处理：`qe_process_phonon` 统一执行或提交 `q2r -> matdyn_phband -> 归档 phband -> matdyn_phdos -> 归档 phdos`

## 8. `qe_process_phonon`
- `qe_process_phonon -r` 在当前声子工作目录内顺序执行 `q2r -> matdyn_phband -> collect phband -> matdyn_phdos -> collect phdos -> plot`。
- `qe_process_phonon -s` 提交 `submit_process_phonon.sh`，脚本只负责 `q2r/matdyn/collect`，不会在 shell 中调用 Python 绘图。
- `qe_process_phonon --collect` 只把根目录已有的 `<prefix>.phband.freq`、`<prefix>_band.labelinfo.dat`、`<prefix>.phdos.dat` 分别归档到 `phband/` 与 `phdos/`。
- `qe_process_phonon --plot` 只读取 `phband/` 与 `phdos/` 中已归档的数据并生成 CSV/PNG，不自动回头收集根目录文件。
- `phband/phonon_band.csv` 保存 `point_index,distance,kx,ky,kz,mode_1_cm-1,...`；`phband/phonon_band_labels.csv` 保存高对称点标签、路径坐标和二维路径距离。
- `phdos/phonon_dos_atoms.csv` 保存频率、`TDOS` 和按结构原子顺序排列的原子级 DOS；`phdos/phonon_dos_elements.csv` 保存频率、`TDOS` 和元素级求和 DOS。DOS 图只绘制 `TDOS + 元素级曲线`，横轴为 DOS，纵轴为 `Frequency (cm^-1)`。
- `workflow_status.json` 复用 `matdyn_phband`、`matdyn_phdos` 记录归档后的计算状态，并用 `phonon_plot` 记录绘图状态。

## 9. 最小配置示例
```toml
pp_dir = "/path/to/pseudo"

[qe]
executable_path = "mpirun -np 8 /path/to/qe/bin"

[slurm]
header = """
#!/bin/bash
#SBATCH -J qe_phonon
"""

[qe_phonon]
work_dir = "custom_ph"

[qe_phonon.scf]
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
- `qe_merge` 调用即直接执行 split 结果回收；split 工作流内部也统一复用这一路径。
