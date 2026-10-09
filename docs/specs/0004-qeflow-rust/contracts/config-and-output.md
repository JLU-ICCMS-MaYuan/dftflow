# 契约：输入配置与输出

## 1. 输入配置（`input.toml`）

### 递归覆盖语义（复刻 `config_templates._merge_dict_in_place`）
- 仅当 `base[key]` 与 `override[key]` **均为 dict** 时递归合并。
- 否则 `override` 覆盖 `base`。
- `override[key]` 为 `None` 或键缺失时**不覆盖** `base`。
- 保证现有 TOML 配置（含 `[qe]` 默认模板与用户覆盖）行为不变。

### 配置块
- `[qe]`：`executable_path`、`lattice_mode`、`pseudo_dir` 等。
- `[slurm]`：队列、分区、节点、时间等两段提交参数。
- `[qe_phonon]`：split 网格、pre/post 阶段参数。
- `[scflow]`：μ*、展宽、求解器选项。

### `executable_path` 契约（复刻 `resolve_qe_command`）
- 取值为"启动命令 + bin 目录"，例如 `mpirun -np 8 /path/to/qe/bin`。
- 运行时按所需二进制自动拼接：`pw.x` / `ph.x` / `q2r.x` / `matdyn.x`。
- 例：`resolve_qe_command("mpirun -np 8 /qe/bin", "pw.x") => "mpirun -np 8 /qe/bin/pw.x"`。

## 2. 状态文件

### `workflow_status.json`
- 顶层记录 scf / ph / merge / q2r / matdyn_* 等阶段状态。
- 复用三态枚举：`unstarted` / `finished` / `error`。

### `task_status.json`
- 记录 phonon q 点级别 task 状态（split 工作流）。
- 续算依据 `split_resume_step` 的 1–11 阶段判定跳过 `finished`。

## 3. 计算与后处理输出
- QE 原生输出（`scf.out`/`nscf.out`/`eband.out`/`ph.out`/`dyn*`）保持原有文件名与格式（由外部 QE 决定）。
- 后处理产物：`*_band.*`、`*.tdos`/`*.pdos.*`、`phonon_band_dos.jpg`/`phdos/phonon_dos.png`、scflow `Tc` 相关 CSV/PNG。
- 绘图用 `plotters` 替代 matplotlib，PNG/CSV 形态、坐标轴与共享频率轴与原输出一致。

## 4. Slurm 两段提交脚本
- **pre**：scf → dyn0 → split 提交。
- **post**：merge → q2r → matdyn → plot。
- 生成逻辑 1:1 对齐现有 `qe_phonon_split_*` 脚本字段与顺序。
