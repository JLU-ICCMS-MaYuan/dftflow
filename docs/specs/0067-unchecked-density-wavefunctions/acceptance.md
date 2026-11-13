# 0067 验收记录

## 结果

已取消原始电荷密度和波函数核验，并覆盖历史记录、复制、电子参考、PDOS、导入和共享 QE 初始化。目录保持现状；未提交科学作业，未改写真实原始数据。其他核验保留，没有新增严格模式开关。

正式入口的真实复用跟踪结果：同一配置、同一现场执行 epw_phonon -c examples/MgB2/ubuntu/input.toml --stage scf -s，旧 CLI 对两类文件发起 3934 次 openat，新 CLI 为 0 次。两次均确认 SCF 已完成，未重算；不是用额外运行新科学作业获取的差值。旧路径还涉及电子参考和其他已完成阶段，因此只看 13 文件／3.2 MB 的 SCF 快照不足以概括全部 I/O。此比较是打开次数证据，不作为通用耗时加速倍数。

## FR／SC 对照

| 要求 | 实现与验收 |
| --- | --- |
| FR-001～002／SC-001～002 | qf-io 的 qe_payload 纯名称规则；核验清单在打开文件之前跳过 raw payload；旧 Identity.verify／identity_available 不访问这些文件。FIFO 回归在 3 秒内完成，证明没有阻塞读取；旧记录、缺失和内容变化回归通过 |
| FR-003／SC-002 | save::all_files 与 save::files 分离；实际复制仍包含数据，copy_tree 只比较受保护文件。普通目录和仅有波函数的目录均复制正确，未因排除而漏复制 |
| FR-004～005／SC-003 | 旧快照预期清单、旧产物记录、QE manifest、电子参考、投影与导入统一忽略对应条目；共享 QE 声子初始化不再预先要求 charge-density／wfc 非空或存在 |
| FR-006 | 原生输入、UPF、XML、日志、收敛与其他产物检查保留；坏 XML 仍停止恢复，不重跑 PW；导入仍拒绝 ../wfc1.dat 等非法路径，通用路径边界不属于原始数据完整性核对 |
| FR-007～008／SC-004 | 前序原话与当前范围澄清完整落档；AGENTS、当前 Overview、根／模块 README 和 Ubuntu 手册同步；全工作区 release 成功覆盖 9 个正式命令，并与 build 制品逐字节核对；真实 SCF 复用通过 |

## 回归

分批去重 420 通过、15 既有 ignored，无未解决失败，包括 EPW、qeflow 和 qf-io。其他不使用该规则的模块未重复运行测试；全工作区发布构建通过。

首次广泛回归有一项旧断言要求坏 XML 必须包含固定 data_invalid 错误前缀，实际程序已经拒绝损坏 XML。保留“必须拒绝／不启动科学程序”的断言，移除文案耦合后单项补测通过。首次编译检查成功；收尾文档注释编辑曾引入字面换行语法错误，已修正并重新编译测试通过。完整回归与收尾测试按日志分别记录，不冒充一次性全量通过。

新增回归覆盖：FIFO 不读、缺失或改写不拒绝、新记录不包含两类数据、正常完整复制、只含 payload 的目录复制、公开 SCF 复用不重算，以及缺失 NSCF 波函数不触发核验拒绝。测试夹具中的内容比较仅用于确认复制行为，不是生产检查。

## 构建与现场

测试结束后执行 bash scripts/build-native.sh build --release --workspace --locked -j 6，覆盖 wannier90、scflow、vaspflow、substitution、qeflow 与 EPW 四入口。无编译版本目录或发布记录。确认当前正式命令与最终 release 制品一致后才进行新入口跟踪。

现场保持 examples/MgB2/ubuntu/run、campaign 和 repair/pp 原位置。Ubuntu 目录 Git 仍仅保留 input.toml 与 readme.md。实际复制错误和 QE／EPW／projwfc 原生读取错误继续报告，不把本项解释成原始程序无需数据。

## 本地证据

完整构建、回归、测试汇总、前后 openat 跟踪和命令输出保存在忽略目录 docs/local/0067-unchecked-density-wavefunctions。线上 Issue／远程 Spec 链接仍由用户管理，未操作当前不匹配的 origin，未推送。
