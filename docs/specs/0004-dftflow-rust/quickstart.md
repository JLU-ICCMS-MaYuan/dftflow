# 快速开始：dftflow Rust

## macOS

```bash
brew install openblas spglib
cargo build --release --workspace --locked
cargo test --workspace --locked
# 将 release 目录复制到稳定版本目录，再安装；该目录不可随后删除。
bash scripts/install-compat.sh /absolute/path/to/version/bin ~/.local/bin
export PATH="$HOME/.local/bin:$PATH"
bash scripts/verify-entries.sh /absolute/path/to/version/bin
```

## Linux 交付（可以在 macOS 执行）

需要 Docker/OrbStack 正常运行，支持 Linux amd64 镜像：

```bash
bash scripts/linux/build.sh
```

脚本固定 Linux 构建镜像与原生依赖源码版本，在目标环境构建、测试、strip、检查 ELF 静态依赖、断网重跑测试，最后生成 `dist/dftflow-linux-x86_64-musl.tar.gz`。不得把 macOS release 目录当作 Linux 交付物。

## MgB2

```bash
qe_scf -i MgB2.vasp -c input_qe.toml -r
qe_nscf -i MgB2.vasp -c input_qe.toml -r
# 只生成输入：去掉 -r。Slurm 提交：换成 -s。
scflow -c input.toml
```

仓库 `test_auto/MgB2/input_qe.toml` 中的机器路径占位符，先按 [手动测试手册](../../manual-testing.md) 注入到独立工作目录。QE、MPI、赝势与 Slurm 是外部依赖；本机容器验证不等于远程计算节点配置完成。

旧 Python 参考为 `python-reference-0004`。`mattersim_opt` 是保留的 Python 例外，普通 Rust 路径不需要 Python。
