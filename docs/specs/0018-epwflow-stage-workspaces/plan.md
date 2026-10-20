# 实施设计

## 执行与目录

采用一个编号工作区入口和集中目录映射；旧工作区读取保持有界兼容。阶段枚举描述真实 DAG：SCF→NSCF→Wannier；SCF→PH；NSCF→电子参考；PH+Wannier→粗网格；粗网格→细网格→超导。目录编号不强制错误的线性依赖。

可见目录使用确认的 1.qe-scf、2.qe-nscf、3.qe-electronic/{qe_eband,qe_eledos}、4.qe-phonon-{mode}/save、5.epw-wannier、6.epw-coarse、7.epw-fine、8.epw-superconductivity/{iso,aniso}。内部状态及历史放 .epwflow；不使用隐藏计算目录加表面链接伪装新布局。

## 身份与尝试

每步清单包含科学请求指纹、上游清单/产物指纹、逐文件哈希、状态及来源。资源与启动器不属于科学请求；二进制布局另验。只复用已完整验证、参数及上游一致的产物。变更使该步骤及真实依赖失效，保留旧目录到历史后生成新的输入。共享配置每次提交重新读取；已提交尝试冻结配置、可执行程序身份、原生输入、脚本，脚本启动核验哈希。活动/未知任务禁止覆盖或重复。

## 科学执行

复用 qeflow/qf-input/qf-phonon 的 QE 原生输入、执行与后处理；保存 SCF 不可变快照供 NSCF/PH 独立复制。三模式 PH 严格配对模式基底与响应势、校验 q 映射后归集。阶段 5 采用 EPW 内嵌 Wannier：ep_coupling=true、elph=false、wannierize=true；阶段 6 elph=true、wannierize=false，读取已核验 ukk；阶段 7 只读取便携粗网格产物。电子图复用原生 Wannier90/postw90 与共享绘图；不造谱。

## 并行与迁移

三处共用预检，分开 QE 与 EPW 规则；读 NSCF XML 核对实际粗 k 数，细阶段核对本次细网格，单 image EPW 要求 np=npool≤Nk。脚本启动另外核对 Slurm 分配。旧 work 只读验证清单/配置/哈希，复制并重定位新清单，保留旧身份与路径作为来源，不删除旧目录。

## 验证与交付

先隔离的公开 CLI/脚本协议测试，涵盖变更矩阵、队列冻结、防重复、预检、三模式映射及变换复用。真实 MgB₂ 仅校验/导入并核对哈希；不运行原生计算。科学结果是否通过另行记录。最后更新文档与 bin/epwflow，验证后仅提交本需求文件。
