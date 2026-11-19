# 已集中迁入的旧目录设计原文

以下是已被0070统一规则取代的历史设计，不能作为现行实现要求。保留完整迁入片段，只在本目录保存；原位置删除且不增加跳转链接。

## 原位置：`docs/specs/0018-epwflow-stage-workspaces/approved-plan.md`

`````markdown
## 2. 目录和计算阶段

新布局直接放在本例 `run/` 下：

```text
run/
├── input.toml
├── POSCAR
├── pp/
├── KPATH.in
├── submit_all.sh
├── 1.qe-scf/
├── 2.qe-nscf/
├── 3.qe-electronic/
│   ├── qe_eband/
│   └── qe_eledos/
├── 4.qe-phonon-nosplit/          # 或 coords、index
│   └── save/
├── 5.epw-wannier/
├── 6.epw-coarse/
├── 7.epw-fine/
├── 8.epw-superconductivity/
│   ├── iso/
│   └── aniso/
└── .epwflow/                    # 内部状态与历史记录
```

- **输入与脚本同目录**：每个实际计算目录都有对应的原生输入、`submit.sh` 和日志；分 q、分温度等子任务同样遵守。批量提交脚本只负责组织依赖，放在这些目录的上层。
- **第 1～4 步**：复用 qeflow 的输入、执行及后处理能力；电子能带与 DOS 保留 `qe_eband`、`qe_eledos` 名称。PH 复用第 1 步的 SCF，不另做一套 SCF。
- **第 4 步**：同时支持 nosplit、coords、index 的 EPW 数据归集，真正生成可校验的 `save/`，包括动力学矩阵、响应势和 phsave。
- **第 5 步**：独立进行 Wannier 拟合，输出与 QE 对比的电子能带、总电子 DOS；复用原生 Wannier90／postw90 及现有绘图能力。
- **第 6 步**：复用第 5 步已验证的 Wannier 变换，计算粗网格电声耦合，并输出电子、声子对比图及质量报告。
- **第 7、8 步**：分别负责细网格 EPC／α²F／λ，以及超导求解。此前暂缓的严格元素／轨道投影仍保持暂缓。

编号帮助阅读；实际执行依赖由程序管理。
`````

## 原位置：`docs/specs/0018-epwflow-stage-workspaces/original-request.md`

`````markdown
```text
1. 我希望提交这个输入文件对应的计算的任务脚本和输入文件在同一个目录，如果要批量提交，批量提交的脚本可以在这些子目录外，但是子目录里面必须有这个子目录里面的输入文件和提交这个子目录的任务的脚本。
2. 我感觉当前的任务目录很复杂很混乱。我希望的结构很简洁，大概是：
run/
├── POSCAR                                  # 共用结构
├── pp/                                     # 共用赝势
├── 1.qe-scf
├── 2.qe-nscf
├── 3.qe-电子能带和态密度，我忘记叫什么了你看看 qeflow 怎么设置的，和 qeflow 保持一致
├── 4.qe-phonon-nosplit 或者 4.qe-phonon-coords 或者 4.qe-phonon-index, 这里面要放置提取好的 save 目录
├── 5.xxx: wannier插值 对比电子结果，出图。
├── 6.xxx: epw粗网格插值，对比电子、声子插值结果，出图。

其实 5.xxx 之后的目录包括 5.xxx 的目录，我没有想好叫什么名字，而且我希望你可以借鉴我在~/code/my_script/epw/里面的目录设计和生成输入文件脚本的方式。

而且我希望修改完 input.toml参数后就可以执行命令，读取新的输入文件，按照新参数开展计算，而不是绑定旧的 64 进程快照，导致重新计算恢复仍会读取旧配置。

为了避免 k 点数小于mpirun np的并行核数和 nk的 pool数，导致有部分 pool 未被分配 k 点而产生数组越界，我希望你可以增加这样一个检查，如果不符合要求，有空的 pool 没有被分配到 k 点，那么就报出这个警告后，停止脚本。目的就是为了解决“当前程序尚不支持变更资源后不支持继续计算“的问题。我希望你能使用 issue 和 spec 新建一个新的需求。然后完成后续的代码！
```
`````

## 原位置：`docs/specs/0018-epwflow-stage-workspaces/plan.md`

`````markdown
## 执行与目录

采用一个编号工作区入口和集中目录映射；旧工作区读取保持有界兼容。阶段枚举描述真实 DAG：SCF→NSCF→Wannier；SCF→PH；NSCF→电子参考；PH+Wannier→粗网格；粗网格→细网格→超导。目录编号不强制错误的线性依赖。

可见目录使用确认的 1.qe-scf、2.qe-nscf、3.qe-electronic/{qe_eband,qe_eledos}、4.qe-phonon-{mode}/save、5.epw-wannier、6.epw-coarse、7.epw-fine、8.epw-superconductivity/{iso,aniso}。状态存放约定由 [0025](../0025-epw-visible-run-state/plan.md) 替代：完成回执归各阶段，共享状态及历史位于可见的 epw-state；不使用隐藏计算目录加表面链接伪装新布局。旧路径迁移与科学数据保留按 0025 验收。
`````

## 原位置：`docs/specs/0025-epw-visible-run-state/plan.md`

`````markdown
## 目录与职责

- `epw-state/` 保存 layout、active、attempts、requests、history、logs、slurm、旧工作流控制记录和跨阶段交付清单；只改存放位置，不复制执行引擎。
- 每个编号阶段使用 `completion.json`；第 3 阶段原生回执放 `3.qe-electronic/receipts/`。后处理回执跟随所属阶段。科学证据枚举排除回执，避免记录自身哈希。
- 路径沿用 layout 唯一映射；阶段目录是否可复用由科学键、依赖及实际文件共同判断。

预算注册表默认使用可见的 `~/epwflow-state/`；显式环境覆盖保持优先。首次写入新默认注册表时承接旧默认位置的 campaign 绑定，防止换目录绕过原账本。
`````

## 原位置：`docs/specs/0050-epw-stage-results/data-model.md`

`````markdown
## 2026-11-16 目录扁平化修订

电子实际路径改为 5.epw-wannier/comparison-native/；声子 force-constants／frequencies 改为 6.epw-coarse/comparison-native/ 下的直接子目录。电子 sampling、DOS、绘图与来源元数据直接位于根；EPW 路径求值 epw 子目录不变。续算日志路径与来源 Identity 同步迁移，不更改科学文件内容。
`````

## 原位置：`docs/specs/0050-epw-stage-results/issue-draft.md`

