# Feature 规格：Git 卫生治理与手动计算操作文档

**Issue**：0005（线上 Issue 待用户自建，本地编号 0005）
**创建日期**：2026-10-13
**状态**：既有治理及 2026-10-14 备份入库补充收尾完成（线上 Issue 由用户自管）

## 背景与目标

仓库完成 Issue 0004 全量迁移收尾（提交 `beb91c9`）后，Git 跟踪状态存在四类遗留问题：
测试运行输出（166MB）被误跟踪、`.gitignore` 缺失部分构建产物、测试输入文件中硬编码
机器路径（Mac 本机与集群）、子模块存在未提交的本地改动。本 Feature 一次性治理上述问题，
并把测试所需的输入文件、命令与操作步骤完整写入文档，方便用户后续手动计算查对。

## 原始需求记录

### 2026-10-13 用户原话

> 请帮我处理当前所有尚未提交到 Git 的文件，并基于 issue 和 spec 的方式来实现。具体要求如下：编译生成的临时文件不要加入 Git 版本控制；测试产生的输出文件不需要保留；各个远程机器的登录情况和软件配置情况不需要提交到 Git，只需记录在本地；测试用的输入文件、需要输入的命令和操作步骤都要完整保留并写入文档，方便我后续手动计算时进行查对。请给出满足以上条件的处理方案和需要执行的步骤。

### 澄清答复（2026-10-13，逐字追加）

1. 问「local_run 下 113 个（166MB）已被 Git 跟踪的真实运行输出文件如何处理」，用户答：
   > 移出 Git 保留本地(仅 git rm --cached，文件留在本地磁盘不上传，配合 .gitignore 防止再次进入)
2. 问「两个子模块的本地改动如何处理」，用户答：
   > vasp 推到我的私有仓库，q-e 我记得我 fock 了官方仓库，有自己的仓库，我仅仅需要你推到我自己的仓库就行。
3. 问「远程机器登录与软件配置不入 Git 的范围」，用户答：
   > 是，路径也算(提交不含机器信息的通用版本，真实路径写到本地-only 文件（如 docs/local/ 并加入 .gitignore）)
4. 问「本次工作按项目惯例以新 Feature Spec 管理，编号用 0005」，用户答：
   > 0005 新 Spec(新建 docs/specs/0005-git-hygiene-manual-testing/，含 issue-draft、spec、acceptance、tasks 全套)
5. 问「q-e 本地提交推到哪里」，用户答：
   > 提供 fork 地址 https://gitlab.com/madegan/q-e 。但是我不清楚有没有 gitlab 的 CLI 可以让你使用。而且我很久没有更新我 fork 的gitlab 仓库了，你需要先更新
6. 问「vasp.6.4.3 推送到私有 Gitee 仓库时提交哪些内容」，用户答：
   > 含构建笔记(makefile.include 修改与 BUILD_MACOS.md 构建笔记一起提交并推送到私有 Gitee 仓库，.bak 备份文件不提交)
7. 问「input_*.toml 中的软件路径如何处理」，用户答：
   > 占位符+自动注入(input_*.toml 用占位符提交，真实路径存 docs/local/paths.toml（不入 Git），conftest.py 拷贝时自动替换，本地 pytest 照常能跑)

### 2026-10-14 补充需求（用户原话）

> 你来帮我处理一下这个分支没有提交的代码。以 issue0005 完成

> 你说的 vasp 本地备份都有哪些文件？

> 也一起提交到对应的 vasp 的那个仓库的远程把

### 2026-10-14 单文件历史清理授权（用户原话）

> 是否允许我清理 `rust` 分支历史中的这个文件，保留本地文件，再重新推送？允许清理这个文件

### 2026-10-14 保留本地文件并停止跟踪（用户原话）

> 暂时不需要删除这些文件，但是保证不用 git 跟踪

用户同时引用了缓存、编辑器临时文件、Rust 覆盖率忽略规则，以及空 `.codex` 占位文件和 105 个历史运行文件的核查清单。

## 用户场景与验收

### US1：构建与测试产物不入 Git（P0）

开发者构建（`target/`、`__pycache__`、`.pytest_cache`）与跑测试产生的运行输出
（`test_auto/MgB2/MgB2_calc/`、`local_run/` 运行产物）均不出现在 `git status` 未跟踪列表中；
已被跟踪的 166MB local_run 运行输出从索引移除但保留在磁盘。

独立验收：`git status --porcelain` 无未跟踪的构建/运行产物；`git ls-files test_auto/MgB2/local_run` 只剩输入/脚本类文件。

