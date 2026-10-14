# Acceptance：0005 Git 卫生治理与手动计算操作文档

逐项对照 `spec.md` 的 SC，验收时附可复查证据。状态：未验收 / 通过 / 失败。

## SC-001 构建与测试产物不入 Git

- **状态**：通过
- **证据**：`git ls-files test_auto/MgB2/local_run` 仅剩 8 个输入/脚本/结构/赝势文件
  （MgB2.vasp、3 个 input_*.toml、2 个 run_*.sh、potcar_lib/{B,Mg}）；105 个运行输出已
  `git rm --cached`，磁盘保留（`ls test_auto/MgB2/local_run` 完好）。`.gitignore` 新增
  `.pytest_cache/`、`docs/local/`、`*.bak` 与 local_run 白名单规则。

## SC-002 已跟踪文件无机器路径

- **状态**：通过（按 spec 澄清的保留范围）
- **证据**：`git grep -nE '/Users/mayuan|/work/home/mayuan|/home/mayuan'` 排除 AGENTS.md
  历史条目、0001–0003 spec 历史记录与 `qeflow/tests/fixtures/golden/` 冻结基线后无命中
  （执行记录 exit 1）。活文档/配置/输入类文件已全部占位符化。

## SC-003 pytest 收集与占位符注入正常

- **状态**：通过
- **证据**：`pytest --collect-only` 收集 121 项；5 个收集错误为 P5 删除 Python 包的既有
  环境性问题（`public`、`qeflow.phonon`、`vaspflow.scf` 等模块不存在），与本次改动无关。
  conftest 注入冒烟：临时测试 `test_zz_smoke_localpaths.py` 验证三个 input toml 替换后
  均为真实路径且无 `@` 残留，1 passed 后已删除。

## SC-004 子模块推送完成

- **状态**：通过（2026-10-14 补齐）
- **证据**：
  - vasp.6.4.3 `git push origin HEAD:master` 成功（`6e06f68..7a82340`，含 makefile.include
    与 BUILD_MACOS.md，`.bak` 未提交）。
  - q-e fork master 同步成功：`git push origin upstream/master:master`
    （`770a0b2d1..ee7f72e8f`，fast-forward 至官方上游最新）。
  - q-e `macos-memusage-fix` 分支推送成功，`git ls-remote` 验证远端指向 `51fdfbd1320d3…`
    （即含 macOS 内存查询补丁的提交）。
  - 临时使用的 GitLab PAT 在推送完成后已从钥匙串清除，并提醒用户轮换。

## SC-005 手动计算操作文档完整

- **状态**：通过
- **证据**：`docs/manual-testing.md` 八节：占位符机制、资产清单、pytest 命令、Rust CLI
  构建、本机 local_run 步骤、集群步骤、scflow 验证脚本、注意事项。

## SC-006 父仓库提交规范且工作区干净

- **状态**：通过
- **证据**：提交消息 `[YYYY-MM-DD HH:MM:SS] 工程治理：…`；提交后 `git status` 仅剩
  external/ 子模块内登记留档的本地项（q-e mbd 指针/.DS_Store、vasp .bak/.DS_Store）。

## 2026-10-14 补充验收：VASP Linux 配置备份入库

- **授权变化**：用户明确要求将唯一未提交的 `makefile.include.linux-aocl.bak` 一起提交到 VASP 远程；该文件成为备份排除与机器路径本地化要求的单文件例外。
- **SC-004 补充证据**：VASP 提交 `5e8bc01c89483b05d2587f13db854791cfbdf430` 仅新增该备份（83 行）；`git push origin HEAD:master` 成功（`7a82340..5e8bc01`），随后 `git ls-remote origin refs/heads/master` 返回相同完整提交号。
- **内容完整性**：提交前 `git hash-object` 与提交后 `git rev-parse HEAD:makefile.include.linux-aocl.bak` 均为 `5b7876d8ccd3f68d49cfa433a8b1f0b6019e99ed`，备份内容未修改。
- **SC-006 补充证据**：VASP 子模块工作区为空；父仓库本次提交同步该指针与 0005 文档、AGENTS.md。最终递归 Git 状态在提交后核验。
- **验证边界**：此次仅做 Git 纳管、推送及文档更新，未重新编译 VASP 或重跑 pytest；之前的计算与测试记录属于历史验收，本次不据此宣称重新通过。父仓库仅本地提交，线上 Issue 仍由用户自管。

## 2026-10-14 补充验收：单文件历史清理与 rust 推送

- **授权范围**：仅从 rust 历史移除 `test_auto/MgB2/local_run/vasp_cohp_uniform/WAVECAR`，保留本地文件；不批量清理其余 104 个历史运行文件。
- **本地回退**：原分支 `119b87d6886dab540c5ca68702fc0d3e3f69a495` 保存于 `refs/backups/rust-before-wavecar-20261014-124611`；逐提交映射及哈希核验记录保存在 gitignored 的 `docs/local/history-cleanup-20261014-124611.json`。该备份不推送。
- **清理结果**：8 个提交重写后，分支头为 `31829a916d661bc7e6a9f5721cf8d26b2029a37d`；每对新旧提交仅允许该文件差异，提交总数不变。当前树仍为 `6a3622200c6cba604a989e51a05c0f23b0bda18a`，其他原有 refs（包括 python-reference-0004）未变。
- **本地文件保留**：清理前后 SHA-256 均为 `a999bdadaafc528dcb5a65ec76c83bbd90436f1c8d9e8e970c122b72cc686f09`，文件仍在本地且被忽略。
- **历史验证**：`git log rust -- <目标路径>` 为空；`git rev-list --objects rust` 不含被拒绝 blob `0f4f1d08d10d1e8878ceae12454c7418891804ce`。本地备份仍保留旧对象，未执行对象回收。
- **推送证据**：普通推送（未强推）成功创建远程 rust，并设置本地分支跟踪 origin/rust；随后本节文档随独立收尾提交推送。
- **边界**：未修改计算逻辑或重新运行计算；历史文档中的旧提交号保留为当时记录，可通过本地映射查询新提交号。线上 Issue 未操作。