`````markdown
```text
计算工作区/
├── 1.qe-scf/
├── 2.qe-nscf/
├── 3.qe-electronic/
├── 4.qe-phonon-<模式>/
├── 5.epw-wannier/comparison-native/electron/
├── 6.epw-coarse/comparison-native/{phonon,epw}/
├── 7.epw-fine/
├── 8.epw-superconductivity/
└── results/
    ├── index.json
    ├── 3.qe-electronic/
    ├── 4.qe-phonon-<模式>/
    ├── 5.epw-wannier/
    ├── 6.epw-coarse/
    ├── 7.epw-fine/
    └── 8.epw-superconductivity/
```
`````

## 原位置：`docs/specs/0050-epw-stage-results/issue-draft.md`

`````markdown
## 2026-11-16 比较目录扁平化（当前有效）

### 原始需求记录（追加）

修改代码，修改 issue 和 spec, 开始落地，不要在直接把这些计算文件放在 `comparison-native/`，去掉中间的 `electron/`。 声子的比较在 6.epw-corse里面。所以声子的`comparison-native/`里面也不需要中间的 phonon 目录

同时迁移已有目录，保留科学数据并验证续算

### 已确认结论

第 5 步电子计算文件直接放入 5.epw-wannier/comparison-native/；第 6 步声子比较文件直接放入 6.epw-coarse/comparison-native/，去掉 electron／phonon 中间目录。原始 PH 数据仍在第 4 步，EPW 原生重读数据的 epw 子目录保留。替代旧 FR-003 及目录图对应路径，旧用户原话保留。

- **FR-014**：编号工作区的电子后处理与 QE 声子比较分别直接使用第 5／6 步 comparison-native 根目录；语义路径映射集中维护，不改非编号历史布局。
- **FR-015**：迁移已有 MgB₂ Ubuntu 工作区的已知比较目录及续算引用；拒绝活动作业和目标冲突，科学数据字节保持，路径类元数据同步，不能因此重复拟合或 DOS／声子计算。
- **FR-016**：电子模型变化不能删除或重算相容 QE 声子比较；原始 PH 被替换则失效对应检查点。
- **SC-014**：实际阶段脚本自动出图、失败续算和模型变化复用均通过，目录无 electron／phonon 中间层；迁移前后科学文件校验及无新增科学调用证据通过。

### 本次文档影响与交付

- [x] 已更新本目录 Spec／Plan／Research／数据模型／Tasks／快速验证／检查表及验收。
- [x] 已更新 docs/overview/epwflow/README.md、workflow.md 与模块使用说明。
- [x] 根 README 不列比较目录，入口与命令无变化，无需修改；其模块说明链接继续有效。
- 121 项适用回归通过，2 项专用用例暂挂；固定入口已发布，真实目录迁移及零提交复用核验完成，详见 [验收记录](acceptance.md)。不宣称线上 Issue 已更新或关闭。
`````

## 原位置：`docs/specs/0050-epw-stage-results/plan.md`

`````markdown
## 拟交付目录

以下目录属于未来计算工作区设计；本轮只创建本 Spec 文档目录。

```text
计算工作区/
├── 1.qe-scf/
├── 2.qe-nscf/
├── 3.qe-electronic/
├── 4.qe-phonon-<模式>/
├── 5.epw-wannier/
│   └── comparison-native/
├── 6.epw-coarse/
│   └── comparison-native/
│       ├── force-constants/
│       ├── frequencies/
│       └── epw/
├── 7.epw-fine/
├── 8.epw-superconductivity/
└── results/
    ├── index.json
    ├── 3.qe-electronic/
    ├── 4.qe-phonon-<模式>/
    ├── 5.epw-wannier/
    ├── 6.epw-coarse/
    ├── 7.epw-fine/
    └── 8.epw-superconductivity/
```

每个阶段包保留图片、对应表格、图参和来源。第 3 步能带与 DOS 分别有完整发布单元，尚未成功的单元不阻碍另一单元可见。没有已有最终图的阶段不创建空图或新增图任务。
`````

## 原位置：`docs/specs/0050-epw-stage-results/plan.md`

`````markdown
## 2026-11-16 目录扁平化修订

集中修改 layout::path 编号映射，内部 electron／phonon 语义键保留但不再成为实体目录。第 6 步归档保留 force-constants／frequencies 及对应已核验检查点；原 PH 替换时清除这些检查点，防止跨来源复用。已有 Ubuntu 工作区通过本次一次性定向迁移处理，不引入常驻旧布局回退。

第 6 步批次变更时，保留的 comparison-q2r／comparison-matdyn 检查点重新关联当前批次，再由 attempt 核验旧配置与当前参数；仅电子模型变化不重复计算，上游 SCF／PH 重建则清除检查点。
`````

## 原位置：`docs/specs/0050-epw-stage-results/research.md`

`````markdown
## 2026-11-16 目录扁平化修订

原 electron 层来源于共享比较工作区；现在电子／声子分属第 5／6 步，删除重复层级。现代码将 QE 声子比较映射第 4 步 comparison，必须同时迁回第 6 步；为保持既有不重复计算约束，粗网格归档需保留独立声子后处理检查点。语义路径仍由唯一 layout 映射处理，不散布物理路径。
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
```text
计算工作区/
├── 1.qe-scf/
├── 2.qe-nscf/
├── 3.qe-electronic/
├── 4.qe-phonon-<模式>/
├── 5.epw-wannier/comparison-native/electron/
├── 6.epw-coarse/comparison-native/{phonon,epw}/
├── 7.epw-fine/
├── 8.epw-superconductivity/
└── results/
    ├── index.json
    ├── 3.qe-electronic/
    ├── 4.qe-phonon-<模式>/
    ├── 5.epw-wannier/
    ├── 6.epw-coarse/
    ├── 7.epw-fine/
    └── 8.epw-superconductivity/
```
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
## 2026-11-16 比较目录扁平化（当前有效）

### 原始需求记录（追加）

修改代码，修改 issue 和 spec, 开始落地，不要在直接把这些计算文件放在 `comparison-native/`，去掉中间的 `electron/`。 声子的比较在 6.epw-corse里面。所以声子的`comparison-native/`里面也不需要中间的 phonon 目录

同时迁移已有目录，保留科学数据并验证续算

### 已确认结论

第 5 步电子计算文件直接放入 5.epw-wannier/comparison-native/；第 6 步声子比较文件直接放入 6.epw-coarse/comparison-native/，去掉 electron／phonon 中间目录。原始 PH 数据仍在第 4 步，EPW 原生重读数据的 epw 子目录保留。替代旧 FR-003 及目录图对应路径，旧用户原话保留。

- **FR-014**：编号工作区的电子后处理与 QE 声子比较分别直接使用第 5／6 步 comparison-native 根目录；语义路径映射集中维护，不改非编号历史布局。
- **FR-015**：迁移已有 MgB₂ Ubuntu 工作区的已知比较目录及续算引用；拒绝活动作业和目标冲突，科学数据字节保持，路径类元数据同步，不能因此重复拟合或 DOS／声子计算。
- **FR-016**：电子模型变化不能删除或重算相容 QE 声子比较；原始 PH 被替换则失效对应检查点。
- **SC-014**：实际阶段脚本自动出图、失败续算和模型变化复用均通过，目录无 electron／phonon 中间层；迁移前后科学文件校验及无新增科学调用证据通过。
`````