### US2：机器信息仅存本地（P0）

仓库内任何已跟踪文件不含 Mac 本机路径（`/Users/mayuan/...`）与集群路径
（`/work/home/mayuan/...`、`/home/mayuan/...`）；真实路径集中写在 gitignored 的
`docs/local/paths.toml`；本地 pytest 因 conftest 自动替换占位符而照常可跑。

独立验收：对已跟踪文件全文检索机器路径无命中；pytest 收集正常、占位符替换后输入文件含真实路径。

### US3：子模块本地改动入库到用户自有仓库（P1）

- `external/vasp.6.4.3`：`makefile.include` 修改与 `BUILD_MACOS.md` 构建笔记提交并推送到
  私有 Gitee 仓库 `git@gitee.com:mayuan_JLUPHY/vasp.6.4.3.git`；按 2026-10-14 补充授权，
  `makefile.include.linux-aocl.bak` 原样入库并推送，作为备份不提交规则的单文件例外。
- `external/q-e`：先从官方上游 `QEF/q-e` 同步用户 fork `https://gitlab.com/madegan/q-e`，
  再把本地分支 `macos-memusage-fix`（提交 `51fdfbd13`）推送到 fork；嵌套子模块
  `external/mbd` 指针变动与 `.DS_Store` 不提交，仅本地留档。
- 父仓库随后提交两个子模块的新指针。

### US4：手动计算操作文档（P0）

新建手动计算操作手册，完整收录：测试输入文件位置与内容说明、需要执行的命令序列
（pytest、各 flow CLI）、操作步骤与注意事项，供用户后续手动计算查对。

## 功能需求

- **FR-001**：`.gitignore` 补齐 `.pytest_cache/`、`docs/local/`、`*.bak`、local_run 输出忽略规则。
- **FR-002**：`git rm --cached` 移出 `test_auto/MgB2/local_run/` 全部运行输出，文件保留磁盘；
  输入/脚本类文件（`input_vasp_local.toml`、`input_vasp_cohp_local.toml`、`input_vasp_smoke.toml`、
  `run_vasp_scf.sh`、`run_vasp_cohp_uniform.sh`、`MgB2.vasp`、`potcar_lib/`）保留跟踪。
- **FR-003**：显式放行并跟踪 `test_auto/MgB2/scflow/validate_scflow_isif4.py`（测试脚本）。
- **FR-004**：`input_qe.toml`、`input_vasp.toml`、`input_w90.toml` 及 local_run 下本地变体输入
  中的机器路径改为 `@NAME@` 占位符；占位符清单集中定义。
- **FR-005**：新建 `docs/local/paths.toml`（不入 Git）保存真实路径；`conftest.py` 在拷贝输入
  文件时读取该文件并替换占位符；文件缺失时给出明确提示并跳过替换（保持可收集）。
- **FR-006**：`docs/qe-environment.md` 改写为通用版本，真实路径引用 `docs/local/` 说明。
- **FR-007**：子模块 vasp.6.4.3 提交并推送（含构建笔记，以及补充授权的 `makefile.include.linux-aocl.bak`）；q-e 同步上游 fork 后
  推送 `macos-memusage-fix` 分支；父仓库提交新子模块指针。
- **FR-008**：新建手动计算操作文档（含输入文件、命令、步骤）；`docs/local/` 下提供远程机器
  登录与软件配置的本地留档模板。
- **FR-009**：`AGENTS.md` 追加本需求处理记录；0005 Spec 全套文档（issue-draft/spec/tasks/acceptance）建立。

## 成功标准

- **SC-001**：`git status --porcelain` 中无未跟踪的构建/测试产物；local_run 仅剩输入/脚本类被跟踪文件。
- **SC-002**：对全部已跟踪文件检索 `/Users/mayuan`、`/work/home/mayuan`、`/home/mayuan` 无命中。
- **SC-003**：`pytest --collect-only` 正常；冒烟运行至少一个非 `run` 测试通过，替换后的输入文件含真实路径。
- **SC-004**：vasp.6.4.3 推送后其 origin/master 含 makefile.include、BUILD_MACOS.md
  与原样保存的 makefile.include.linux-aocl.bak；
  q-e fork 上存在 `macos-memusage-fix` 分支且含 `51fdfbd13`。
- **SC-005**：手动计算操作文档存在且覆盖 test_auto/MgB2 全部可用输入与命令序列。
- **SC-006**：父仓库按规范提交，工作区干净（子模块无未提交的需入库改动）。

