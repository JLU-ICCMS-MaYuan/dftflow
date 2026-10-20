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
