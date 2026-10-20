# 验收记录

日期：2026-10-20。用户已明确要求实施完整方案；不提交科学作业、不推送。

## 已完成实现与定向协议

- bands.x 使用自身 `-pd .true.`，保留当前 MPI 启动数量，不继承 PW／EPW pool；dos 和其他原生程序参数不变。冻结尝试增加 bands／DOS 程序身份。
- `.epwflow/electronic/` 分别记录 PW 能带、bands 与 DOS，绑定局部科学键、上游产物、程序／原生输入与日志／退出码／产物身份。PW 与 DOS portable save 同时验证完整文件集合。
- 同请求失败保留有效 PW；改变路径／物理参数更新受影响阶段，改变 DOS 展宽只更新 DOS／图，资源变化排除在科学身份之外。失败后处理日志单独归档，脚本篡改仍拒绝并提示生成新尝试。
- 定向 3 项公开入口测试通过：32 MPI 的 pencil 参数与空 pool 参数隔离、bands 失败后改为 16 MPI 继续而 SCF／NSCF／PW 调用不增加；损坏波函数拒绝与旧 PW 接入；修改脚本拒绝及新尝试恢复。后处理展宽改动不重复 PW／Wannier。
- 负向旧数据覆盖：非零 PW 退出码、错误原生输入、缺波函数及冻结来源改动均拒绝。初次失败是测试硬编码错误 prefix，修正夹具后 3 项全部通过；不隐去该失败。随后新增 SCF 电荷一致性校验及相应负向用例，须在最终回归复测。

## 真实 MgB₂ 只读核验

显式运行 `numbered_real_legacy_pw_is_read_only --ignored --nocapture`，配置来自本例根 input.toml，通过。原始输入与终态冻结配置完全匹配，结构／赝势／KPATH／PW 程序身份通过；零退出码、完成日志、XML 几何、183 个路径 k 点、16 带及全部 portable 波函数检查通过。没有写入现场的新完成记录，没有执行 bands.x、PW 或 Wannier。

受保护范围为已有 SCF、NSCF、电子参考、PH、第 5 步及旧 epwflow/work，**1734 文件、317016282 字节**，核验前后每个原路径、大小及 SHA-256 均未变化。电子 save 的电荷密度与 SCF 快照哈希相同。完整逐文件证据、只读收据输出和日志保存在忽略目录 `docs/local/0020-electronic-resume/`。

## 待完成验证与交付

完整 epwflow／qeflow 回归、最终电荷一致性负向测试、规范构建、安装及安装后身份核验尚在进行，不把此前的定向通过替代最终结论。

## 科学与协作边界

协议替身不证明修复后的真实 bands.x 或 Wannier 已成功；本轮没有真实科学计算与新 Slurm 提交，不增加预算预留。0019 已独立提交 a7b070e；本缺陷另行提交。线上 Gitee、远程双向链接及 Issue 关闭由用户管理。

KISS／DRY／YAGNI：复用 qeflow 同步执行、共享日志／XML／SCF 快照与哈希工具；电子任务收据独立于总调度，不增加配置开关和第二套作业系统。

## 中间验证干扰与处理

第一轮全量命令在执行长链测试时，后续 Cargo 构建替换了共享 target/debug 程序，两个用例失败，其中一个明确记录可执行文件为 deleted，另一个返回部分求解。该轮总体失败，不计完整通过；已经独立结束的测试组与失败日志保留。改为构建与测试严格串行，重跑整个受影响的 three_stage_execution 及尚未执行的 qeflow 测试；最终结果另行追加，不删除失败证据。

## 最终分组回归

为避免替换运行中的测试程序，按组执行并固定执行期间的产物：首轮已独立结束的 17 组为 153 通过／6 ignored；被干扰的整组以最终编号目录 10 通过／1 ignored 和旧接口长链 2 通过覆盖替换；选窗 9 通过，qeflow 33 通过。合计 **207 项通过、0 项最终失败、7 项默认 ignored**，不把首轮失败组中的通过项再次计数。显式真实旧数据只读检查另有 1 项通过，不与默认回归重复相加；7 项默认暂挂不算科学验收通过。

最终编号目录测试包含新 SCF 电荷一致性负向检查。规范 build-native.sh 的 EPW 三程序构建成功；安装制品使用现有 native 解释器／RPATH 并移除调试符号，三份制品 --help 通过。临时目录实际安装脚本 4 项检查通过：有效模块安装与入口、帮助失败原样返回 17 且旧安装哈希不变、缺失源拒绝、无关模块保留与临时清理。正式安装及安装后核验待下段追加。

分组日志与去重计数为 docs/local/0020-electronic-resume/regression-summary.json 及其中引用的日志；最终真实只读日志为 final-real-read-only.log，构建与制品身份为 build.log、build-manifest.json，安装自测为 install-tests.json。构建中有既有未使用代码及未来兼容性警告，本轮未扩大修改无关依赖。

## 正式安装与安装后核验

通过 `scripts/install-bin.sh --module epwflow` 仅更新仓库 bin/epwflow 的三个程序；其他 6 份独立程序文件 SHA-256 保持。实际 source scripts/env.sh 后三个命令均定位 bin/epwflow，实际根 input.toml 的 `--stage wannier --check --json` 返回 configured，SCF／NSCF／PH 状态仍为 reuse；电子阶段及 Wannier 为 update，不冒充计算完成。未在真实工作区执行生成或提交命令，旧输入、脚本和原清单保留。

| 程序 | 安装 SHA-256 |
| --- | --- |
| `epw_phonon` | `658bf414c51da9c5bc2e62be18ccdb256109bcf5a1e3b7a1529d119f6e29e4f8` |
| `epw_wannier` | `4548fa44487294d77beea0c1b0a86cecf65f0a4df33b4ca1e73871a588c58b2f` |
| `epw_superconductivity` | `40fe4ba5a5e6a7c5377e5d0d6a6030c7e8ed5e2a052cbaacc1b2fe68e5465111` |

安装、入口及实际状态证据为忽略目录的 install.log、installed-verification.json、installed-paths.txt、installed-check.json、installed-status.json。最终 EPW 文档测试执行成功（0 用例）；不把零用例计为额外通过。最终科学文件完整集合和哈希结果见 final-scientific-verification.json；本轮真实科学作业数为零。

FR-001～006、SC-001～004 均已完成工程验证和文档回写。上述中间“待完成”段落保留过程语境，以最终分组及安装核验为准；真实修复后 bands.x／Wannier 的科学运行仍由用户后续执行，不纳入本轮完成声明。采用用户已确认的本地 Issue 约定，线上状态和远程链接未操作。
