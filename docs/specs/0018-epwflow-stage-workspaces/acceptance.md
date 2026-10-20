# 验收记录

用户已授权实施完整方案。本 Feature 的工程验收与真实科学验收分开记录；本轮不提交真实科学作业，不放宽质量标准、不另开预算。

## 实现与协议覆盖

- 八阶段目录映射、原生输入与本地提交脚本；电子两叶目录与分 q／参考／展宽／温度尝试均有自己的脚本。脚本固定绝对工作目录，运行前重新检查 Slurm／MPI／pool。
- 当前根配置决定新请求；阶段科学参数和上游产物决定复用。排队尝试冻结输入、脚本和程序身份，根配置随后修改或暂时损坏不会改变已排队任务。
- 公开进程覆盖 MPI 变化保留 QE、细网格变化只更新下游、能窗变化保留 QE、粗 q 变化保留 Wannier；后处理展宽改动及改回、路径密度变化均不重复拟合或 EPC。
- 失败粗网格换 MPI 后保留 QE 和固定独立参考；超导部分失败不能发布阶段完成，换 MPI 恢复保留成功模式。
- 32 k／64 pool 拒绝，32 MPI／32 pool 合法；手工提交的 Slurm 任务数不一致也在原生程序前停止。活动任务防重复、未知终态与文件完整性防护继续适用。
- nosplit／coords／index 三种 PH 来源均核验并发布 save，coords 局部 q 编号规范化；第 6 步读取第 5 步 ukk，设置 wannierize=false。

## 真实 MgB₂ 只读导入

来源为本例旧 epwflow/work。核验旧运行绑定、F2 清单、终态及文件哈希后，物化到根编号目录；没有调用科学程序或提交作业。

- 旧现场全部 **1,336 文件、196,757,749 字节** 的路径、大小及 SHA-256 在导入前后完全一致。
- 新 F2 清单有 **136 个产物、32 个 NSCF k 点、16 带、2 个不可约 q 点**，逐项校验通过；SCF、NSCF、PH 显示 reuse，电子参考及后续显示 update。
- 导入原 q2r／matdyn 与旧独立质量参考，后者重新核验绑定并保存可移植副本。重复导入返回 reused。
- 本机根 input.toml 沿用用户当前 **32 MPI／32 pool**；共同 POSCAR、pp 和旧配置保持。新配置、运行数据和逐文件证据留在忽略目录。

机器证据：docs/local/0018-workspace/source-before.json、source-after.json、source-verification.json、import.stdout、import-repeat.stdout、status-imported.json，以及新工作区 .epwflow/legacy-import.json。没有把旧结果的目录迁移称为新链科学验收。

## 回归与安装

2026-10-20，通过规范入口执行：

```bash
bash scripts/build-native.sh test -p epwflow -p qeflow --locked --config 'profile.test.package.sha2.opt-level=3'
```

共 **203 通过、0 失败、6 ignored**，31 个测试组（含零用例的二进制／文档测试组）。6 项依赖真实数据或真实科学作业的测试未执行，不计通过；完整日志和计数留在 docs/local/0018-workspace/tests.log、tests-summary.json。编号工作区的 6 个新增公开进程测试全部通过，另 2 个旧公开全链进程测试保持通过。

规范构建与分组安装：

```bash
bash scripts/build-native.sh build -p epwflow --locked --config 'profile.dev.package.sha2.opt-level=3'
bash scripts/tests/install-bin.sh /本地兼容制品目录 /仓库/bin/tools/qf-gen-dyn0
bash scripts/install-bin.sh --module epwflow /本地兼容制品目录
```

构建成功后复制制品，移除调试符号并沿用本机原生运行库的解释器／RPATH；源码、构建与安装分开记录，不替换历史运行绑定程序。安装回归通过重复更新、55 入口、单模块更新、缺失／损坏源、目标冲突及源目标重叠保护。正式安装只更新 bin/epwflow 三程序，其余五模块和 dyn0 的哈希不变。

| 安装程序 | SHA-256 |
| --- | --- |
| `epw_phonon` | `36bbf30f08740473c6e1eb75f951962eac37ee69b6c4eb8cf117558af6e12b17` |
| `epw_wannier` | `dff07b2ca456c4296ee3feb415031432883aaf3bc88d09b9733b22f02009e9f9` |
| `epw_superconductivity` | `5719a6aff5858eff2ea29a1bc5a3e0cf738ba600fba52c1a03e600362fedc629` |

安装后通过 scripts/env.sh 核对三个命令均来自仓库 bin/epwflow；实际根配置的 --all 只生成八阶段目录、10 个本地 submit.sh 及根 submit_all.sh。全部脚本 Bash 语法检查通过；第 5 步 --check --json 通过，状态仍为 SCF／NSCF／PH reuse。各温度／展宽等依赖实际数据的子任务在执行时生成，不以空父目录冒充已完成计算。

预算消费保持 **3315 节点秒**；没有新增科学作业或计算预留。构建、安装与实际工作区证据留在 docs/local/0018-workspace/build.log、install-tests.log、install.log、installed-verification.json、installed-paths.txt、generated.stdout、status-installed.json、check-installed.json。21 份入口／模块／Spec 文档的本地链接、安装脚本 Bash 语法和 Git 差异空白检查通过；Epic 关联也已核验。

## 科学验收边界

协议替身只验证原生 IO、调用、依赖、失败恢复及哈希；不生成可用于科学结论的参考。真实 MgB₂ 新布局拟合、粗网格、细网格及 iso／aniso 的完整数值验收仍由既有 0015 等需求追踪，本轮没有运行这些作业。严格元素／轨道投影继续暂缓。历史科学结果及其失败结论保持原记录，不因本 Feature 的工程测试通过而改变。