## 原位置：`docs/specs/0050-epw-stage-results/tasks.md`

`````markdown
## 2026-11-16 目录扁平化修订

- [x] T090（FR-014）修改编号映射、路径契约与进程回归断言。
- [x] T091（FR-016）保持电子模型变化后的独立声子检查点复用，验证 PH 来源失效。
- [x] T092（FR-015／SC-014）定向迁移已有工作区并核验科学文件、来源及续算。
- [x] T093 完成 release 回归、固定入口发布、Overview 与验收回写并提交。
`````

## 原位置：`docs/specs/0055-electronic-plot-windows-pdos/contracts/electronic-results.md`

`````markdown
## 目录与文件

- 第 3、5 步的 plot/ 各生成 electronic.png 和 electronic-from-lowest-to-highest.png，各自对应自包含 .plot.json。
- 第 4、6 步的 plot/ 各生成 phonon.png 和 phonon.plot.json。
- results 对应阶段的 plot/ 发布相同图片与图参。数值 CSV、指标、归一化和科学来源继续独立保留。
- 新生成端不输出分离能带／DOS 图片、-valence 图片或比较总览；逐带范围和衰减诊断保留。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/research.md`

`````markdown
## 配置版本目录变更

用户要求集中在 epw-state，覆盖早先输入旁约定。原文快照不能添加改写后的绝对路径，否则失去原文证据；仅变更读取基准也不足以支持配置目录与 work_dir 不同。因此来源目录归属于版本，原目录和内容 SHA-256 存在已有第 1 步阶段日志，不新增元数据文件。仅旧版本迁移时查询旧活动／未知任务，禁止移动仍被使用的冻结配置；新版本正常写入不因无关活动任务而阻断。旧脚本不重写，尚未结束的旧任务先完成后再迁移。

预存来源日志不能触发编号目录识别；布局应以明确标记为准。既有 --work 旧现场查询只读取固定绑定，不校验与该查询无关的外部原输入；有效 --config 检查按最新需求允许登记原文及来源，但不建立科学现场或执行原生程序。相应基础回归更新的是旧检查的目录存在断言，不放宽科学调用与结果校验。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
### 文件布局

```text
工作区/
├── config.toml                    当前输入示例，现有 epwinput.toml 也可继续使用
├── epw-state/
│   ├── 0000-config.toml           首次被 CLI 读取的原文
│   ├── 0001-config.toml           下一次观察到变化的原文
│   ├── 1.qe-scf.log
│   ├── 2.qe-nscf.log
│   ├── 3.qe-electronic.log
│   ├── 4.qe-phonon-nosplit.log     当前模式也可为 coords 或 index
│   ├── 5.epw-wannier.log
│   ├── 6.epw-coarse.log
│   ├── 7.epw-fine.log
│   ├── 8.epw-superconductivity.log
│   └── imports/                  仅用于外部导入
├── 1.qe-scf/
│   ├── 1.qe-scf.jobs
│   └── 原生输入、输出和脚本
├── 4.qe-phonon-nosplit/
│   ├── 4.qe-phonon-nosplit.jobs
│   └── PH、q2r、matdyn 的实际结果
└── 8.epw-superconductivity/
    ├── 8.epw-superconductivity.jobs
    ├── iso-…/
    └── aniso-…/
```

其他编号目录采用相同的 `.jobs` 位置，不额外建立 `continue.log`。`imports/` 没有导入时可不创建；初始化后的八个日志文件必须齐全。配置不在工作区根时，编号备份仍在解析后的工作区 `epw-state/`；原输入目录和原文哈希随版本登记于第 1 步日志，相对路径按原输入目录解析。
`````

## 原位置：`docs/specs/0012-epwflow-interpolation/plan.md`

`````markdown
各展宽分别使用 fine/smearing-NNNN，复制同一粗网格重启数据，重新生成全细网格选择；不沿用验证点 selecq。fine 主数据绑定 spectrum_index 所选目录；其余展宽角色加 smearing_NNNN_ 前缀共同交付和核验，避免覆盖与错配。F1 完成门及 F4 执行门使用同一质量绑定和原生适配。
`````

## 原位置：`docs/specs/0049-epw-multitemperature-submit/spec.md`

`````markdown
- **FR-003**：目录包含方法、实际 mu 和首尾温度；同方法各温度共目录。
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
> 比较目录以文末“2026-11-16 目录扁平化”修订为准：第 5、6 步分别直接使用 comparison-native/，旧 electron／phonon 路径仅保留为历史记录。
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
- **FR-003：迁移电子比较职责。** 新主路径使用 `5.epw-wannier/comparison-native/electron`，承载电子比较所需的数据、来源记录和必要后处理；第 6 步不再重新生成电子比较数据。
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
- **FR-003：迁移电子比较职责。** 新主路径使用 `5.epw-wannier/comparison-native/electron`，承载电子比较所需的数据、来源记录和必要后处理；第 6 步不再重新生成电子比较数据。
`````

## 原位置：`docs/specs/0050-epw-stage-results/issue-draft.md`

`````markdown
> 比较目录以文末“2026-11-16 目录扁平化”修订为准：第 5、6 步分别直接使用 comparison-native/，旧 electron／phonon 路径仅保留为历史记录。
`````

## 原位置：`docs/specs/0050-epw-stage-results/issue-draft.md`

`````markdown
- **FR-003：迁移电子比较职责。** 新主路径使用 `5.epw-wannier/comparison-native/electron`，承载电子比较所需的数据、来源记录和必要后处理；第 6 步不再重新生成电子比较数据。
`````

## 原位置：`docs/specs/0050-epw-stage-results/issue-draft.md`

`````markdown
2. 电子对比数据、来源和必要后处理放在 `5.epw-wannier/comparison-native/electron`。没有第 4／6 步产物也能交付两张电子对比图。
`````

## 原位置：`docs/specs/0050-epw-stage-results/plan.md`

`````markdown
> 比较目录以文末“2026-11-16 目录扁平化”修订为准：第 5、6 步分别直接使用 comparison-native/，旧 electron／phonon 路径仅保留为历史记录。
`````

## 原位置：`docs/specs/0050-epw-stage-results/data-model.md`

`````markdown
> 比较目录以文末“2026-11-16 目录扁平化”修订为准：第 5、6 步分别直接使用 comparison-native/，旧 electron／phonon 路径仅保留为历史记录。
`````

