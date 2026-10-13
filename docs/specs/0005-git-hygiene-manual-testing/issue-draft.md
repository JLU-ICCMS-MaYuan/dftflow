# Issue 草稿：0005 Git 卫生治理与手动计算操作文档

> 本文件为线上 Gitee Issue 草稿，由用户自行创建（编号确认后回填链接）。
> 类型标签：`type-feature`；优先级：`P0`。

## 标题

Git 卫生治理：移出测试运行输出、机器路径本地化、子模块入库推送与手动计算操作文档

## 原始需求（用户原话，2026-10-13）

> 请帮我处理当前所有尚未提交到 Git 的文件，并基于 issue 和 spec 的方式来实现。具体要求如下：编译生成的临时文件不要加入 Git 版本控制；测试产生的输出文件不需要保留；各个远程机器的登录情况和软件配置情况不需要提交到 Git，只需记录在本地；测试用的输入文件、需要输入的命令和操作步骤都要完整保留并写入文档，方便我后续手动计算时进行查对。请给出满足以上条件的处理方案和需要执行的步骤。

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
