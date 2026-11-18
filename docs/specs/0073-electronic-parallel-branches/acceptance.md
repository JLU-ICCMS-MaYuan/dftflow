# 验收记录

## 范围与结论

2026-11-18 按 GitHub #73 的目标实现两个独立 Slurm 分支、独立恢复、统一收尾与即时范围诊断。科学网格收敛和声子插值精度不属于本验收。

源码基线为 df7c6bd6；现有工作区同时开展 #74，验证使用基线导出的隔离目录 /tmp/dftflow-issue73，只叠加 #73 文件，未引入 #74 的共享绘图与依赖修改。

## 自动回归

| 验证 | 实际结果 | 证据 |
| --- | --- | --- |
| #73 公开 CLI 和分支脚本专项 | 11 通过 | /tmp/issue73-tests.log；epwflow/tests/support/electronic_parallel_0073.rs |
| #72 自动绘图回归 | 7 通过 | /tmp/issue73-stage72.log |
| epwflow 库测试 | 87 通过，4 忽略 | /tmp/issue73-lib.log |
| 扩大进程回归，修改前 df7c6bd6 | 33 通过、106 失败、1 忽略 | /tmp/issue73-baseline-process.log |
| 初次扩大进程回归，#73 | 36 通过、107 失败、1 忽略 | /tmp/issue73-full-process.log |

11 项专项覆盖：两个作业先提交而不执行科学、afterany 收尾依赖、活动或未知提交拒绝、两个真实进程同时进入受控原生程序并合并检查点、双向失败恢复、两边失败恢复、即时诊断及诊断失败继续 bands、改变路径采样只更新能带、删除 bands 表只补 bands、缺失原子投影只补 projwfc。对实际文件、原生调用次数和作业号作断言，不以源码字符串代替执行。

扩大回归中原有 106 项失败在本轮全部复现，主要涉及旧测试未准备直接上游输入、仍假定旧逐任务提交状态等。本轮没有将这些失败计入通过。唯一新增失败为 #72 中“投影失败仍发布组合图”的旧预期，与 #73 的两分支成功后统一绘图冲突；已更新该断言并复测 #72 全部 7 项通过。另有 #73 新增 11 项专项通过。

## 真实 MgB₂ 与 Slurm

只读接续 examples/MgB2/ubuntu 已有 SCF／NSCF，实际复制到新的测试目录；没有重算 SCF／NSCF／PH／EPW。每个科学分支为 12 MPI，OMP/MKL 线程为 1，Slurm 用户 header 为 12 ntasks。将路径采样设为每段 4 个间隔用于调度与恢复验证，不声称能带收敛。

1. test_auto/MgB2/local_run/0073-electronic-parallel-20261118/：独立提交能带 37、DOS 38、收尾 39。真实 pw.x、bands.x、dos.x、projwfc.x 日志均有 JOB DONE，完成数据核验并生成范围诊断和两张组合图。DOS 仍排队时，能带范围图已经生成；本机资源决定两分支实际顺序运行。
2. test_auto/MgB2/local_run/0073-electronic-recovery-20261118/：能带 40、DOS 41、收尾 42。仅测试包装命令使 bands 返回 17，真实路径 PW、dos、projwfc 正常结束；收尾报告能带分支失败，组合图没有发布。解除测试故障后，--resume -s 只新增能带恢复 43、收尾 44；原路径 PW 仍属于 40，DOS／projwfc 仍属于 41，没有重复提交 DOS 分支。
3. before-resume.json 与 resume-verification.json 对照成功的 PW 日志／XML、总 DOS 与投影日志，内容及修改时间完全一致；源 SCF／NSCF XML 的 SHA-256 也不变。未核验电荷密度或原始波函数的内容、大小、存在性及文件集合。

测试中的失败由新目录内 bands-controlled.x 与 fail-bands 控制，不代表原生 bands 科学错误；原始示例不改动。作业完成信息通过只读复制 Slurm jobcomp 日志后查询，37／38／39／41／43／44 为 COMPLETED，40／42 为受控故障 FAILED，见首个目录 slurm-completions.txt。科学原始证据保留在忽略目录，没有纳入提交。正式固定安装入口与测试私有程序分别记录。

## 构建、文档与协作收尾

- Spec、Plan、Research、数据模型、内部契约、Tasks、Quickstart 和检查清单齐备；Overview、模块 README、根 README 已更新对应行为。
- GitHub #73 已关联为 #8 的原生子 Issue；类型保持唯一 type:feature。
- 隔离源码执行 `bash scripts/build-native.sh build --release -p epwflow --bins --locked` 成功，4 个公开模块入口通过官方发布脚本覆盖现用命令；逐个 SHA-256 与隔离受测制品完全一致，`--help` 正常。已安装入口再次对真实成功目录执行 `--stage electronic -s`，退出 0 并复用全部科学数据，`.jobs` 原文不变，没有提交新作业。证据：/tmp/issue73-release.log、真实目录 installed-verification.json 和 installed-reuse.stdout。
- 本地 Spec 尚未发布到远程分支，双向远程链接未通过；#73 保持 open，不能据本地文件冒充关闭门完成。Git 推送遵守 AGENTS.md 的单独明确授权要求。