## 原位置：`docs/specs/0051-epw-real-space-decay/plan.md`

`````markdown
- 阶段内目录为 `6.epw-coarse/decay/`，发布目录为 `results/6.epw-coarse/decay/`。四张独立图命名为 `decay-H`、`decay-dynmat`、`decay-epmate`、`decay-epmatp`，总览为 `decay-overview`，各有 PNG／SVG／PDF。
`````

## 原位置：`docs/overview/epwflow/README.md`

`````markdown
旧隐藏编号布局通过显式 `epw_wannier --migrate-state` 转换，先检查旧任务和未决执行，再直接核验现用科学文件。支持的旧状态一次性撤下，不保留旧隐藏树或复制历史；外部来源放在 `imports/`，预算不重置。未知旧内容明确列出并保留现场，转换可重试。旧非编号工作区使用 `--import-work` 导入新的编号目录。
`````

## 原位置：`docs/overview/epwflow/workflow.md`

`````markdown
```text
计算工作区/
├── 3.qe-electronic/{qe_eband,qe_eledos}/
├── 4.qe-phonon-<当前模式>/
├── 5.epw-wannier/
│   ├── <prefix>_band.dat、<prefix>_band.kpt
│   └── comparison-native/            # 第 5 步电子数据、postw90 与来源
├── 6.epw-coarse/
│   ├── epw_coarse.in、epw_coarse.out
│   ├── comparison-native/
│   │   └── epw/                     # 读取已有模型补声子频率
│   └── comparison/                  # 整理表格与数值差
└── results/
    ├── index.json
    ├── 3.qe-electronic/{qe_eband,qe_eledos}/
    ├── 4.qe-phonon-<当前模式>/
    ├── 5.epw-wannier/
    ├── 6.epw-coarse/{comparison,figures}/
    └── 8.epw-superconductivity/
```
`````

## 原位置：`docs/specs/0012-epwflow-interpolation/contracts/interface.md`

`````markdown
每个展宽在 fine/smearing-NNNN 独立运行；fine 主数据来自 spectrum_index 所选目录，其余展宽的产物角色以 smearing_NNNN_ 前缀保存于同一 artifacts 列表，所有文件均须验源。F4 仅使用未加前缀的所选谱／矩阵角色，不再要求所选谱恰为最后一个展宽。版本 1 的清单外壳不变；本细化不允许旧简化文本进入原生生产路径。
`````

## 原位置：`docs/specs/0050-epw-stage-results/research.md`

`````markdown
> 比较目录以文末“2026-11-16 目录扁平化”修订为准：第 5、6 步分别直接使用 comparison-native/，旧 electron／phonon 路径仅保留为历史记录。
`````

## 原位置：`docs/specs/0051-epw-real-space-decay/spec.md`

`````markdown
- **FR-006：阶段保存与发布。**阶段内保存派生图表，并即时发布到计算工作区根的 `results/6.epw-coarse/decay/`；结果包含四份原始数据副本、逐行绘图数据、图参及运行来源，可移动后仅凭包内数据重绘。原始科学文件保留在阶段目录，不因发布改写。
- **FR-007：局部恢复与缺项说明。**绘图或发布失败保留科学检查点，恢复仅补未完成派生结果。历史工作区缺少文件时明确列出全部缺项，不自动重跑 EPW、不借用其他运行、不伪造完整交付。数据损坏与单纯缺图分开处理。
- **FR-008：人工判断边界。**图表只提供实空间衰减事实，不增加收敛阈值、自动评级、网格扫描、独立采样任务或后续科学阶段放行判据。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/plan.md`

`````markdown
6. 科学数据：PH／粗／细数据从其编号目录重新解析，生成内存对象供下游。比较声子的 Γ onsite／Hermitian q2r 与 matdyn 计算放在第 4 步 `comparison/`，与该阶段默认 simple ASR 数值分开；相关子任务归第 4 步日志，电子模型变化不重复声子后处理。导入来源和必要映射保留 imports；移除共享清单读写和 post 缓存。结果包清单保持自包含交付用途，不承担调度／续算状态。
7. 续算：PH、q2r、matdyn、电子各子任务、比较、展宽及方法／温度在所属日志独立表示；已通过科学检查的子项可复用。明确重算仅清空本阶段可替换内容并撤下受影响下游状态，保留阶段根目录。
8. 旧状态退出：先查询旧当前任务的调度状态，再提取可用的配置／阶段／导入信息；不要求已归档的输入在线。退出代码只处理固定程序状态路径，不能删除 imports 或现用科学数据；运行中／未知任务明确列出。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
- **FR-031**：编号原文配置统一位于 `<work_dir>/epw-state/NNNN-config.toml`；每次配置驱动 CLI 比较指定输入与最后编号，相同不新增，不同新增下一编号，A/A/B/A 保存三份，保留全部原文字节。
- **FR-032**：移动快照后，相对路径仍按该版本原输入目录解析；排队任务继续使用冻结版本。旧版本迁移须核对冲突和任务终态，更新阶段／子任务配置引用后再移除旧副本；中断可恢复，不覆写科学文件或重算。
- **SC-016**：覆盖按字节比较、并发、旧版本迁移／冲突／重试、相对路径与跨目录配置、排队后改输入；SiP₂ 根目录的旧快照迁入状态目录，已有阶段状态与科学产物不变。
`````

## 原位置：`epwflow/README.md`

`````markdown
当前任务、科学检查和续算记录统一写 `epw-state/` 八份中文阶段日志，Slurm 作业号仅放各计算目录同名 `.jobs`。电子后处理位于 `5.epw-wannier/comparison-native/`，能带由 EPW 内嵌 Wannier90 输出，DOS 调用 postw90，原模型保持只读。
`````

## 原位置：`epwflow/README.md`

`````markdown
```text
run/
├── input.toml、POSCAR、pp/、KPATH.in
├── 1.qe-scf/
├── 2.qe-nscf/
├── 3.qe-electronic/
│   ├── qe_eband/
│   └── qe_eledos/
├── 4.qe-phonon-nosplit/       # 也可为 coords 或 index
│   └── save/
├── 5.epw-wannier/
├── 6.epw-coarse/
│   ├── comparison-native/
│   ├── comparison/
├── 7.epw-fine/
├── 8.epw-superconductivity/
│   ├── iso/
│   └── aniso/
├── results/                 # 按阶段即时发布图、数据、参数及来源
│   ├── index.json
│   ├── 3.qe-electronic/
│   ├── 4.qe-phonon-nosplit/
│   ├── 5.epw-wannier/
│   ├── 6.epw-coarse/
│   └── 8.epw-superconductivity/
└── epw-state/
```
`````

