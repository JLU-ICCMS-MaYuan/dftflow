# 快速开始：qeflow Rust 二进制

## 开发构建（Darwin / Linux）
```bash
cd qeflow-rust
cargo build --release        # 开发验证，产物 target/release/qeflow
```

## 发布构建（Linux musl 静态二进制）
```bash
rustup target add x86_64-unknown-linux-musl
cd qeflow-rust
cargo build --release --target x86_64-unknown-linux-musl
strip target/x86_64-unknown-linux-musl/release/qeflow
```

## 命令行映射
`qeflow` 的单二进制 `clap` 子命令映射现有 `qe_*` 命令：

| 原 Python CLI | Rust 子命令 |
| --- | --- |
| `qe_scf` | `qeflow qe-scf` |
| `qe_nscf` | `qeflow qe-nscf` |
| `qe_eband` | `qeflow qe-eband` |
| `qe_eledos` | `qeflow qe-eledos` |
| `qe_phonon_split_*` | `qeflow qe-phonon-split-*` |
| `scflow` | `qeflow scflow` |

参数 `-i/--input`、`-c/--config`、`-r/--run`、`-s/--submit` 与原 CLI 行为兼容。

## 最小示例
```bash
qeflow qe-scf -i MgB2.vasp -c input_qe.toml --run
qeflow qe-nscf -i MgB2.vasp -c input_qe.toml --run
qeflow scflow -c input.toml
```

## 配置兼容
复用现有 `input.toml`，递归覆盖语义与 `[qe]/[slurm]/[qe_phonon]` 配置块保持不变；`executable_path` 仍表示"启动命令+bin 目录"，运行时自动拼接 `pw.x/ph.x/q2r.x/matdyn.x`。
