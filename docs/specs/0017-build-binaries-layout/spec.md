# 统一 DFTFlow 编译可执行程序目录规格

**Issue**：[本地 0017 草案](issue-draft.md)（线上 Gitee Issue 尚未创建）

**创建日期**：2026-10-20

**状态**：已确认需求基线；本轮仅完成规划文档，尚未实施。

## 原始需求记录

> 我希望你能把所有编译好的可执行程序都放在/data/home/mayuan/code/dftflow/bin里面，并且按照目录分割好，比如 epwflow， qeflow， substitutionflow，vaspflow 等等。不要像 /data/home/mayuan/code/dftflow/test_auto/MgB2/local_run/010-public-pbe/bin-window-contrac这样放在测试目录下面。把这一条新建一个 issue ，spec 完成。

> 仅 DFTFlow 自身产物（推荐）

## 附件与推断来源

- 无附件，`images: []`。
- AI 推断：示例中的目录表示 DFTFlow 仓库根 `bin/` 下按程序家族组织；答复确认范围只包括 DFTFlow 自身，不包括外部科学软件。
- 当前代码证据：根 Cargo workspace、`scripts/build-native.sh`、`scripts/linux/build.sh`、`scripts/install-bin.sh`、`scripts/install-current.sh`、`tools/qf-gen-dyn0/Cargo.toml`、根 `.gitignore`、README 与 Overview。

## 背景与目标

主要 workspace 使用 Cargo 平台配置构建，默认产物目前在 `target/`；独立 `qf-gen-dyn0` 工程说明使用独立 `target/release/`。仓库已有 `scripts/install-bin.sh`，能将扁平的八个主二进制整理为家族目录，并可附加安装 dyn0 工具；根 README 已记录该分组布局。与此同时，Linux musl 构建产物在 `dist/.../bin`，EPW 测试运行目录可存在三份最终程序副本。

目标是统一 DFTFlow 对外可运行程序的仓库内构建位置和分组契约，使受支持构建产物都可从根 `bin/` 找到，不再把交付程序保存在算例/测试目录，并让安装、打包和测试脚本直接消费该契约。

### 需求结论与变更取舍

纳入用户最新确认的边界：只整理 DFTFlow 自身交付程序。最终程序按家族放在 `bin/<family>/`；Cargo 编译缓存及测试 harness 可留在 `bin/.build/` 等专用区，不作为家族产品。测试目录允许输入、日志和结果，不保存 DFTFlow 可交付二进制。外部 QE/VASP/Wannier90 求解器不属于 DFTFlow 构建产物。

现有 `install-bin.sh` 已提供分组复制、启动核验和失败保护，Plan 以复用为主；安装后的 `current/bin` 维持现有契约，Linux 发布包继续保留独立发行目录，但其来源从规范构建产物生成。

## 用户场景与验收

### US1：按家族查找构建程序（优先级：P1）

开发者完成仓库支持的构建后，能在根 `bin/` 对应家族目录中找到可直接运行的程序，无需到 Cargo 默认目录或测试算例目录寻找、复制。

**优先级理由**：这是用户明确提出的主要结果，且能独立验收。

**独立验收**：执行构建入口后逐项检查家族路径、可执行权限和 `--help`；再次构建后路径集合稳定。

**验收场景**：

1. **假如** workspace 与独立工具依赖就绪，**当**运行相应构建入口，**那么**所有受支持 DFTFlow 交付程序都位于 `bin/<family>/`。
2. **假如** EPW 的三个阶段程序已构建，**当**检查交付目录，**那么**三个程序都在 `bin/epwflow/`。
3. **假如**构建已执行过一次，**当**再次构建相同目标，**那么**程序仍写入相同路径并覆盖旧内容，不产生新编号目录。

### US2：测试目录只存放测试数据与证据（优先级：P1）

开发者在编号算例目录运行脚本时，脚本从仓库规范 `bin/` 使用 DFTFlow 程序；算例目录不承担二进制仓库。

**优先级理由**：用户明确要求不再把程序放在 `test_auto` 计算目录。

**独立验收**：盘点受支持测试目录的 DFTFlow 可交付程序并核对活动引用；不存在依赖旧副本的通过路径。

**验收场景**：

1. **假如**测试算例目录有 DFTFlow 调用脚本，**当**从该目录按文档启动测试，**那么**脚本解析到根 `bin/<family>/` 中的程序。
2. **假如**测试路径存在历史 DFTFlow 二进制副本，**当**迁移完成，**那么**副本移出测试目录且脚本不再引用旧路径。

## 边界与异常场景

- 未构建、缺失或不可执行的交付程序必须使安装/打包在改变现有安装内容前失败，并指出缺失家族和程序。
- 家族目录路径冲突为文件、链接或不可写时，构建/整理脚本应报告明确错误，不静默写入测试目录或扁平备用目录。
- Cargo 缓存和测试 harness 可存在于 `bin/` 下构建专用目录；它们不计入公开程序清单和家族程序数量。
- Linux `dist` 是发行包目录，不是仓库常规构建源目录；包中可继续使用发行格式，但来源必须绑定本次成功构建的根 `bin/` 程序。
- 安装器重复执行必须继续覆盖固定安装位置，不能因新增构建目录创建时间戳安装历史。