## 原位置：`epwflow/README.md`

`````markdown
```text
8.epw-superconductivity/
├── iso-mu0.13-20-40/     # epw.in、epw.sh、epw.out/err 与所有温度原生输出
├── aniso-mu0.13-20-40/
├── manifest.json
└── results/
```
`````

## 原位置：`docs/specs/0049-epw-multitemperature-submit/issue-draft.md`

`````markdown
> 我希望你把目录命名为aniso/iso-mu0.13-<起始温度>-<终止温度> 默认计算的mu=0.13
`````

## 原位置：`docs/specs/0049-epw-multitemperature-submit/spec.md`

`````markdown
> 我希望你把目录命名为aniso/iso-mu0.13-<起始温度>-<终止温度> 默认计算的mu=0.13
`````

## 原位置：`docs/specs/0049-epw-multitemperature-submit/spec.md`

`````markdown
用户省略 mu_star 后生成目录，得到 iso-mu0.13-20-40／aniso-mu0.13-20-40；显式 0.1 时使用实际值。独立验收为各温度原生结果在同方法目录，结果包能逐点验证。优先级依据：用户明确目录要求。
`````

## 原位置：`docs/specs/0050-epw-stage-results/issue-draft.md`

`````markdown
每个结果子目录保存图片、对应数值表、绘图参数和来源记录，支持独立重绘。原始科学文件留在计算阶段；没有既有图片的阶段不凭空新增绘图任务。
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
每个结果子目录保存图片、对应数值表、绘图参数和来源记录，支持独立重绘。原始科学文件留在计算阶段；没有既有图片的阶段不凭空新增绘图任务。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/issue-draft.md`

`````markdown
采用本条最新要求，替代原先配置同目录的保存位置：在工作区 epw-state 中保存原文编号快照，每次公开 CLI 执行前按字节比较最后一份，变化才递增；保留已有编号、内容和计算绑定。旧快照核对后迁入并更新阶段引用，科学输入输出不改写、不重算，活动／未知旧任务期间不移动其冻结配置。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/quickstart.md`

`````markdown
编号版本统一位于解析后的 `work_dir/epw-state/`。例如原文依次为 A、A、B、A，会得到 `0000-config.toml`、`0001-config.toml`、`0002-config.toml`，中间相同的 A 不重复保存。相对路径继续以原输入目录为基准；每版原始目录及原文哈希保存在 `1.qe-scf.log` 的 `Configuration snapshot` 小节，不手工编辑该记录或快照。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
采用本条最新要求，替代原先配置同目录的保存位置：在工作区 epw-state 中保存原文编号快照，每次公开 CLI 执行前按字节比较最后一份，变化才递增；保留已有编号、内容和计算绑定。旧快照核对后迁入并更新阶段引用，科学输入输出不改写、不重算，活动／未知旧任务期间不移动其冻结配置。
`````

## 原位置：`docs/specs/0010-epwflow-qe-data/contracts/interface.md`

`````markdown
默认 outdir 是按配置目录解析的基址；实际保存树为 <outdir>/<run_id>/<stage_id>。同一绑定内路径一经准备不变，用户自定义优先，工作目录与保存目录分别显示。
`````

## 原位置：`docs/specs/0012-epwflow-interpolation/contracts/interface.md`

`````markdown
默认 outdir 是按配置目录解析的基址；实际保存树为 <outdir>/<run_id>/<stage_id>。同一绑定内路径一经准备不变，用户自定义优先，工作目录与保存目录分别显示。
`````

## 原位置：`docs/specs/0013-epwflow-superconductivity-tc/contracts/interface.md`

`````markdown
默认 outdir 是按配置目录解析的基址；实际保存树为 <outdir>/<run_id>/<stage_id>。同一绑定内路径一经准备不变，用户自定义优先，工作目录与保存目录分别显示。
`````

## 原位置：`docs/specs/0014-epwflow-delivery/contracts/interface.md`

`````markdown
默认 outdir 是按配置目录解析的基址；实际保存树为 <outdir>/<run_id>/<stage_id>。同一绑定内路径一经准备不变，用户自定义优先，工作目录与保存目录分别显示。
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
- **FR-010：统一结果目录。** `results` 位于计算工作区根目录，与编号阶段目录同级；每个阶段已有的最终图片及配套数据立即按阶段发布，不等最后统一收集。
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
最终结果采用以下结构：
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
- **SC-008：**完整结果目录移动后可重绘；旧工作区接入不改写原始科学数据。
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
**独立验收**：中断发布并恢复，比较前后科学调用和文件哈希；移动完整结果目录，在没有科学程序的环境重绘。
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
4. **假如**完整结果目录已移动，**当**只使用包内数据和图参重绘，**那么**不依赖原工作区绝对路径、不调用 QE／EPW／Wannier 或提交作业。
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
- **FR-010：统一结果目录。** `results` 位于计算工作区根目录，与编号阶段目录同级；每个阶段已有的最终图片及配套数据立即按阶段发布，不等最后统一收集。
`````

## 原位置：`docs/specs/0050-epw-stage-results/spec.md`

`````markdown
- **SC-008（US4；FR-003、005、010）**：完整结果目录移动后可重绘；旧工作区接入不改写原始科学数据。
`````

## 原位置：`docs/specs/0050-epw-stage-results/issue-draft.md`

`````markdown
- **FR-010：统一结果目录。** `results` 位于计算工作区根目录，与编号阶段目录同级；每个阶段已有的最终图片及配套数据立即按阶段发布，不等最后统一收集。
`````

## 原位置：`docs/specs/0050-epw-stage-results/issue-draft.md`

`````markdown
最终结果采用以下结构：
`````

## 原位置：`docs/specs/0050-epw-stage-results/issue-draft.md`

`````markdown
- **SC-008：**完整结果目录移动后可重绘；旧工作区接入不改写原始科学数据。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/contracts/files.md`

`````markdown
- `epw-state/` 保存八份阶段日志和编号原文配置，目录例外仅 `imports/`。声子日志使用实际模式名。
`````

## 原位置：`docs/specs/0008-epwflow-foundation/spec.md`

`````markdown
outdir 是保存目录基址，缺省配置值 ./tmp；按运行和阶段派生独立可写保存树，不能让 PH、电子态或各 q 点共享可写目录。check/prepare 必须展示派生后的实际路径，不能仅打印基址掩盖数据位置；F2 负责把对应实际路径写入科学输入。
`````

## 原位置：`docs/specs/0051-epw-real-space-decay/spec.md`

`````markdown
- **FR-006：**阶段内保存派生图表，并发布到 `results/6.epw-coarse/decay/`，包含原始数据副本、绘图数据、参数和来源，支持移动后独立重绘。
`````

## 原位置：`docs/specs/0051-epw-real-space-decay/spec.md`

