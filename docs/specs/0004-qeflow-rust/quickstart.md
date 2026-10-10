# 快速开始：dftflow Rust 交付形态（规划）

> 以下是 P5 验收后的预期用法，**不是当前工作树可直接执行的安装说明**。当前 Rust 代码仍在 `qeflow-rust/`，五个完整二进制与 48 个兼容入口尚未实现。

## 构建与发布

```bash
# P0 建立根 workspace 后，在开发机验证
cargo build

# P5 在具备静态 OpenBLAS、spglib 与 PDF 所需依赖的 Linux 构建环境发布
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
# 对五个交付二进制逐一执行 strip，并用 file/readelf 验证静态链接
```

发布包包含 `qeflow`、`vaspflow`、`scflow`、`wannier90`、`substitution` 五个二进制、48 个旧命令名的同名兼容入口，以及声子 dyn0 生成所需的 `qf-gen-dyn0` 别名。兼容入口指向家族二进制，不复制二进制文件。安装时须保证本地和 Slurm 计算节点的 PATH 都能找到它们。

## 命令兼容

| 命令族 | 旧命令数量 | 交付入口 |
| --- | ---: | --- |
| `qe_*` | 17 | 旧名直调 `qeflow`，例如 `qe_scf`、`qe_phonon_split_index` |
| `vasp_*` | 19 | 旧名直调 `vaspflow`，含 `vasp_elf` |
| `scflow` | 1 | 独立二进制 `scflow` |
| `wannier_*` | 4 | 旧名直调 `wannier90` |
| `substitution*` | 7 | 旧名直调 `substitution` |
| `mattersim_opt` | 1，范围外 | 保留 Python CLI；仅可选 ML 路径需要 |

旧命令的参数名、短选项、默认值和退出码按冻结 Python 版本对照。家族二进制的子命令可以作为附加用法，但不能代替旧名兼容要求；生成的 Slurm 脚本仍可调用旧名。

```bash
qe_scf -i MgB2.vasp -c input_qe.toml -r
qe_nscf -i MgB2.vasp -c input_qe.toml -r
scflow -c input.toml
```

## 配置、图像与可选 Python

复用现有 `input.toml`。实际配置块包含 `[qe]`、`[qe_phonon.*]`、`[vasp]`、`[wannier90]`、`[substitutionflow]`、`[scflow]` 等；详细语义见 `contracts/config-and-output.md`。

CSV/DAT/JSON 的文件结构精确对齐、浮点按预定容差对照；图片保留 PNG、phonon 联合 JPG 与 scflow PDF 的文件形态和内容。普通 Rust 路径无需 Python；选择 substitution 的 `mattersim_opt` 步骤时须另行安装原有 MatterSim Python 环境和保留的 `mattersim_opt` CLI。QE/VASP/LOBSTER/vaspkit 仍需由目标机提供。