## 假设与依赖

- `madegan` 为用户本人的 GitLab 账号，`https://gitlab.com/madegan/q-e` 是用户 fork（origin 已指向它）。
- GitLab 无通用官方 CLI；若 `glab` 未安装则用纯 git 命令完成 fork 同步与推送。
- 推送需要用户已配置的 Gitee SSH / GitLab HTTPS 凭据；凭据交互失败时留档停止，不伪造推送成功。
- `docs/local/` 内文件属于用户本机资料，永不提交。

## 范围外事项

- 不修改任何 Rust/Python 计算逻辑、不动 pytest 测试用例本身。
- 不操作线上 Gitee Issue（0005 由用户自建）。
- `potcar_lib/`（VASP 赝势）继续按现状跟踪，本 Feature 不改其跟踪状态。
- q-e 嵌套子模块 `external/mbd` 的指针变动不提交（用户无 libmbd fork），仅文档留档。
- 默认不做 Git 历史改写；2026-10-14 用户明确授权的唯一例外为从 `rust` 历史移除
  `test_auto/MgB2/local_run/vasp_cohp_uniform/WAVECAR`，保留本地文件并重新推送。其余运行输出历史不清理。

## 澄清记录

### 2026-10-13

- 用户确认采用「占位符 + conftest 自动注入」方案处理输入文件机器路径（见原始需求记录第 7 条）。
- 用户确认 local_run 运行输出移出 Git 但保留本地（第 1 条）；子模块改动推送到用户自有仓库（第 2、5、6 条）。
- 「机器信息不入 Git」范围按用户第 3 条答复执行：路径也算，真实路径仅存 `docs/local/`。

### 2026-10-13（实施期范围细化，不改变用户需求）

- SC-002 的检索范围限定为**活文档与配置/输入类文件**；以下三类保留机器路径不改写：
  1. `AGENTS.md` 与 `docs/specs/0001~0003` 内的历史验收记录（项目规范要求历史条目不改写）；
  2. `qeflow/tests/fixtures/golden/` 冻结黄金基线（改动即破坏逐字节对照）；
  3. 测试断言改为从 `docs/local/paths.toml` 读取期望值（`test_vaspflow.py` 三处），
     合成测试串（`test_execution_semantics.py`、`test_qeflow.py`、`test_qe_phonon.py` 的
     pyproject 硬编码路径）改为中性字符串/相对定位——此为满足 FR-004 的最小测试断言调整，
     不改变测试语义。
- `docs/overview/qe-install-m5.md` 属机器软件配置文档，移至 `docs/local/` 本地留档，
  总览索引留说明条目（用户需求第 3 条「软件配置只需记录在本地」）。

### 2026-10-14（备份入库例外）

- 用户在获知唯一未提交项为 VASP Linux/AOCC/AOCL 编译配置备份后，明确要求一起提交到 VASP 远程仓库。
- 此要求覆盖此前该文件不提交的约定；仅此文件原样保留软件路径并推送用户私有 VASP 仓库，父仓库只记录子模块指针，不复制备份内容。其他机器信息仍按原规则仅存本地。
- 本次只处理备份、子模块指针和 0005 追踪记录，不修改计算逻辑，不运行 VASP，不操作线上 Issue；父仓库提交到现有 qeflow-rust 分支。

### 2026-10-14（单文件历史清理）

- Gitee 拒绝超过 100 MB 的历史对象，用户明确授权仅清理上述 WAVECAR 并重新推送。
- 本次不采用此前建议的 105 个运行文件批量清理，不扩大 .gitignore，也不改动其他分支、标签或子模块仓库。
- 历史重写前保留本地备份引用；当前代码树、本地 WAVECAR 内容及其他 refs 必须不变，清理后 rust 可达历史不得再包含目标文件或 blob。

### 2026-10-14（仅保留本地并忽略）

- 补齐用户引用的 Python 工具缓存/覆盖率、编辑器/macOS 临时文件、Rust 备份/覆盖率忽略规则。
- 根目录空 `.codex` 仅取消跟踪、保留磁盘；规则只忽略该占位文件，未来 `.codex/` 共享配置目录不忽略。
- 105 个历史 local_run 运行输出均维持当前不跟踪、磁盘保留的状态；8 个输入/脚本/赝势文件继续跟踪。此前已授权清理的单个 WAVECAR 历史不恢复，其余历史不继续改写。
- 验收：运行输出及占位文件未被索引跟踪且均命中忽略规则，本地内容 SHA-256 不变，输入白名单保持原样。
