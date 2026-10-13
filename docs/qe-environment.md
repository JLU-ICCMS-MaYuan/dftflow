# QE 计算环境与可执行文件路径参考

本文档集中记录运行 `qeflow` 相关计算所需的**环境变量**与 **QE 可执行文件路径**的填写规则，
在编写 `input.toml` 时以本文件作为契约参考。

> **机器真实路径不入 Git（Issue 0005）**：本文件只写通用格式与占位符；
> 各机器（Mac 本机、计算集群）的 QE/VASP/Wannier90/LOBSTER 真实路径统一记录在
> 本地留档文件 `docs/local/paths.toml`（已被 `.gitignore` 忽略，永不上传）。
> macOS QE 编译踩坑记录见本地留档 `docs/local/qe-install-m5.md`。

## 一、QE 可执行文件路径

| 项目 | 说明 |
| --- | --- |
| QE 可执行文件目录 | QE 编译产出的 `build/bin` 目录（含 `pw.x`、`ph.x`、`q2r.x`、`matdyn.x`、`cell2ibrav.x` 等） |

> 在 `input.toml` 的 `[qe]` 段中，`executable_path` 应填写包含上述 bin 目录的**完整启动命令**
> （例如 `mpirun -np 8 /path/to/q-e/build/bin`），`qeflow` 会自动在该目录下
> 拼接 `pw.x` / `ph.x` / `q2r.x` / `matdyn.x` 等具体二进制名。
> 若该目录下存在 `cell2ibrav.x`，`qeflow` 会优先调用它做晶格参数 `ibrav` 自动判断；否则回退到内置 Python 等价逻辑。

## 二、推荐环境变量（参考）

以下环境变量不是 `qeflow` 强制读取，但在本地手动运行 QE 或调试时建议设置。

```bash
# 将 QE 可执行文件目录加入 PATH（可选，便于手动调用 pw.x 等）
export PATH="/path/to/q-e/build/bin:$PATH"

# 并行/线程控制（按实际机器核数调整）
export OMP_NUM_THREADS=8

# 若使用 mpirun 启动 QE，可在此指定 MPI 运行环境（视本机 MPI 安装而定）
# export MPIRUN="mpirun -np 8"
```

## 三、与 `input.toml` 的对应关系

| `input.toml` 字段 | 含义 | 参考值 |
| --- | --- | --- |
| `[qe].executable_path` | QE 启动命令 + bin 目录 | `mpirun -np N /path/to/q-e/build/bin` |
| `[qe].lattice_mode` | 晶格写出模式 | `ibrav_angstrom` / `ibrav_celldm` / `cell_parameters` |
| `[qe].pseudo_dir` | 赝势目录（也可在 `pp_dir` 单独指定） | 见具体项目赝势目录 |

## 四、测试输入文件的占位符机制

`test_auto/MgB2/input_*.toml` 及 `local_run/` 下的输入与脚本使用 `@KEY@` 占位符，
真实路径来自 `docs/local/paths.toml` 的 `[placeholders]` 表：

| 占位符 | 用途 | 使用位置 |
| --- | --- | --- |
| `@QE_EXEC@` | QE 启动命令 + bin 目录 | `input_qe.toml` |
| `@VASP_EXEC@` | 集群 VASP 启动命令 | `input_vasp.toml` |
| `@LOBSTER_EXEC@` | 集群 LOBSTER 可执行文件 | `input_vasp.toml` |
| `@WANNIER90_EXEC@` / `@PW2WANNIER90_EXEC@` / `@POSTW90_EXEC@` | Wannier90 三件套 | `input_w90.toml` |
| `@LOCAL_VASP_EXEC@` / `@LOCAL_LOBSTER_EXEC@` | 本机 macOS VASP/LOBSTER | `local_run/*` |

- pytest 运行时由 `test_auto/MgB2/conftest.py` 自动注入，无需手动操作；
- 手动计算（如 local_run 脚本）前，先执行
  `python test_auto/MgB2/_local_paths.py test_auto/MgB2/local_run`
  就地替换占位符（替换产生的真实路径文件**不要提交**）。

## 五、维护约定

- QE/VASP 等安装目录变化时，**只更新本机的 `docs/local/paths.toml`**，不把真实路径写回本文件或任何已跟踪文件；
- 新增占位符时，同步更新本表、`docs/local/paths.toml` 与 `test_auto/MgB2/local_paths.py` 的校验清单。