`````markdown
- **SC-005：**移动结果目录后，仅凭包内数据可重绘；缺少声子比较或细网格结果不影响衰减图交付。
`````

## 原位置：`docs/specs/0051-epw-real-space-decay/issue-draft.md`

`````markdown
- **FR-006：**阶段内保存派生图表，并发布到 `results/6.epw-coarse/decay/`，包含原始数据副本、绘图数据、参数和来源，支持移动后独立重绘。
`````

## 原位置：`docs/specs/0051-epw-real-space-decay/issue-draft.md`

`````markdown
- **SC-005：**移动结果目录后，仅凭包内数据可重绘；缺少声子比较或细网格结果不影响衰减图交付。
`````

## 原位置：`docs/specs/0049-epw-multitemperature-submit/contracts/config-cli.md`

`````markdown
目录首尾采用配置列表首末项，无单位后缀或多余零。
`````

## 原位置：`docs/specs/0049-epw-multitemperature-submit/contracts/config-cli.md`

`````markdown
目录为 8.epw-superconductivity/iso-mu0.13-20-40 和 aniso-mu0.13-20-40，各有 epw.in、epw.sh、epw.out／err、slurm-epw.log 及带温度的原生输出。
`````

## 原位置：`docs/specs/0056-epw-stable-workdir/plan.md`

`````markdown
公共 archive_step 将阶段目录的直接子项逐个移动到历史的 calculation/，不重命名阶段根。先保存现用阶段脚本；所有移动成功后恢复脚本并继续原来的状态失效过程。当前 Slurm 的日志通过硬链接备份到归档，原位置不移动；其余条目移动前检查目标不存在。错误通过上下文给出源／目标，向上传播阻止计算。归档中断保留已移动内容；下次显式恢复保留现存历史并归档剩余内容，不自动合并或覆盖历史。
`````

## 原位置：`docs/specs/0014-epwflow-delivery/spec.md`

`````markdown
运行实体位于 test_auto/MgB2/local_run/001-epw-validation/，各次实验目录命名为编号加英文目的简述；科学程序要求的 tmp、save、原生输出子目录名保留。脚本目录不再保留运行数据、PDF、历史输入或兼容数据链接；说明文档可保留。历史文件内容不改写，通过迁移映射和单独的新路径来源清单复查，不在生产读取器引入机器路径兼容分支。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/issue-draft.md`

`````markdown
## 原始需求记录

后续真实重算授权：

> 你把整个代码重新编译拿到新的 CLI 之后, 把/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow里面的那些计算目录和epw-state, results都删除，然后重新测试。

> 旧输入已被归档，但 `native-active.json` 仍引用原路径，程序在进入恢复流程前就因找不到旧输入而停止了。 我觉得这很有问题。就是这些文件一直在阻止我重算。我觉得这些防御性设计让这个程序就很难用。epw-state里面的各个文件都是做什么用的？我总感觉保存这么多东西，没用。

下列文件名是用户批注的上下文；引用块逐字保留对应批注。

`layout.json`：

> 这个部分保留到 4.qe-phonon-xxx.log里面。但是不要写成 json 格式。尽可能美观有条理的展示给我。

`current-config.toml`：

> 不需要保存这个，而是改造成 0000-config.toml, config.toml，只要修改一次输入文件，就备份一次。这样我就可以拿到每一次修改后的 toml文件。

`parameter-map.json`：

> 删除不需要

`active.json`：

> 也放到 4.qe-phonon-xxx.log里面

`attempts/`：

> 我觉得这个完全不需要保留，里面的内容也都不需要保留。代码上也完全不需要这部分的暴露。

`native-active.json`：

> 不需要保留这个输出

`native-active-iso/aniso.json`：

> 这两个记录放在 8.epw-superconductivity.log里面。

`native-jobs/`：

> 我建议为每个任务新建一个 xxx.jobs，比如 1.qe-scf.jobs，专门用来存放任务号，用来判断这个阶段的任务完成没完成。

`native-jobs/history/`：

> 不需要保留。

`native-progress/`：

> 这个其实很重要，这个我需要和你探讨一下，能不能在 xxx.log和 xxx.jobs里面兼容这个续算的功能。或者你可以把续算的功能放到各个计算目录里面记录，统一都叫做 continue.log

`requests/`：

> 不保留。对于上一个步骤的依赖，可以从代码上去上一个目录里面检查。我记得你有这部分代码，为什么还要再暴露一个 requests目录呢？都删了这部分代码。不暴露输出 requests

`native-attempts/`：

> 不保留

`qe-data/、coarse/、interpolation/`：

> 不保留。根本用不上，我也不会看这些目录。

`post/`：

> 不需要保留。你直接去 4.qe-phonon-xxx目录里面去找。

`history/`：

> 不需要保留，就直接覆盖就行。

`imports/`：

> 这个很重要，这个需要保留，以后导入外部计算得到的 save 目录，就用这个目录来完成。

> 我现在的思路是在 epw-state里面只放 8 个文件，这8 个文件对应 8 个计算状态：1.qe-scf.log   2.qe-nscf.log   3.qe-electronic.log    4.qe-phonon- xxx.log(因为)    5.epw-wannier.log    6.epw-coarse.log     7.epw-fine.log    8.epw-superconductivity.log  ，然后你往这样的 8 个 文件也要规范输出并且具有人类的可读性。人类的可读性非常重要，你是给我看的，不光是你自己看。
>
> 新建一个 issue 和 spec 来完成。

续算记录位置的澄清答复：

> 各计算目录只放 xxx.jobs：续算信息统一并入 epw-state/ 的八份阶段日志。

配置备份触发时机的澄清答复：

> 每次运行 CLI 时检查（推荐）：内容变了就保存为下一份 0000-config.toml、0001-config.toml；不改内容就不重复备份。

> 开始落地
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/issue-draft.md`

`````markdown
## 原始需求记录追加：编号配置统一存入状态目录

> 我希望你能把0000-config.toml ，0001-config.toml都放在 epw-state目录里面，每次执行之前都对比 --config 指定的和最后一个编号的 toml 是不是相同，如果不同就保留到 epw-state里面一个新的。使用 issue 和 spec 完成。



本条追加需求已实现并完成本地回归、正式 CLI 覆盖发布及 SiP₂ 无计算迁移验收；两份快照编号与字节不变，八日志绑定迁入后保持科学状态，重复调用不新增版本。Documentation Impact：Spec／Plan／Tasks／模型／契约／Quickstart／验收、相关 Overview、根及模块 README 和 SiP₂ README 均同步；详见 [本项验收](acceptance.md)。不操作线上 Issue，不推送。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/issue-draft.md`

