# 实施任务：统一 DFTFlow 构建可执行程序目录

**输入**：[spec.md](spec.md)、[plan.md](plan.md)、[research.md](research.md)

## 阶段 1：准备与契约

**目的**：形成可验证的唯一构建/交付清单，查明现有运行引用，避免破坏历史验收环境。

- [ ] T001 [US1] 盘点全部受支持 Cargo 构建入口、输出位置、二进制清单及平台配置，记录于本 Spec 的 `research.md`。
- [ ] T002 [US2] 只读核验测试目录 DFTFlow 可执行文件的哈希、脚本/调度器引用、进程与可复建来源，记录于验收证据。
- [ ] T003 [US1] 定义唯一家族映射及 staging 接口，确认与 `scripts/install-bin.sh` 的既有契约一致。

## 阶段 2：用户故事 1——统一家族构建输出（P1）

**目标**：所有公开 DFTFlow 程序由受支持构建入口生成到根 `bin/` 家族路径。

**独立验收**：空缓存构建后清单所有程序可执行且 `--help` 通过；第二次构建不改变路径集合。

### 实施

- [ ] T004 [US1] 更新 `scripts/build-native.sh`，把 Cargo target/cache 定向到仓库 `bin/.build/` 并在成功后归档产品。
- [ ] T005 [US1] 适配 `tools/qf-gen-dyn0` 独立 Cargo 构建及其 README，使产物落入 `bin/tools/`。
- [ ] T006 [US1] 适配 `scripts/install-current.sh` 及其入口/隔离测试，从家族目录或显式 staging 取得八个主程序，同时保持固定安装目的地。
- [ ] T007 [US1] 适配 `scripts/linux/build.sh` 和 `scripts/linux/package-native-runtime.sh`，让 musl 发布包来源绑定规范产物并保持发行验证。
- [ ] T008 [US1] 扩展 `scripts/tests/install-bin.sh` 覆盖规范家族输入、重复覆盖、缺失/损坏源和失败保护；复用 `scripts/install-bin.sh` 映射。

## 阶段 3：用户故事 2——移除测试目录程序副本（P1）

**目标**：测试脚本通过仓库 `bin/` 运行程序，算例目录只保存输入、日志和结果。

**独立验收**：运行相关测试时无需在 `test_auto` 放置 DFTFlow 二进制；旧路径扫描无活动引用。

### 实施

- [ ] T009 [US2] 核验完成后将 `test_auto/MgB2/local_run/010-public-pbe/bin-window-contract/` 的 EPW 三程序迁出到 `bin/epwflow/`，按记录确认仅移除这些副本且来源数据不变。
- [ ] T010 [US2] 更新 `test_auto/MgB2/epwflow/` 与本地可跟踪测试脚本中的二进制路径，保持忽略目录和历史输入边界。
- [ ] T011 [US2] 更新 `README.md`、`docs/overview/dftflow-rust.md`、`docs/overview/tools/qf-gen-dyn0.md`、`tools/qf-gen-dyn0/README.md` 与 `docs/manual-testing.md` 的构建路径说明。

## 最终阶段：质量门与收敛

- [ ] T012 [US1] 在空 target/cache 下运行 workspace 和独立工具构建、测试及全部公开程序 `--help`。
- [ ] T013 [US1] 连续运行两次构建，比较家族路径集合、权限与 SHA-256，验证旧程序被稳定路径覆盖。
- [ ] T014 [US2] 执行测试目录可执行文件清点、旧路径活动引用扫描、所有修改脚本 `bash -n` 与测试回归。
- [ ] T015 [US1] 运行固定安装隔离测试、分组安装验证及 Linux musl 打包/离线验证，检查 `bin/` 仍被忽略且工作树仅含本任务改动。

## 依赖与执行顺序

- T001–T003 是基础依赖；T004/T005 建立程序来源后，T006–T008 才能适配消费方。
- T009 必须在 T002 证实无活动引用/作业且程序来源可复建后执行；T010 依赖迁移完成。
- T011 可与 T009/T010 分不同文件并行；最终 T012–T015 在所有实现和引用更新后执行。

## 需求覆盖

| 来源 | 任务 | 说明 |
| --- | --- | --- |
| FR-001 / SC-002 | T001、T004、T012 | Cargo 缓存根与路径验证 |
| FR-002/003 / SC-001 | T003–T005、T012–T013 | 家族清单和启动验证 |
| FR-004 / SC-004 | T006–T008、T015 | 安装与发行包消费新来源 |
| FR-005/006 / SC-003 | T002、T009–T011、T014 | 安全迁移与引用回归 |
| FR-007/008 / SC-005 | T004、T013、T015 | 忽略、覆盖和无历史累积 |

## MVP 与增量策略

1. 先完成 workspace/独立工具构建及产品归档，并验证所有入口。
2. 再适配安装和 Linux 打包消费者。
3. 最后迁移测试目录副本、更新文档并完成全量交叉验证。
