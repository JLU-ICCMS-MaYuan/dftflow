# Tasks：0005 Git 卫生治理与手动计算操作文档

对应 `spec.md` 的 FR/SC。任务按依赖顺序排列，实施完成并验证后勾选。

## T001 建立 Spec 全套文档（FR-009）

- [x] 新建 `docs/specs/0005-git-hygiene-manual-testing/`：issue-draft.md、spec.md、tasks.md、acceptance.md。
- 证据：本目录四个文件已创建，spec.md 含原始需求逐字记录。

## T002 补齐 .gitignore（FR-001）

- [x] 新增忽略项：`.pytest_cache/`、`docs/local/`、`*.bak`、`.DS_Store`（已有确认）、local_run 输出忽略。
- [x] local_run 采用「先忽略再白名单放行输入/脚本」规则。

## T003 untrack 测试运行输出（FR-002、FR-003、SC-001）

- [x] `git rm -r --cached test_auto/MgB2/local_run`，再 `git add` 放行输入/脚本类文件与 `potcar_lib/`（113 → 8 个被跟踪文件）。
- [x] 放行 `test_auto/MgB2/scflow/validate_scflow_isif4.py` 并跟踪。
- [x] 确认 `git ls-files test_auto/MgB2/local_run` 仅剩输入/脚本类文件，磁盘文件未删除。

## T004 机器路径占位符化与本地注入（FR-004、FR-005、FR-006、SC-002、SC-003）

- [x] `input_qe.toml`、`input_vasp.toml`、`input_w90.toml` 及 local_run 输入变体中机器路径改为 `@NAME@` 占位符（`qeflow/examples/input.toml` 通用化）。
- [x] 新建 `docs/local/paths.toml`（不入 Git）保存真实路径与占位符键清单。
- [x] `conftest.py` 增加占位符替换：拷贝输入文件时读 `docs/local/paths.toml` 替换；缺文件时提示并跳过（`_local_paths.py` 注入模块）。
- [x] `docs/qe-environment.md` 通用化改写；`docs/overview/qe-install-m5.md` 移入 `docs/local/`（机器软件配置本地留档）。
- [x] 全库检索已跟踪文件确认无 `/Users/mayuan`、`/work/home/mayuan`、`/home/mayuan` 残留（历史记录与冻结 golden 基线按 spec 澄清保留）。
- [x] 测试断言调整：`test_vaspflow.py` 三处改为读 `docs/local/paths.toml`；`test_execution_semantics.py`/`test_qeflow.py` 合成串中性化；`test_qe_phonon.py` pyproject 路径改相对定位。

## T005 手动计算操作文档（FR-008、SC-005）

- [x] 新建 `docs/manual-testing.md`：输入文件位置与说明、pytest 命令序列、Rust CLI 构建与旧名入口、本机/集群手动计算步骤。
- [x] 新建 `docs/local/remote-machines.md` 模板：远程机器登录方式与软件配置本地留档（不入 Git）。

## T006 子模块 vasp.6.4.3 入库推送（FR-007、SC-004）

- [x] 子模块内提交 `makefile.include` 修改与 `BUILD_MACOS.md`（排除 `*.bak`），提交 `7a82340`。
- [x] 推送到 `git@gitee.com:mayuan_JLUPHY/vasp.6.4.3.git`（master `6e06f68..7a82340`）。
- [x] 验证 origin HEAD 含上述两个文件（push 输出确认 fast-forward）。

## T007 子模块 q-e fork 同步与分支推送（FR-007、SC-004）

- [x] 为 q-e 添加 upstream `https://gitlab.com/QEF/q-e.git` 并 fetch（upstream/master = `ee7f72e8f`）。
- [ ] 将 fork `master` 同步到上游最新：**被阻断**——HTTPS 非交互推送无凭据、SSH 公钥未注册、钥匙串无凭据，留档待用户提供 PAT 或自行推送。
- [ ] 推送本地分支 `macos-memusage-fix`（含提交 `51fdfbd13`）到 origin：**被阻断**（同上）。
- [x] 嵌套子模块 `external/mbd` 指针变动与 `.DS_Store` 不提交，本地留档。

## T008 验证与提交（SC-001~SC-006）

- [x] `git status --porcelain` 无未跟踪构建/测试产物；pytest 收集正常（121 项；5 个收集错误为 P5 删除 Python 包的既有环境性问题）；conftest 注入冒烟测试通过（临时测试验证后删除）。
- [x] 父仓库提交子模块新指针与全部变更（AGENTS.md 规范格式）。
- [x] 更新 `AGENTS.md` 需求处理记录。
