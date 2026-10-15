# 编译历史清理操作指南

清理分为两类：仓库 `target/` 是可重新生成的构建缓存，`~/.local/share/dftflow/versions/` 是安装后的可执行程序。旧安装副本只有在命令入口、作业和需要继续使用的脚本都不再引用时才能删除。**本节只提供操作说明，不会自动清理。** 当前安装仍采用 `versions/` 布局；固定路径覆盖安装尚未实施。

## 哪些可以清理，哪些需要保留

| 内容 | 清理条件与原因 |
| --- | --- |
| `versions/<旧版本>/` | 确认没有入口或作业引用，且无需回退后，可删除整个旧版本目录；它们是重复安装的五个程序副本。不要只凭时间或 `candidate`、`tmp` 名称判断。 |
| 仓库 `target/` | 没有正在进行的编译，且安装副本已独立复制后可清理；下次构建会重新生成，耗时会增加。独立工具的 `tools/qf-gen-dyn0/target/` 同理。 |
| 仓库 `dist/` 中旧交付包 | 已完成安装、无需分发或回退的包可清理；保留仍需交付的版本。 |
| `~/.local/bin/.dftflow-backup-*` | 不需要恢复旧命令入口后可清理。这些通常是旧软链接，体积小；删除它们不会自动删除 `versions/` 中的程序。 |
| `native/` | 保留。本机已安装 ELF 依赖其中的私有加载器和共享库；头文件、静态库及 pkgconfig 元数据用于再次编译，不能把整个目录当历史垃圾删除。 |
| `tools/qf-gen-dyn0/<正在使用的版本>/` | 保留。这是独立工具的正式安装程序，不是主项目重复编译缓存；其旧版本也必须先检查引用。 |
| `~/.local/bin/` 中有效命令入口 | 保留。删除后会影响命令查找；删除其目标目录则会留下失效链接。 |
| 计算目录、输入快照、QE 输出及 `docs/local/` 验收资料 | 不属于编译历史垃圾，不纳入这次清理范围。 |

## 本机历史版本清单（2026-10-15 核对）

本次检查时，51 个主程序入口均指向 `versions/linux-x86_64-phonon-additions-20261015-094047/bin/`；排队中的后处理作业脚本也固定调用该版本，因此必须保留。独立工具入口指向 `tools/qf-gen-dyn0/20261014-180957/bin/`，同样保留。其他用户安装时应以自己实际的入口和作业引用为准。

下面 12 个目录是**待检查引用的清理候选**，均位于 `~/.local/share/dftflow/versions/`。合计约 495 MiB；删除理由都是“已由当前版本替代的安装副本”，不是计算数据过期：

```text
linux-x86_64-0006-20261014
linux-x86_64-0006-candidate
linux-x86_64-0006-dyn0-20261014
linux-x86_64-20261014-140742-bf79876
linux-x86_64-20261014-180957
linux-x86_64-nosplit-input-20261014-202240
linux-x86_64-phonon-additions-20261015-092724
linux-x86_64-phonon-additions-20261015-093111
linux-x86_64-phonon-console-20261014-203516
linux-x86_64-phonon-plot-20261014-205600
linux-x86_64-phonon-plot-20261014-210009
linux-x86_64-tmp-20261014-190259
```

## 删除前检查与手动清理

先查看大小及入口目标。以下命令适用于 Linux/macOS；`find` 会列出该目录中的软链接，也需自行检查其他安装位置、shell 别名或包装脚本：

```bash
du -sh "$HOME/.local/share/dftflow/versions/"*
find "$HOME/.local/bin" -maxdepth 1 -type l -exec ls -l {} +
```

在 Slurm 集群上检查自己的排队和运行作业，并核对其实际提交脚本以及后续调用的脚本。Slurm 保存的脚本可能与磁盘上的脚本不同，不能只检查当前文件：

```bash
squeue -u "$USER" -o "%.18i %.10T %.30j %Z"
# 用实际作业号替换 123456；stdout 表示只查看，不写入脚本文件
scontrol write batch_script 123456 stdout
```

然后针对每个候选版本搜索仍准备复用的脚本。下面的 `run_root` 必须改成自己的计算根目录；有多个计算根目录时逐一检查。`--hidden --no-ignore` 用于覆盖被 Git 忽略的运行目录：

```bash
run_root="/path/to/calculations"
old_version="linux-x86_64-0006-candidate"
rg -n --hidden --no-ignore --glob "*.sh" --fixed-strings \
    "$HOME/.local/share/dftflow/versions/$old_version/" "$run_root"
```

如果仍有作业引用，等其结束再清理；如果仍需复用旧脚本，应先用当前程序重新生成并检查脚本。搜索没有命中也不代表其他计算目录或调度器保存的脚本没有引用。

确认某一个目录不再使用后，明确指定该目录删除。**不要对整个 `versions/`、`native/` 或安装根目录执行批量删除。** 以下只以一个历史候选版为例：

```bash
rm -r -- "$HOME/.local/share/dftflow/versions/linux-x86_64-0006-candidate"
# 清理后检查常用入口仍可启动
"$HOME/.local/bin/qeflow" --help
"$HOME/.local/bin/qf-gen-dyn0" --help
```

需要释放构建缓存时，在仓库根目录执行对应平台的清理命令（二选一）。这不会清理安装副本，但会清掉对应平台的构建产物；确保没有脚本直接调用 `target/` 中的程序：

```bash
# Linux x86_64
cargo --config ".cargo/linux-x86_64.toml" clean
# macOS arm64
cargo --config ".cargo/macos-arm64.toml" clean
```
