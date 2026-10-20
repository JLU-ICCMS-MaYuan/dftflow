# 验收记录

日期：2026-10-20。实施前工作树无已跟踪修改。范围与用户补充一致。

## 已完成的脚本审计与验证

- 通过 `git ls-files` 限定 DFTFlow 自有文件，扫描 308 份 Rust／Python／shell／TOML；排除否定断言及检查选项状态的测试后，严格选项注入零命中。另以 `git grep` 核对非文档文件中的组合严格选项，零命中。
- 19 份跟踪 shell 文件全部通过 `bash -n`；Docker 传给 `bash -c` 的两段正文另行解析，语法通过。
- 在临时仓库复制 examples/MgB2/prepare.sh，使用隔离的程序替身生成 index 和 nosplit 提交脚本；全部生成脚本通过语法及选项审计，不执行科学程序。
- Linux 构建脚本的首个 curl／Docker 调用注入退出码 23，脚本返回 23，调用记录只有一次；缺失 ELF 文件的静态校验返回失败。不把这两项记作完整 Docker 或平台构建成功。
- scripts/tests/env.sh 通过：Bash／Zsh、每种 shell 的原仓库与含空格位置各 55 个真实入口、重复加载、仓库外调用、缺失 bin 不修改 PATH、拒绝直接执行、临时 HOME 新交互终端加载。
- scripts/tests/install-compat.sh 和 scripts/tests/install-current.sh 使用已有真实程序副本在临时目录通过；涵盖独立工具保留、旧入口迁移、重复安装、无历史副本及无效源保护。
- 两份历史 Python 测试中的旧正向严格选项断言改为否定断言，AST 解析通过；不宣称运行了旧 Python 科学模块测试。

## Rust 回归与剩余检查

最初扩大范围执行六个 package 的回归，已结束的测试未见失败，但进入耗时的科学插值测试后主动中止；不将该次运行计作完整通过。改用以下覆盖本次修改的定向回归，最终 78 项通过、0 失败、0 暂挂；未选中的科学测试不计入本次结论。

| 验证命令（均经 scripts/build-native.sh） | 通过数 | 结果 |
| --- | --- | --- |
| test --locked -p qf-runner -p qf-phonon -p df-substitution -p qeflow -p wannier90flow | 57 | 含实际 QE 提交／失败退出、声子链与 Wannier 预处理失败退出 |
| test --locked -p epwflow --test three_programs --test superconductivity | 20 | 含实际生成脚本的选项状态、环境退出码、提交回执及原生适配 |
| test --locked -p epwflow --test three_stage_execution numbered_scripts_leave_shell_options_unset_and_stop_on_environment_failure | 1 | 新编号目录脚本实际初始化；可选变量可读，失败返回 17 且不执行后续入口 |

分组安装的全量重复扫描耗时较长，真实程序和轻量夹具两次完整扫描均主动中止，不计作通过或代码失败；最后改用实际 install-bin.sh 的有界定向验证，四项全部通过：有效三程序安装、--help 返回 17 时原样退出、缺失程序拒绝、目标模块为目录链接时拒绝。三个失败场景均逐文件核对原安装 SHA-256 不变，安装锁及暂存目录均清理。该验证使用内容各异的临时可执行夹具；真实 55 入口的启动能力已经单独验证。

构建输出存在既有的未使用代码和未来兼容性警告，本次未改无关代码或依赖。

## 设计与一致性复核

FR-001／002 对应生产生成器和所有跟踪自有脚本；FR-003 由局部 `|| exit $?` 与已有 if/exit 处理；EPW 两套阶段脚本复用同一环境加载段。没有引入配置开关、其他严格选项拼写或全局错误陷阱。FR-004 的 CLI／资源参数及科学数据路径未变。KISS／YAGNI 体现在直接删除和局部检查，DRY 体现在共用环境加载逻辑；不改模块职责。

## 交付边界

只修改源码、跟踪脚本与文档，不执行科学计算、不修改冻结历史运行现场、不覆盖安装、不推送。线上 Gitee Issue 及双向远程链接由用户维护，本地技术完成不等于线上关闭。

FR-001～004 与 SC-001～003 均有上述验证依据；78 项定向 Rust 测试、19 份脚本语法、示例生成、环境加载及安装失败边界通过。完整 Docker 构建、旧 Python 科学模块测试及未选中的科学插值验收不属于本次通过结论。
