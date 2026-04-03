# QEFLOW 使用说明
## 1. executable_path 语义
- `[qe].executable_path` 填写的是“启动命令 + QE bin 目录”，不是具体某个 `*.x` 的路径。
- 典型写法：

```toml
[qe]
executable_path = "mpirun -np 8 /home/mayuan/soft/qe-7.4.1/bin"
```

- 各 CLI 会在这个 bin 目录后自动拼接对应可执行文件：
  - `qe_scf` / `qe_nscf` 自动使用 `pw.x`
  - `qe_eband` 自动使用 `pw.x`、`bands.x`、`projwfc.x`
  - `qe_eledos` 自动使用 `dos.x`、`projwfc.x`
- 如果在 `[bands]`、`[projwfc]`、`[dos]` 中单独提供 `executable_path`，则该模块优先使用自己的值。

## 2. 基础模板
- 仓库自带共享模板：`qeflow/input.toml`
- 测试样例：`test_auto/MgB2/input_qe.toml`

## 3. 关键配置块
```toml
[qe]
executable_path = "mpirun -np 8 /home/mayuan/soft/qe-7.4.1/bin"

[qe_params.CONTROL]
outdir = "'./tmp'"
pseudo_dir = "'/path/to/pseudo'"

[qe_params.SYSTEM]
ecutwfc = 60.0
ecutrho = 480.0

[qe_params.ELECTRONS]
conv_thr = 1.0e-8

[k_points]
kpoints = [6, 6, 6]
wan = false

[pseudo_map]
Y = { pseudo = "Y_pbe_v1.uspp.F.UPF" }
H = { pseudo = "H.pbe-rrkjus_psl.1.0.0.UPF" }
```

## 4. 模块覆盖示例
`qe_eband` 与 `qe_eledos` 可以给某些后处理模块单独指定命令来源：

```toml
[bands]
executable_path = "mpirun -np 8 /home/mayuan/soft/qe-7.4.1/bin"
args = "-pd .true."

[projwfc]
executable_path = "mpirun -np 8 /home/mayuan/soft/qe-7.4.1/bin"
args = "-pd .true."

[dos]
executable_path = "mpirun -np 8 /home/mayuan/soft/qe-7.4.1/bin"
args = "-pd .true."
```
