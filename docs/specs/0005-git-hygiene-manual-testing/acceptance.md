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

- **状态**：部分通过（vasp 已推送，q-e 被凭据阻断）
- **证据**：vasp.6.4.3 `git push origin HEAD:master` 成功（`6e06f68..7a82340`，含
  makefile.include 与 BUILD_MACOS.md，`.bak` 未提交）。
  q-e：upstream 已添加并 fetch（upstream/master=`ee7f72e8f`）；fork master 同步与
  `macos-memusage-fix` 推送被阻断——HTTPS 非交互无凭据、SSH 公钥未注册 GitLab、
  钥匙串无 gitlab.com 项、glab 未安装。待用户提供 PAT 或自行推送后补验收。

## SC-005 手动计算操作文档完整

- **状态**：通过
- **证据**：`docs/manual-testing.md` 八节：占位符机制、资产清单、pytest 命令、Rust CLI
  构建、本机 local_run 步骤、集群步骤、scflow 验证脚本、注意事项。

## SC-006 父仓库提交规范且工作区干净

- **状态**：通过
- **证据**：提交消息 `[YYYY-MM-DD HH:MM:SS] 工程治理：…`；提交后 `git status` 仅剩
  external/ 子模块内登记留档的本地项（q-e mbd 指针/.DS_Store、vasp .bak/.DS_Store）。
