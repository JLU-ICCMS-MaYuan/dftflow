# QEFLOW 使用说明

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
  - `qe_ph_nosplit` / `qe_ph_split_*` 的 `ph.x`
  - `qe_matdyn_phband` / `qe_matdyn_phdos` 的 `matdyn.x`

## 2. 模板与覆盖
- 默认模板定义在 [templates.py](/home/mayuan/code/dftflow/qeflow/templates.py)。
- 递归覆盖逻辑定义在 [config_templates.py](/home/mayuan/code/dftflow/public/config_templates.py)。
- `qe_phonon` 相关新 CLI 继续共用同一个 `[qe_phonon]` 配置块，不为每个命令单独拆 TOML 节。

## 3. 声子 CLI 体系
### 3.1 独立单步 CLI
- `qe_ph_nosplit`
- `qe_ph_split_assign_index`
- `qe_ph_split_assign_coords`
- `qe_merge`
- `qe_q2r`
- `qe_matdyn_phband`
- `qe_matdyn_phdos`
- `qe_check`

### 3.2 可恢复总工作流 CLI
- `qe_phonon_nosplit`
- `qe_phonon_split_assign_index`
- `qe_phonon_split_assign_coords`

## 4. `work_dir` 规则
- 用户始终在“当前目录”执行命令。
- 程序先读取当前目录的配置文件，再读取 `[qe_phonon].work_dir`。
- 如果当前目录 basename 等于 `work_dir`，说明用户已经进入真实计算目录，程序直接在当前目录检查和续算。
- 如果当前目录 basename 不等于 `work_dir`，程序会进入或创建 `./<work_dir>` 作为真实计算目录。
- 因此用户不需要区分自己应不应该先 `cd qe_phonon`。

## 5. 目录结构
`work_dir` 内部固定使用以下语义：

```text
qe_phonon/
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
  run_merge.sh
  workflow_status.json
  Mg1B2.dyn0
  q_index_map.json
  q_01/
    scf.in
    split_ph.in
    run_split.sh
    task_status.json
  q_02/
  ...
```

- split 子目录固定使用 `q_01`、`q_02`、`q_03` 命名。
- `q_index_map.json` 顶层会显式记录 `mode = "split_assign_index"` 或 `mode = "split_assign_coords"`；`qe_merge` 与 `workflow_status.json.merge` 都以它为唯一模式来源。
- `qe_merge` 会按 `q_index_map.json.mode` 自动切换 merge 规则：
  - `split_assign_coords`：允许子目录只产出 `<prefix>.dyn`，也允许 `dvscf` 直接位于子目录 `_ph0/`。
  - `split_assign_index`：主语义仍是 `<prefix>.dyn<index>` 与 `_ph0/<prefix>.q_<index>`；但第 1 个 q 点允许 `_ph0/dvscf` 特例。
- 每个 `q_XX/run_split.sh` 只包含两步：
  - `pw.x < scf.in > scf.out`
  - `ph.x < split_ph.in > split_ph.out`

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
- `qe_phonon_split_assign_index`
  - 根目录 `scf.out` 未完成时，只推进 SCF。
  - `scf.out` 完成但 `dyn0` 未完成时，只推进 `dyn0`。
  - `dyn0` 完成后，首次生成全部 `q_XX/` 输入和脚本。
  - 若 q 子任务部分完成，只刷新并打印各目录状态，不自动回收任务。
  - 当全部 q 子任务完成后，先进入 `merge`，再执行 `q2r -> phband -> phdos`。
- `qe_phonon_split_assign_coords`
  - 与上面相同，但 `split_ph.in` 写入单个 q 点坐标而不是 `start_q/last_q`。

split 工作流现在固定为四阶段：
- 前计算：`scf -> dyn0`
- split：全部 `q_XX/run_split.sh`
- merge：根目录 `run_merge.sh`
- 后处理：`q2r -> matdyn_phband -> matdyn_phdos`

## 8. 最小配置示例
```toml
pp_dir = "/path/to/pseudo"

[qe]
executable_path = "mpirun -np 8 /path/to/qe/bin"

[slurm]
header = """
#!/bin/bash
#SBATCH -J qe_phonon
"""

[pseudo_map]
Mg = { pseudo = "Mg.UPF" }
B = { pseudo = "B.UPF" }

[qe_phonon]
work_dir = "qe_phonon"

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

## 9. 说明
- `pseudo_map` 只强制要求 `pseudo`，质量会按元素自动补。
- `fildyn`、`flfrc`、`flfrq`、`fldos` 由程序按 `prefix` 自动派生，不建议手写。
- `qe_ph_split_assign_index` 与 `qe_ph_split_assign_coords` 假定根目录 `scf.out` 已完成；若希望自动推进 SCF，请使用 `qe_phonon_split_assign_index` 或 `qe_phonon_split_assign_coords`。
- `qe_merge` 可独立检查、执行或提交 split 结果回收；split 工作流内部也统一复用这一路径。