## 功能需求

- **FR-001**：所有受支持的 DFTFlow 构建入口必须将仓库内 Cargo target/cache 输出置于根 `bin/` 内的构建专用区域；Linux 容器可使用专用挂载缓存，但不得将其作为仓库交付位置或写入算例目录。
- **FR-002**：构建完成后必须把可交付程序稳定整理到 `bin/epwflow/`、`bin/qeflow/`、`bin/substitutionflow/`、`bin/vaspflow/`、`bin/wannierflow/`、`bin/scflow/` 与 `bin/tools/`。
- **FR-003**：程序映射必须为：`epw_phonon`、`epw_wannier`、`epw_superconductivity`→`epwflow`；`qeflow`→`qeflow`；`substitution`→`substitutionflow`；`vaspflow`→`vaspflow`；`wannier90`→`wannierflow`；`scflow`→`scflow`；独立 `qf-gen-dyn0`→`tools`。
- **FR-004**：构建、固定安装、分组安装、兼容入口与 Linux 打包脚本必须从新的家族目录或由其产生的明确构建快照读取程序，不得依赖测试目录中的二进制副本。兼容命令软链接可按现有映射同组生成。
- **FR-005**：仓库测试与验收脚本调用 DFTFlow 程序时必须从仓库根 `bin/` 解析路径；测试 harness、探针和临时工具不要求纳入产品家族目录。
- **FR-006**：已发现的测试目录 DFTFlow 交付程序副本必须迁出，相关脚本和手册路径同步更新；修改前核对活动 Slurm 作业、生成脚本和数据来源引用，保留运行结果及必要的历史证据。
- **FR-007**：`bin/` 作为本机生成产物目录继续被 Git 忽略；不提交本机可执行文件、Cargo 缓存或运行库。
- **FR-008**：支持重复构建覆盖稳定路径；任何失败都不得退回到算例目录保存程序，也不得破坏已安装固定版本。

## 成功标准

- **SC-001**：受支持 workspace 构建生成的八个主程序均位于其指定家族目录；独立工具构建生成 `bin/tools/qf-gen-dyn0`。所有清单项均可执行且 `--help` 成功。
- **SC-002**：根 `bin/` 下全部唯一可执行程序均属于明确产品清单；现有兼容命令软链接按家族归档；测试 harness/缓存若存在，只位于构建专用子目录。
- **SC-003**：受检查的 `test_auto` 目录没有 DFTFlow 交付二进制，所有活动脚本对程序的引用均解析至仓库 `bin/`。
- **SC-004**：workspace 和独立工具测试通过；固定安装和 grouped install 的缺失输入、重复更新及启动检查通过；Linux 发布包离线验证通过。
- **SC-005**：连续两次构建的交付相对路径集合完全一致，程序更新后 SHA-256 对应当前构建，未产生按算例或时间新增的程序目录。

## 假设与依赖

- 低影响推断：`bin/.build/` 用作 Cargo 内部输出区，家族目录只放可交付程序；具体命名可在 Plan 中按 Cargo/平台配置验证，不能改变最终根目录与家族契约。
- 受支持构建入口以 `scripts/build-native.sh`、`scripts/linux/build.sh` 和独立工具构建说明为基线；实际搜索若发现其他可执行构建入口，须纳入清单。
- 复用 `scripts/install-bin.sh` 已有的家族映射和验证逻辑；不改变 `~/.local/share/dftflow/current/bin` 与 `~/.local/bin` 的默认位置。

## 待确认决策

无。用户已确认范围只包括 DFTFlow 自身产物，且确认本 Issue/Spec 草案需求基线。

## 范围外事项

- QE、VASP、Wannier90 等外部程序与系统级共享软件目录。
- 测试 harness、临时探针及第三方 Cargo 构建脚本的文件名归属。
- 科学计算逻辑、输入/结果数据路径、资源预算、安装默认值或线上仓库写操作。
- 自动删除历史测试产物之外的用户数据；每个待迁移文件均须检查来源与引用。

## 澄清记录

- 问：Issue/Spec 中“所有编译好的可执行程序”按哪种范围定义？
- 答复原文（已追加至原始需求记录）：`仅 DFTFlow 自身产物（推荐）`。
- 结论：外部科学软件排除；公开 DFTFlow 产品纳入；测试 harness 与临时探针不作为产品家族输出，但构建输出根仍受 FR-001 约束。

## 最终草案确认

用户答复“确认，按此草案继续”，确认了展示的 Issue 草案与 Spec 需求基线，授权建立完整 Spec 文档。Spec 设计细节在 Plan/Research 中展开；本确认不授权实现代码、迁移二进制或切换安装。
