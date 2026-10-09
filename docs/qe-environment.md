# QE 计算环境与可执行文件路径参考

本文档集中记录运行 `qeflow` 相关计算所需的**环境变量**与 **QE 可执行文件路径**，
在后续编写 `input.toml` 时，请以本文件作为信息参考来源，确保 `executable_path` 等配置字段指向正确的二进制目录。

## 一、QE 可执行文件路径

| 项目 | 路径 |
| --- | --- |
| QE 可执行文件目录 | `/Users/mayuan/Codes/dftflow/external/q-e/build/bin` |

该目录下包含 `pw.x`、`ph.x`、`q2r.x`、`matdyn.x`、`cell2ibrav.x` 等 QE 原生可执行程序。

> 在 `input.toml` 的 `[qe]` 段中，`executable_path` 应填写包含上述 bin 目录的**完整启动命令**
> （例如 `mpirun -np 8 /Users/mayuan/Codes/dftflow/external/q-e/build/bin`），`qeflow` 会自动在该目录下
> 拼接 `pw.x` / `ph.x` / `q2r.x` / `matdyn.x` 等具体二进制名。
> 若该目录下存在 `cell2ibrav.x`，`qeflow` 会优先调用它做晶格参数 `ibrav` 自动判断；否则回退到内置 Python 等价逻辑。

## 二、推荐环境变量（参考）

以下环境变量不是 `qeflow` 强制读取，但在本地手动运行 QE 或调试时建议设置，便于与本文件保持一致。

```bash
# 将 QE 可执行文件目录加入 PATH（可选，便于手动调用 pw.x 等）
export PATH="/Users/mayuan/Codes/dftflow/external/q-e/build/bin:$PATH"

# 并行/线程控制（按实际机器核数调整）
export OMP_NUM_THREADS=8

# 若使用 mpirun 启动 QE，可在此指定 MPI 运行环境（视本机 MPI 安装而定）
# export MPIRUN="mpirun -np 8"
```

## 三、与 `input.toml` 的对应关系

| `input.toml` 字段 | 含义 | 参考值 |
| --- | --- | --- |
| `[qe].executable_path` | QE 启动命令 + bin 目录 | `mpirun -np 8 /Users/mayuan/Codes/dftflow/external/q-e/build/bin` |
| `[qe].lattice_mode` | 晶格写出模式 | `ibrav_angstrom` / `ibrav_celldm` / `cell_parameters` |
| `[qe].pseudo_dir` | 赝势目录（也可在 `pp_dir` 单独指定） | 见具体项目赝势目录 |

> 本文件是配置 `input.toml` 时的**权威路径来源**；如 QE 安装目录发生变化，请同步更新本文件，
> 并保持 `input.toml` 中的 `executable_path` 与新路径一致。
