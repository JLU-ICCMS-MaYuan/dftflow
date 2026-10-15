# 固定路径更新与编译历史清理

## 当前目录布局

```text
~/.local/share/dftflow/
├── current/bin/                        五个主程序，更新覆盖同一路径
├── native/linux-x86_64/                原生依赖与私有加载器，保留
└── tools/qf-gen-dyn0/current/bin/       独立工具，保留
~/.local/bin/                          指向上述固定程序的命令入口
```

新安装流程不创建 versions 时间戳目录，默认不备份入口。主程序更新命令：

```bash
build_bin="$PWD/target/linux-x86_64/x86_64-unknown-linux-gnu/release"
bash scripts/install-current.sh "$build_bin"
```

macOS 使用对应构建目录；旧 glibc 计算节点需先按根 README 完成兼容打包，再把打包目录传给安装器。程序先复制到临时目录，通过全部入口检查后逐文件原子替换，结束时删除临时文件。五个程序并非整组原子切换；应在计算链无活动作业时更新。配置快照仍固定，current 程序会随更新变化。

## 可以清理什么

| 内容 | 删除条件与原因 |
| --- | --- |
| 旧 versions/<版本>/ | 迁移有效程序、入口和仍需使用的旧脚本，确认无作业引用后删除；属于被替代的安装副本。 |
| 仓库 target/ | 无编译活动且没有脚本直接调用其中程序时可以清理；是可重新生成的缓存，下次构建更慢。独立工具自己的 target/ 同理。 |
| dist/ 旧交付包 | 已安装且无需再次分发或回退后删除。 |
| ~/.local/bin/.dftflow-backup-* | 无需恢复旧入口后删除；通常只是旧软链接，不会释放对应安装程序的空间。 |
| native/、current/、独立工具 | 保留，属于有效程序、编译依赖或运行依赖。 |
| 计算目录、输入快照、QE 输出、docs/local 验收证据 | 不属于编译垃圾，不纳入清理范围。 |

## 迁移已有 versions 安装

先核对命令入口及其实际目标：

```bash
find "$HOME/.local/bin" -maxdepth 1 -type l -exec ls -l {} +
du -sh "$HOME/.local/share/dftflow/versions/"*
```

Slurm 集群还需检查排队和运行作业：

```bash
squeue -u "$USER" -o "%.18i %.10T %.30j %Z"
# 用实际作业号替换；查看 Slurm 保存的提交脚本，不执行脚本
scontrol write batch_script 123456 stdout
```

存在活动作业时先等待其完成。扫描自己的所有计算根目录，核对磁盘脚本是否仍引用旧程序；Slurm 保存的脚本也要单独检查：

```bash
run_root="/path/to/calculations"
rg -n --hidden --no-ignore --glob "*.sh" --fixed-strings \
    "$HOME/.local/share/dftflow/versions/" "$run_root"
```

选择入口实际指向的有效版本作为迁移源，不凭目录名判断新旧：

```bash
# 把此示例路径换成实际有效版本；该版本须已完成需要的运行库打包
source_bin="$HOME/.local/share/dftflow/versions/your-active-version/bin"
bash scripts/install-current.sh "$source_bin"
```

重新生成仍需使用的脚本，或者只替换其中的旧程序绝对路径为 current/bin，保留计算参数、输入快照和命令参数。确认脚本可用及入口验证通过后，逐个明确指定旧目录删除：

```bash
# 仅在实际核对迁移完成后执行；不要原样复制占位符
rm -r -- "$HOME/.local/share/dftflow/versions/your-old-version"
"$HOME/.local/bin/qeflow" --help
"$HOME/.local/bin/qf-gen-dyn0" --help
```

不要删除 native，私有 ELF 加载器使用绝对路径；不要删除 current 或有效入口。搜索没有命中不能证明尚未扫描的目录没有引用。

## 清理编译缓存

在仓库根目录选择对应平台执行，已安装 current 不受影响：

```bash
# Linux x86_64
cargo --config ".cargo/linux-x86_64.toml" clean
# macOS arm64
cargo --config ".cargo/macos-arm64.toml" clean
```

## 本机清理记录

2026-10-15 首次清理删除 6 个无已知引用旧版本，约 242 MiB；记录位于 gitignored 的 docs/local/build-history-cleanup-20261015.json。随后执行固定 current 迁移的结果见 [验收记录](acceptance.md)。版本清单属于历史记录，不应再作为当前删除清单。其他材料 work 目录未完成全量扫描，其中旧脚本复用前需检查或重新生成。
