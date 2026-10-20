# 构建目录迁移验证指南

本指南定义实施后的端到端检查流程。文档阶段不运行构建，也不移动现有二进制。

## 前置条件

- 在仓库根目录操作，Rust/Cargo、原生依赖与平台配置已就绪。
- 构建入口会使用仓库 `bin/`；如目标环境依赖本地忽略配置，先确认配置文件存在。
- EPW 测试目录迁移前，先完成运行任务、脚本引用和哈希来源检查。

## Workspace 构建与位置检查

```bash
bash scripts/build-native.sh build --release --workspace --locked
```

预期：公开程序出现在以下位置，全部可执行；已有兼容命令软链接也位于对应家族目录：

```text
bin/epwflow/{epw_phonon,epw_wannier,epw_superconductivity}
bin/qeflow/qeflow
bin/vaspflow/vaspflow
bin/substitutionflow/substitution
bin/wannierflow/wannier90
bin/scflow/scflow
```

## 独立工具

```bash
bash scripts/build-native.sh build --release --locked --manifest-path tools/qf-gen-dyn0/Cargo.toml
bin/tools/qf-gen-dyn0 --help
```

若 Cargo 参数透传不能同时保持 workspace 平台配置与独立工程设置，使用 Plan 实施时定义的仓库封装入口；不得改用算例路径。

## 运行和打包验证

```bash
cargo test --workspace --locked
bash scripts/tests/install-current.sh bin
bash scripts/tests/install-bin.sh "<扁平 staging 目录>" "<独立临时目标目录>" "bin/tools/qf-gen-dyn0"
```

实际隔离安装测试应使用全新临时目录和由本次构建生成的 staging；示例占位路径须替换。Linux musl 构建完成后核验发布清单、静态依赖和离线帮助启动。

## 路径与重复构建检查

```bash
find bin -maxdepth 3 -type f -perm -111 -print | sort
rg -n 'bin-window-contract|local_run/.*/bin|target/release/(qeflow|vaspflow|wannier90|substitution|epw_)' test_auto scripts README.md docs
git status --short
```

预期：最终程序仅位于指定家族目录；缓存在 `bin/.build/`；测试/运行目录没有 DFTFlow 产品 ELF；活动引用扫描无旧程序路径；`bin/` 仍被忽略。重复执行构建后，家族交付路径集合一致，程序哈希与本次 staging 一致。

## 从任意目录使用命令

`PATH` 不递归搜索子目录，因此需要加入七个家族目录。以下配置适用于 Bash/Zsh；其他机器将第一行替换为本机仓库根目录下 `bin` 的绝对路径。

```bash
_dftflow_bin="/data/home/mayuan/code/dftflow/bin"
for _dftflow_family in tools scflow wannierflow vaspflow substitutionflow qeflow epwflow; do
    _dftflow_dir="$_dftflow_bin/$_dftflow_family"
    case ":${PATH-}:" in
        *":$_dftflow_dir:"*) ;;
        *) PATH="$_dftflow_dir${PATH:+:$PATH}" ;;
    esac
done
export PATH
unset _dftflow_bin _dftflow_family _dftflow_dir
```

首次加载时，新加入的家族目录优先于原有 `PATH`，后续重复加载不重复添加；不把 `.build` 或测试目录加入搜索范围。已存在于原 `PATH` 中的家族目录保留原位置。此配置可直接粘贴到当前终端执行。

如需新终端持久生效，将上述同一配置段手动放入所用 shell 的启动文件一次：Bash 通常为 `~/.bashrc`，Zsh 为 `~/.zshrc`。Bash 登录 shell 还需其实际使用的 `~/.bash_profile`、`~/.bash_login` 或 `~/.profile` 加载同一配置或 `~/.bashrc`。新开对应交互终端，或在当前 shell 加载刚编辑的文件：

```bash
# Bash 用户加载自己编辑的配置文件
source "$HOME/.bashrc"
```

```zsh
# Zsh 用户加载自己编辑的配置文件
source "$HOME/.zshrc"
```

从仓库外核对解析位置及帮助启动（仅验证调用，不提交科学计算）：

```bash
(
    cd "/tmp" || exit 1
    command -v epw_phonon
    command -v qeflow
    command -v qe_scf
    command -v substitution
    epw_phonon --help
    qeflow --help
)
```

预期解析位置分别位于 `bin/epwflow/`、`bin/qeflow/`、`bin/substitutionflow/`。同样核对其余产品及兼容命令；出现旧安装或同名 alias/函数时，用 `type -a 命令名` 查找来源，Bash 可执行 `hash -r` 清除命令缓存。当前 shell 的环境仅被其后代继承；独立批处理/Slurm 脚本需明确加载配置或继续使用程序绝对路径。

### 本轮验证记录（2026-10-20）

从本文提取 PATH 配置，在仓库外临时目录分别使用不加载用户配置的 Bash（`--noprofile --norc`）和 Zsh（`-f`）运行；初始环境仅提供 `/usr/bin:/bin` 作为命令搜索路径，并使用临时 HOME 隔离用户配置。

- 两种 shell 各 55 个已安装产品及兼容入口全部通过 `command -v` 路径核验与 `--help`，均解析至对应家族目录。
- 配置连续加载两次后 `PATH` 完全一致；七个家族目录各出现一次，原 `/usr/bin:/bin` 保留。
- 本目录 7 篇文档的相对链接及 8 段 Bash/Zsh 示例语法检查通过。
- 没有更改真实 shell 启动文件。README 集成、新终端持久生效验证和其余构建/迁移任务仍由 Tasks 跟踪，本记录仅证明配置示例及当前安装入口在隔离 shell 下可用。