`````markdown
## 需求结论

- `epw-state/` 顶层恰好八个阶段日志文件，另允许用户明确保留的 `imports/` 目录；声子日志按当前实际模式命名。
- `layout.json` 的布局／声子模式及 `active.json` 的声子阶段任务信息并入第 4 步日志；第 3 步等其他阶段分配写各自日志。
- iso／aniso 状态分别显示在第 8 步日志内；续算统一进入八份日志，不生成 `continue.log`。
- `xxx.jobs` 位于对应编号计算目录，记录子任务与当前 Slurm 任务号；任务实时状态向 Slurm 查询，科学完成另核验实际输出。
- 不再生成或读取 `current-config.toml`、`parameter-map.json`、`active.json`、`attempts/`、`native-active*.json`、`native-jobs/`、`native-progress/`、`requests/`、`native-attempts/`、共享 `qe-data/`／`coarse/`／`interpolation/`、`post/` 和 `history/`。删除相应持久化、路径门槛和对外暴露代码，不能换位置保留同一套记录。
- 上游依赖直接检查所属计算目录中的科学输入输出；声子后处理就近使用第 4 步结果，不另建共享缓存。
- 当前输入文件旁保存 `0000-config.toml`、`0001-config.toml` 等原文快照。每次配置驱动的 CLI 调用检查，内容变化才备份；不保存程序展开后的第二套配置，不强制改名用户现有输入。
- 重算覆盖本阶段旧现场，不自动建立历史；旧记录缺文件不能阻止已结束任务按当前配置重算。正在运行或提交结果未知的任务不能被覆盖。
- 保留外部 `save/` 导入能力与来源追踪；旧目录退出、下游检查和清理命令同步改造。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
## 原始需求记录

后续真实重算授权（2026-10-28）：

> 你把整个代码重新编译拿到新的 CLI 之后, 把/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow里面的那些计算目录和epw-state, results都删除，然后重新测试。

本次追加范围：全工作区 release 编译并发布 CLI，清除指定 MgB₂ 工作区的八个计算目录、状态及结果，从 SCF 重跑当前小网格八阶段及 iso／aniso 20／30／40 K。保持科学输入不变，不将功能验收称为 Tc 或网格收敛。原“未安装／未执行科学计算”为首轮受控回归的历史事实。

> 旧输入已被归档，但 `native-active.json` 仍引用原路径，程序在进入恢复流程前就因找不到旧输入而停止了。 我觉得这很有问题。就是这些文件一直在阻止我重算。我觉得这些防御性设计让这个程序就很难用。epw-state里面的各个文件都是做什么用的？我总感觉保存这么多东西，没用。

下列文件名是用户批注的上下文；引用块逐字保留对应批注。

`layout.json`：

> 这个部分保留到 4.qe-phonon-xxx.log里面。但是不要写成 json 格式。尽可能美观有条理的展示给我。

`current-config.toml`：

> 不需要保存这个，而是改造成 0000-config.toml, config.toml，只要修改一次输入文件，就备份一次。这样我就可以拿到每一次修改后的 toml文件。

`parameter-map.json`：

> 删除不需要

`active.json`：

> 也放到 4.qe-phonon-xxx.log里面

`attempts/`：

> 我觉得这个完全不需要保留，里面的内容也都不需要保留。代码上也完全不需要这部分的暴露。

`native-active.json`：

> 不需要保留这个输出

`native-active-iso/aniso.json`：

> 这两个记录放在 8.epw-superconductivity.log里面。

`native-jobs/`：

> 我建议为每个任务新建一个 xxx.jobs，比如 1.qe-scf.jobs，专门用来存放任务号，用来判断这个阶段的任务完成没完成。

`native-jobs/history/`：

> 不需要保留。

`native-progress/`：

> 这个其实很重要，这个我需要和你探讨一下，能不能在 xxx.log和 xxx.jobs里面兼容这个续算的功能。或者你可以把续算的功能放到各个计算目录里面记录，统一都叫做 continue.log

`requests/`：

> 不保留。对于上一个步骤的依赖，可以从代码上去上一个目录里面检查。我记得你有这部分代码，为什么还要再暴露一个 requests目录呢？都删了这部分代码。不暴露输出 requests

`native-attempts/`：

> 不保留

`qe-data/、coarse/、interpolation/`：

> 不保留。根本用不上，我也不会看这些目录。

`post/`：

> 不需要保留。你直接去 4.qe-phonon-xxx目录里面去找。

`history/`：

> 不需要保留，就直接覆盖就行。

`imports/`：

> 这个很重要，这个需要保留，以后导入外部计算得到的 save 目录，就用这个目录来完成。

> 我现在的思路是在 epw-state里面只放 8 个文件，这8 个文件对应 8 个计算状态：1.qe-scf.log   2.qe-nscf.log   3.qe-electronic.log    4.qe-phonon- xxx.log(因为)    5.epw-wannier.log    6.epw-coarse.log     7.epw-fine.log    8.epw-superconductivity.log  ，然后你往这样的 8 个 文件也要规范输出并且具有人类的可读性。人类的可读性非常重要，你是给我看的，不光是你自己看。
>
> 新建一个 issue 和 spec 来完成。

续算记录位置的澄清答复：

> 各计算目录只放 xxx.jobs：续算信息统一并入 epw-state/ 的八份阶段日志。

配置备份触发时机的澄清答复：

> 每次运行 CLI 时检查（推荐）：内容变了就保存为下一份 0000-config.toml、0001-config.toml；不改内容就不重复备份。

> 开始落地
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
## 原始需求记录追加：编号配置统一存入状态目录

> 我希望你能把0000-config.toml ，0001-config.toml都放在 epw-state目录里面，每次执行之前都对比 --config 指定的和最后一个编号的 toml 是不是相同，如果不同就保留到 epw-state里面一个新的。使用 issue 和 spec 完成。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
- **FR-001**：编号工作区初始化后，`epw-state/` 顶层保存八份阶段 `.log` 和递增的 `NNNN-config.toml` 原文配置；只保留 `imports/` 这一目录例外，不引入隐藏状态仓库。编号配置位置按追加需求 FR-031 调整。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
- **FR-003**：工作区布局和实际声子模式进入第 4 步日志，删除 `layout.json` 的生成和运行依赖；根据当前配置与真实目录定位阶段。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
- **FR-007**：删除 `active.json` 和 `attempts/` 以及对应提交快照／回执目录机制；声子阶段任务信息写第 4 步日志，其他阶段任务信息写各自日志。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
- **FR-010**：每个编号计算目录就近使用与阶段同名的 `.jobs`，仅保存当前批次的子任务与 Slurm 作业号，不另建 `native-jobs/` 或其历史；实时调度状态查询 Slurm。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
- **FR-011**：阶段与子任务续算信息集中在八份日志，删除 `native-progress/`；不增加 `continue.log` 或第二套续算文件。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
- **FR-013**：删除 `native-attempts/`、其检查点目录和失败尝试归档；粗／细网格与比较任务的当前续算状态并入所属阶段日志。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
- **FR-014**：删除共享 `epw-state/qe-data/`、`coarse/`、`interpolation/` 的持久化和运行读取依赖；相关下游就近读取实际科学数据，所需数据结构可以在内存中构造，不能换位置恢复同一套持久清单。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
- **FR-015**：删除 `post/` 及共享后处理缓存，直接使用第 4 步的 PH／q2r／matdyn 产物，必要后处理状态进入所属阶段日志。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
- **FR-016**：重算直接覆盖本阶段现用文件，不创建 `history/`、失败尝试目录或其他计算历史备份；配置版本和明确保留的导入数据除外。
`````

