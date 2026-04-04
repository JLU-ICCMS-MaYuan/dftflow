# Wannier90flow 使用说明
## 0. executable_path 语义
- 与 `qeflow` 不同，Wannier90 相关模块不会从一个 bin 目录自动拼接 `*.x`。
- `[wannier90].executable_path`、`[pw2wannier90].executable_path`、`[postw90].executable_path` 都应直接填写各自程序的完整路径或完整命令。
- `test_auto/MgB2/input_w90.toml` 可以作为现有 TOML 示例；运行时默认模板定义在 `wannier90flow/templates.py`。
- 运行时默认模板定义在 `wannier90flow/templates.py`，`input.toml` 只负责覆盖。

## 0.1 覆盖规则
- 模板里有默认值，`input.toml` 没写：用模板默认值
- 模板里有默认值，`input.toml` 写了：用 `input.toml`
- 模板里没有默认值，但 `input.toml` 写了：用 `input.toml`
- `prefix`、`kpoint_path` 等依赖具体结构或运行现场的值，属于运行时派生值，不放进静态模板里

## 1. wannier_init

### 1.1 功能概述
- 读取 POSCAR 与配置文件（TOML/JSON），生成 `*.win` 输入文件。
- 自动生成 `pw2wannier90` 输入文件（若不存在）。
- 生成 `run_wannier90.sh` 运行脚本。
- 可选本地执行 `wannier90 -pp` → `pw2wannier90` → `wannier90`。

### 1.2 基本用法
```bash
wannier_init -i POSCAR -c wannier90flow/input.toml
```

### 1.3 直接执行（本地）
```bash
wannier_init -i POSCAR -c wannier90flow/input.toml --run
```

### 1.4 关键配置字段（示例）
```toml
[wannier_init]
system_name = "Y1H6"
num_wann = 9
num_bands = 21
exclude_bands = "1:4"

[wannier_init.k_points]
mp_grid = [6, 6, 6]

[wannier_init.projections]
list = ["Y:d", "H:s"]

[wannier90]
executable_path = "/path/to/wannier90.x"

[pw2wannier90]
executable_path = "/path/to/pw2wannier90.x"
input_file = "Y1H6.pw2wan"
prefix = "Y1H6"
outdir = "./out"
seedname = "Y1H6"
```

### 1.5 产出文件
- `wannier90/<seed>.win`
- `wannier90/run_wannier90.sh`
- `wannier90/<seed>.pw2wan`（若不存在会自动生成）

## 2. wannier_post

### 2.1 功能概述
- 生成 `run_postw90.sh` 运行脚本。
- 可选本地执行 `postw90` 以计算 DOS 等后处理任务。

### 2.2 基本用法
```bash
wannier_post -i wannier90/Y1H6.win -c wannier90flow/input.toml
```

### 2.3 直接执行（本地）
```bash
wannier_post -i wannier90/Y1H6.win -c wannier90flow/input.toml --run
```

### 2.4 关键配置字段（示例）
```toml
[postw90]
executable_path = "/path/to/postw90.x"

[wannier_post]
dos_kmesh = "25 25 25"
dos_energy_min = 5.0
dos_energy_max = 20.0
dos_energy_step = 0.01
dos_project = { Y_d = "1:5", H_p = "6:11" }
```

### 2.5 产出文件
- `wannier90/run_postw90.sh`
- `wannier90/<seed>_dos_Y_d.dat`（按 dos_project 名称重命名）

## 3. wannier_window

### 2.1 功能概述
- 解析 `EIGENVAL` 或 QE/FLEUR XML 中的能带数据。
- 提供能带范围查询、能量区间带数统计、自动能窗估计。

### 2.2 基本用法
```bash
wannier_window EIGENVAL -n 5
wannier_window EIGENVAL -n 1 8
```

```bash
wannier_window qe.xml -e 5.0 12.0
wannier_window qe.xml -e 5
```

### 2.3 自动能窗估计
```bash
wannier_window EIGENVAL --auto 1 9 3
```

#### 可选偏移参数
```bash
wannier_window EIGENVAL --auto 1 9 3 --auto-delta 0.01
```

### 2.4 输出说明
- `-n 1`：返回该能带的 `emin/emax`
- `-n 1 8`：返回 band1 的最小值与 band8 的最大值
- `-e emin emax`：统计能量区间内每个 k 点的带数，并输出最小/最大值
- `-e band_index`：等价于 `-n band_index`
- `--auto nbnd1 num_wann nbnd3`：
  - `dis_froz_min = dis_win_min = band(nbnd1).min - delta`
  - 自适应调整 `nbnd2=nbnd1+num_wann`，直到区间最大带数等于 `num_wann`
  - `dis_froz_max = band(nbnd2).max + delta`
  - `dis_win_max = band(nbnd2+nbnd3).max + delta`
