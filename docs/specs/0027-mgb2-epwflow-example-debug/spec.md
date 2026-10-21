# 规格：MgB₂ 指定目录的真实 EPW 验证

Issue：[本地 0027](issue-draft.md)，type:bug。无附件，images: []。用户直接授权修改指定 TOML、运行和 debug，不重复请求确认；不虚构已审阅本文。

## 原始需求记录

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。

## 用户故事

US1：在指定 epwflow 目录使用 epwinput.toml 和上一级 POSCAR，通过三个公开程序生成、提交、续算并查看真实结果。US2：遇到失败能定位原生程序、配置、安装、调度或科学质量原因，修复有依据，不重复相容上游。

## 功能需求

- FR-001：配置解析至指定 POSCAR、共用 pp 与本目录 KPATH；工作目录保持本目录，状态可见，命令明确使用 -c epwinput.toml。
- FR-002：核对源码、构建与 bin/epwflow 三程序身份，通过规范构建和分模块安装部署；保留已有未提交工作的来源说明。
- FR-003：实际验证 QE、Wannier、粗网格、细网格与超导可达链路；每个阶段以退出状态、原生日志、产物和适用数值门判断，失败不能以提交成功掩盖。
- FR-004：保持共享 8 节点小时／100 GiB 总额度与历史消费，活动保护和科学质量检查不降低；旧准备记录归档前核验，原始文件内容保留。
- FR-005：本例 README 提供本目录直接执行命令和结果位置；缺陷先记录 Issue／Spec，源代码和真实科学验收分别登记。

## 成功标准

- SC-001：指定 TOML 的 check 通过，读取的结构／赝势与用户指定一致；生成无额外嵌套目录。
- SC-002：三程序与实际脚本的可达阶段都有明确证据；未通过阶段标记阻塞或失败，不发布伪科学结果。
- SC-003：失败恢复、复用、配置快照、缺失产物和并行相关回归通过；真实计算与回归分记。
- SC-004：数据、预算与安装来源核对完整，README 及验收可复查；本项范围安全分离后依仓库规则自动提交，不推送。

## 边界与假设

采用当前 32 MPI／32 pool 与物理参数作为首轮基线；32 粗 k 不改成 64 pool。用户允许为 debug 修改 TOML，但不得通过放宽质量门冒充成功，不盲目升级外部 QE／W90／EPW。阶段质量不通过时保留诊断并阻断下游；参数修正须有原生或科学依据。本轮未发现可复用的指定工作区编号 QE 数据，允许从第 1 步真实运行。