## 原位置：`docs/specs/0064-epw-readable-state-logs/spec.md`

`````markdown
- **FR-017**：继续保留 `imports/` 与外部 `save/` 导入功能、来源及必要映射；不得因删除共享清单而丢失真实导入来源或自动重做 PH。
`````

## 原位置：`docs/specs/0050-epw-stage-results/data-model.md`

`````markdown
## 根 results 与阶段包

根索引 `results/index.json` 的版本化格式由 `foundation/stage_results.rs` 定义：

| 字段 | 约束 |
|---|---|
| schema_version | 新阶段索引版本，从 1 开始；不能冒充已有结果包版本 |
| entries | 已完整发布的阶段结果单元数组 |
| entries[].stage | 编号阶段与真实所选声子模式 |
| entries[].unit | 子任务／图组标识；第 3 步能带与 DOS 分开 |
| entries[].manifest | Artifact：`relative_path`、`byte_size`、`sha256`，拒绝越出 results 的路径 |

阶段清单为 `stage-manifest.json`，字段是 `schema_version`、`stage`、`unit`、`request_key`、`parameters`、`sources` 和 `files`。科学请求身份保存在每个单元的 `request_key`；`sources` 保留审计 Identity，`files` 使用包内相对 Artifact。图片、图参及数值一并核验。发布沿用文件 journal；`.index-update.json` 只承担清单已发布但根导航未更新时的事务恢复，不是独立科学状态。

根索引是派生导航，可由完整阶段包重建；不能拿它代替 epw-state 的工作流状态，也不能靠手改索引“登记”科学完成。

阶段包使用现有 delivery 能力组织以下信息：包版本、阶段／单元身份、来源指纹、数值表、绘图参数、图片及各格式哈希、实际程序路线、比较口径、窗口、差值和未配对说明。第 5 步包独立于第 4／6 步；第 6 步总览绑定第 5 步电子数据指纹，并把重绘需要的电子数值和图参纳入完整结果目录。
`````

## 原位置：`docs/specs/0050-epw-stage-results/data-model.md`

`````markdown
## 状态与发布一致性

逻辑状态是现有状态系统内的职责划分，不要求新增以下同名磁盘文件：

1. **科学证据有效**：原生结果及身份已核验，可作为检查点；不是最终图表全部交付。
2. **派生数据就绪**：比较表、图参、窗口与来源已核验。
3. **单元已发布**：图像、数据、清单完整，已通过发布校验。
4. **阶段交付完成**：该阶段所有应交付单元均已发布，随后才记录最终交付成功。

每个完整单元先发布清单，再更新根导航；恢复优先重验已有清单与发布 journal。部分发布不能把整个阶段标成完整，但完整子任务应保持可见。采用已有发布锁与恢复机制避免并发写索引丢条目；活动／未知作业仍服从现有防重规则。
`````

## 原位置：`docs/specs/0050-epw-stage-results/data-model.md`

`````markdown
## 版本和旧布局

已知旧路径 `6.epw-coarse/comparison-native/electron`、`5.epw-wannier/native-post` 及 `6.epw-coarse/results` 只作为有界只读来源。核验后导出派生结果，不搬移原科学产物、不改旧清单内容或冻结脚本。新包显式标明读取的旧版本及来源，旧版本无法提供必要身份时报告缺项，不能重捕哈希冒充已核验历史。
`````

## 原位置：`docs/specs/0018-epwflow-stage-workspaces/spec.md`

`````markdown
- FR-001（目录）：八个编号目录；输入、提交脚本、日志位于实际计算目录；所有子任务遵守相同规则。
`````

## 原位置：`docs/specs/0018-epwflow-stage-workspaces/data-model.md`

`````markdown
- 工作区：版本、PH 模式、根配置路径、布局；按 [0025](../0025-epw-visible-run-state/plan.md)，可见的 epw-state 管理共享元数据、冻结输入、导入的参考副本和历史，完成回执位于各阶段 completion.json，实际阶段计算位于编号目录。旧隐藏路径仅在显式迁移/导入边界读取。
`````

## 原位置：`docs/specs/0025-epw-visible-run-state/spec.md`

`````markdown
- **FR-001**：阶段完成与电子原生任务回执归各自编号阶段；共享运行记录位于可见目录，旧非编号主路径也不创建隐藏状态。
`````

## 原位置：`docs/overview/epwflow/README.md`

`````markdown
第 8 步的 `epw_superconductivity --stage superconductivity -s` 一次提交所选方法：both 为两个独立 Slurm 作业，无方法间依赖。每个方法共用 `8.epw-superconductivity/iso-mu<实际值>-<起始温度>-<终止温度>/` 或 `aniso-mu<实际值>-<起始温度>-<终止温度>/`，首尾按配置列表顺序命名，不建温度子目录。每个目录保留原生输入、脚本、日志、各温度 Δ／Z 和不可变尝试快照。
`````

## 原位置：`docs/overview/epwflow/README.md`

`````markdown
新工作区设置 `layout="numbered"`，采用八个实际计算目录。原生输入、脚本和输出共目录；`epw-state/` 保存八份英文阶段日志和编号原文配置，目录例外只有 `imports/`。每次 CLI 观察到输入原文变化时，在工作区 `epw-state/` 保存递增的 `NNNN-config.toml`，阶段日志指向当前及计算使用的版本。子任务、后处理、绘图和各温度的续算均进入日志，不另外保存阶段 completion、receipts、请求、共享 manifest、尝试或历史。
`````

## 原位置：`epwflow/README.md`

`````markdown
`results/` 位于计算工作区根，与编号目录同级。第 3、5 步各发布两张电子组合图，第 4、6 步各发布一张声子组合图，均使用阶段 plot/；第 3、4 步保留 QE 元素投影；第 8 步是已有超导图。第 7 步没有独立既有图，不创建空包或额外任务。每个包带表格、图参、来源和文件校验清单。
`````
