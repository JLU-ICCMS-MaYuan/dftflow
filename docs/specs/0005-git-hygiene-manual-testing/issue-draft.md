# Issue 草稿：0005 Git 卫生治理与手动计算操作文档

> 本文件为线上 Gitee Issue 草稿，由用户自行创建（编号确认后回填链接）。
> 类型标签：`type-feature`；优先级：`P0`。

## 标题

Git 卫生治理：移出测试运行输出、机器路径本地化、子模块入库推送与手动计算操作文档

## 原始需求（用户原话，2026-10-13）

> 请帮我处理当前所有尚未提交到 Git 的文件，并基于 issue 和 spec 的方式来实现。具体要求如下：编译生成的临时文件不要加入 Git 版本控制；测试产生的输出文件不需要保留；各个远程机器的登录情况和软件配置情况不需要提交到 Git，只需记录在本地；测试用的输入文件、需要输入的命令和操作步骤都要完整保留并写入文档，方便我后续手动计算时进行查对。请给出满足以上条件的处理方案和需要执行的步骤。

### 2026-10-14 补充需求（用户原话）

> 你来帮我处理一下这个分支没有提交的代码。以 issue0005 完成

> 你说的 vasp 本地备份都有哪些文件？

> 也一起提交到对应的 vasp 的那个仓库的远程把

### 2026-10-14 单文件历史清理授权（用户原话）

> 是否允许我清理 `rust` 分支历史中的这个文件，保留本地文件，再重新推送？允许清理这个文件

### 2026-10-14 保留本地文件并停止跟踪（用户原话）

> 暂时不需要删除这些文件，但是保证不用 git 跟踪

用户同时引用了缓存、编辑器临时文件、Rust 覆盖率忽略规则，以及空 `.codex` 占位文件和 105 个历史运行文件的核查清单。

## 背景与问题

Issue 0004 完成全量迁移收尾后遗留四类 Git 卫生问题：

1. `test_auto/MgB2/local_run/` 下 113 个真实 VASP/lobster 运行输出（166MB）被误跟踪上传；
2. `.gitignore` 缺 `.pytest_cache/`、`docs/local/` 等项；
3. `test_auto/MgB2/input_*.toml` 与 `docs/qe-environment.md` 硬编码 Mac 本机与集群软件路径；
4. 子模块 `external/vasp.6.4.3`（本机构建配置）与 `external/q-e`（macOS 内存补丁分支）存在未入库的本地改动。

## 方案要点

- local_run 运行输出 `git rm --cached` 移出，保留本地，`.gitignore` 白名单放行输入/脚本；
- 输入文件路径改 `@NAME@` 占位符，真实路径存 gitignored 的 `docs/local/paths.toml`，`conftest.py` 拷贝时自动替换；
- vasp.6.4.3 提交 makefile.include + BUILD_MACOS.md 推私有 Gitee；q-e fork（madegan/q-e）先同步上游 QEF/q-e 再推 `macos-memusage-fix`；
- 新建 `docs/manual-testing.md` 完整记录测试输入、命令序列与操作步骤。

## 验收标准

见 `spec.md` SC-001 ~ SC-006 与 `acceptance.md`。

## 关联

- Spec：[docs/specs/0005-git-hygiene-manual-testing/](./spec.md)（线上双向链接待 Issue 建立后回填）
- 关联 Issue：0004（[IKKGLP](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKKGLP)）收尾后的卫生治理

## 2026-10-14 补充收尾

- 按用户补充授权，将 VASP 的 `makefile.include.linux-aocl.bak` 原样提交并推送私有仓库，作为此前备份排除及机器路径本地化要求的单文件例外。
- 父仓库同步子模块指针；验收以备份 blob 一致、远程 master 指向及递归工作区状态为依据。结果见 acceptance.md。

## Documentation Impact

- 更新本目录 Spec、Tasks、Acceptance 与本地 Issue 草稿，以及 AGENTS.md 需求记录。
- 不影响计算功能、CLI 或安装操作，README 与功能 Overview 无需修改。

## 2026-10-14 单文件历史清理结果

用户授权仅移除 rust 历史中的 `test_auto/MgB2/local_run/vasp_cohp_uniform/WAVECAR`，保留本地文件；清理已完成并成功推送 rust，设置跟踪 origin/rust。其他运行输出历史未清理，线上 Issue 状态仍由用户自管。

## 2026-10-14 保留本地并忽略

补齐用户指定的忽略规则；空 `.codex` 取消跟踪但保留磁盘，未来同名目录可跟踪。105 个历史运行输出继续本地保留、当前不跟踪，不进一步改写历史。验证见 acceptance.md，任务见 T011。
