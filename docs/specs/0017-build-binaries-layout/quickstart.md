# 0017：构建、直接调用与迁移验证

## 日常构建

在仓库根目录执行；原生开发依赖、Rust/Cargo 与 `jq` 需就绪。Linux 使用私有兼容前缀时还需 `patchelf`。

```bash
# 主 workspace 八个产品，成功后已发布到 bin/<模块>/
bash scripts/build-native.sh
# 仅更新 EPW 三程序
bash scripts/build-native.sh build --release --locked -p epwflow --bins
# 独立 dyn0 工程，同一入口发布到 bin/tools/
bash scripts/build-native.sh build --release --locked --manifest-path tools/qf-gen-dyn0/Cargo.toml
```

原始 Cargo 输出与缓存位于 `bin/.build/<平台>/`，本机依赖位于 `bin/.native/<平台>/`。交付构建读取本次制品报告，完成必要兼容打包及全部选中入口启动验证后更新公开目录；不用再安装到用户目录。直接执行裸 `cargo build` 不经过发布步骤，日常使用上述 wrapper。显式交叉目标只构建，不替换宿主程序。

发布来源记录在 `bin/.build/published/<程序>.json`，包含原始 Cargo 制品路径及原始、发布后 SHA-256。私有依赖和缓存不加入 PATH；发布前发现本机进程或 Slurm 脚本仍引用选中产品时会失败并保留原程序，待活动任务结束后重新构建即可。

## 从任意目录调用

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
hash -r
command -v epw_phonon qeflow qe_scf qf-gen-dyn0
```

其他仓库位置替换为实际绝对路径。加载器支持 Bash/Zsh，七家族路径始终前置且重复加载幂等，保留其他系统路径。Bash 可用 `hash -r` 清除之前缓存的命令位置；Zsh 可用 `rehash`。

新终端需由用户将相同 source 行加入实际使用的 `~/.bashrc` 或 `~/.zshrc`；Bash 登录文件应加载相应配置。本需求按原约定不自动编辑真实启动文件；已经打开的其他终端需要各自加载一次。独立批处理环境也应显式 source 或使用产品绝对路径。

## 测试与检查

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --workspace --locked --no-fail-fast
bash scripts/build-native.sh test --locked --manifest-path tools/qf-gen-dyn0/Cargo.toml
bash scripts/tests/build-publish.sh
bash scripts/tests/env.sh
bash scripts/tests/install-current.sh
```

`test`、`check` 和仅构建库不会发布日常 CLI。自动发布回归用隔离的实际 Cargo 工程验证，不提交科学作业；环境回归在隔离 Bash/Zsh 中检查全部真实入口、重复加载、含空格路径及新终端。

## 本机迁移与历史

旧 `install-current.sh` 已退役，运行时明确失败且不创建旧安装。旧依赖、用户入口、测试目录历史二进制的迁移与删除结果以 [验收记录](acceptance.md) 为准，源码实现不等同于所有机器已完成迁移。

本机迁移先将依赖复制到仓库私有前缀，更新 `.pc`／CMake／ELF／环境文件等实际引用；以旧路径不可访问时的构建和启动验证为删除前提，最后清理自有入口和旧根目录。不能跳过活动任务核验直接删除加载器，也不能重写冻结回执冒充历史身份。

历史二进制按原哈希保存在 `bin/.history/0017/` 并另记原路径清单。它们不是日常入口；旧冻结记录保持原文，通过迁移清单定位原始证据。继续工作应使用现有新尝试或导入契约，而非修改旧身份。旧程序若硬编码旧加载器，仅保留字节不代表其原路径仍可直接执行。

## 平台验证边界

Linux musl 发行链经 `scripts/linux/build.sh` 在 Docker 中构建和离线验证，制品快照位于 `bin/.build/linux-x86_64-musl/package-source/`，发行包仍在 `dist/`。交叉发行不覆盖宿主程序。没有 Docker 的机器不能把脚本语法检查当作真实 musl 构建通过；macOS 的当前实测状态同样在验收中单列。
