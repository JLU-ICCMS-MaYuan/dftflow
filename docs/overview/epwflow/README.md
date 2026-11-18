# EPW 工作流

当前版本：编号工作区（schema_version = 2）

最后更新：2026-11-17

[返回项目总览](../README.md)

---

## 目录

- [快速导航](#-快速导航)
- [常用命令](#常用命令)
- [当前入口与验证边界](#当前入口与验证边界)
- [科学输入与输出文件名](#科学输入与输出文件名)
- [公开程序及选项](#公开程序及选项)
- [与 qeflow 的实际复用关系](#与-qeflow-的实际复用关系)
- [常见问题](#常见问题)

## 🚀 快速导航

### 新手入门

- [架构总览](architecture.md) ⭐ 必读
- [完整流程图](workflow.md)
- [开算前检查](preflight-and-reuse.md)
- [5 分钟快速开始：MgB₂ Ubuntu 示例](../../../examples/MgB2/ubuntu/readme.md)

### 核心概念

- [阶段配置详解](native-stage-config.md)
- [NSCF 网格生成](nscf-grid.md)
- [超导求解](native-superconductivity.md)

### 使用手册

- 三个主程序：`epw_phonon`（1～4）／`epw_wannier`（5～6）／`epw_superconductivity`（7～8）
- [常用命令模式](#常用命令)
- [历史清理](epw_clear.md)

### 参考手册

- [CLI 完整参考](#公开程序及选项)
- [文件命名规范](#科学输入与输出文件名)
- [与 QE 复用关系](#与-qeflow-的实际复用关系)
- [模块操作手册](../../../epwflow/README.md)

### 历史与演进

重要里程碑：[0018 编号工作区](../../specs/0018-epwflow-stage-workspaces/acceptance.md)／[0026 原生提交](../../specs/0026-epw-direct-native-submit/acceptance.md)／[0043 常规对比](../../specs/0043-epw-comparison-max-differences/acceptance.md)／[0050 阶段结果](../../specs/0050-epw-stage-results/acceptance.md)／[0058 移除预算](../../specs/0058-epw-remove-resource-limits/acceptance.md)／[0064 可读日志](../../specs/0064-epw-readable-state-logs/acceptance.md)。
已废弃：`layout.json` 状态入口、独立抽样参考、预算账本；旧布局仅通过显式迁移或导入接续。

---

## 常用命令

在配置目录执行；每步完成并核验后再执行下一条。指定阶段只开展本阶段，缺少直接输入会停止，按提示先运行数据所属阶段。

```bash
# 检查配置与并行参数；不等于科学验收
epw_phonon -c "input.toml" --check --json
# 八阶段按需逐项执行，不要一次连续提交
epw_phonon -c "input.toml" --stage scf -s
epw_phonon -c "input.toml" --stage nscf -s
epw_phonon -c "input.toml" --stage electronic -s
epw_phonon -c "input.toml" --stage phonon -s
epw_wannier -c "input.toml" --stage wannier -s
epw_wannier -c "input.toml" --stage coarse -s
epw_superconductivity -c "input.toml" --stage fine -s
epw_superconductivity -c "input.toml" --stage superconductivity -s
```

第 3～6 步一次阶段分配自动完成本阶段科学计算、核验、必要后处理与绘图；第 7／8 步原生任务结束后重复同一公开命令核验、推进。gap 按 iso／aniso 分别提交原生多温度作业。

```bash
# 查询；修复失败原因后显式恢复
epw_wannier -c "input.toml" --status --json
epw_wannier -c "input.toml" --stage wannier --resume -s
# 现场重绘与独立结果包重绘，均不补算科学数据
epw_phonon -c "input.toml" --stage electronic --plot
epw_wannier -c "input.toml" --plot
epw_superconductivity --plot --results "run/results"
epw_superconductivity --results-check --results "run/results" --json
```

`--stage scf --resume -s` 会强制重算已退出的 SCF 并覆盖现用结果；检查或复用 SCF 使用普通 `--stage scf -s`，活动／未知作业禁止覆盖。

## 当前入口与验证边界

- 真实验收记录：2026-10-28 曾用发布入口从干净 MgB₂ 小网格完成八阶段，iso／aniso 的 20／30／40 K 六点收敛且数据有效；八日志、作业号、配置备份和图表均核对。见 [0064 验收](../../specs/0064-epw-readable-state-logs/acceptance.md)及[联合验收](../../specs/0050-epw-stage-results/real-chain-validation.md)。这不构成 Tc 或网格收敛验收，也不证明后续源码已经安装或重新科学验收。
- 当前只处理目标阶段的直接文件，不递归核验／自动补算上游，不顺带刷新其他阶段图片。完成与复用依据本阶段参数、原生日志、格式、数值和收敛检查；Slurm 成功退出不能代替科学成功。
- 新科学调用合并 stdout／stderr 到 `.out`；脚本为 `<功能>.sh`，调度日志为 `slurm-<功能>.log`，覆盖写入。作业在目标目录提交，脚本以 `#SBATCH --chdir` 指定目录；环境加载失败显式退出，脚本不自动启用严格 shell 选项。
- 当前任务、失败原因和下一步保存在八份阶段日志，Slurm 作业号另存计算目录同名 `.jobs`；状态查询不提交、不迁移、不补图。提交未知和活动作业必须先核对，禁止重复提交。
- `[slurm].header` 提供完整用户 Slurm 文件头，`programs.launcher` 独立设置 MPI；不恢复 `[resources]`、不自动补资源申请。本机示例为 12 MPI、12 pool，OMP／MKL 为 1；其他机器按实际配置。
- 电荷密度与原始波函数不核验内容、大小、非空、存在性或文件集合，旧记录对应条目忽略；复制／链接及原生读取照常进行，真实 I/O 错误正常报告。结构、赝势、程序、输入、XML、日志、收敛及其他直接产物检查保留。
- 第 8 步 `task="gap"` 执行显式温度列表；`task="tc"` 只执行明确请求的 iso 原生线性化温度列表，要求 `tc_linear=true`，不做自动区间搜索或双初值补算。执行完成、收敛与数据有效分别记录，partial 不冒充完整交付。

<a id="外部原始声子接续0010"></a>

外部 PH 接续使用 `epw_phonon -c "input.toml" --import-phonon "external-phonon/save" -s`：要求已有有效 SCF／NSCF，只核验、转换和归集来源，再完成 q2r／matdyn 与出图，不回退执行 PW／PH；来源只读。详见 [0010 验收](../../specs/0010-epwflow-qe-data/acceptance.md)。

<a id="预算功能已移除"></a>

预算功能已移除：不读写账本、不要求 campaign 身份、不限制全局作业数；旧 `[budget]`、`campaign_id`、`ledger` 字段须迁移，集群限制由 Slurm 管理。

<a id="常规对比与最大差值0043"></a>

常规差值只描述已有共同坐标的能量／频率差，无阈值、RMS 评级或下游放行门；不为配对补算或重采样。第 6 步直接读取第 4 步原生声子频率／DOS，保留双方 ASR、采样与展宽，不另做比较专用 q2r／matdyn。

<a id="编号工作区的电子后处理续算"></a>

电子后处理失败保留有效 PW 数据，修复后只补未完成子任务。参数来自各程序独立 `args`，不自动追加 pencil 或沿用其他程序的 pool。详见[文件接续](preflight-and-reuse.md)。

<a id="实际解纠缠外窗与测试目录"></a>

### 电子双能窗、元素 PDOS 与保留色

第 3／5 步各生成能带＋DOS 组合图：默认 E_F±3 eV 与 QE 路径全部本征值范围；第 5 步保留实际四个 Wannier 窗口标注，QE 实线先绘、EPW 蓝色虚线后绘。显示范围不改变科学输入或差值；第 3 步有 QE 元素 PDOS，第 5 步只比较总 DOS，EPW 元素／轨道投影尚未提供。
两张电子图的 DOS 横轴均参考当前 `[plot]` 能窗内所有 DOS 曲线的最大值，加 10% 留白并向上整理刻度；全能窗图的深能级尖峰不撑宽横轴，截断峰用箭头及实际峰值标注，原始数值不变。详见[组合图显示范围](native-stage-config.md#组合图显示范围)。
第 4 步输出 QE 声子谱＋总量／元素 DOS，第 6 步只输出声子谱＋TDOS 对比及实空间衰减诊断，不生成电子声子四面板总览。各阶段 `plot/` 图片及必要图参发布到 `results/`；完整新版结果目录支持独立重绘，历史包按自身格式兼容。

<a id="三个前置电子阶段的能带范围图0068"></a>

SCF／NSCF 及 electronic 路径完成后还会生成逐带范围诊断，不等待下游；诊断图不放宽正式 Wannier 检查。参见 [0068 验收](../../specs/0068-epw-stage-band-ranges/acceptance.md)。

## 各阶段自动绘图（0072）

第 1～6 步的当前阶段分配在科学任务结束后执行对应绘图，无需另提绘图作业。第 1／2 步生成逐带范围图；第 3 步生成范围图和近费米能、全能窗电子组合图，正常 DOS 面板包含元素 PDOS；第 4 步生成声子能带／DOS 组合图；第 5 步只比较电子能带／总 DOS；第 6 步比较声子能带／DOS，并生成实空间衰减图。

图像位于 `run/results/<编号阶段>/`。当前阶段复用时补齐本阶段缺图，不检查或补画上游图片，不因缺图重跑科学程序。SCF／NSCF 的独立目录可使用 `epw_phonon --plot --results "run/results/1.qe-scf"` 等形式重绘；第 3／5 步也保留既有现场重绘命令。

PNG 写入失败与科学计算失败分开记录：已核验科学数据和完成标记保留，绘图／后处理及 Slurm 作业仍可能标记失败并返回非零。修复绘图原因后显式重绘，不把作业失败等同于科学数据丢失，也不把保留数据等同于图片交付完成。验证范围见 [0072 验收](../../specs/0072-stage-auto-plotting/acceptance.md)。

## 科学输入与输出文件名

| 科学调用 | 输入／执行输出 |
| --- | --- |
| SCF／NSCF | `scf.in/out`／`nscf.in/out` |
| QE 电子后处理 | `elebanddata.in/out`、`eletdos.in/out` |
| 第 5～7 步 | `epw_wannier.in/out`、`epw_coarse.in/out`、`epw_fine.in/out` |
| 第 8 步／原生比较 | `epw.in/out`／`epw_compare.in/out` |
| PH／q2r／matdyn | 保留对应调用已有的成对命名，具体见实际科学目录 |

同一次调用输入与执行输出主文件名相同，stdout／stderr 合并到 `.out`；不生成独立 `.err` 或 `.exitcode`，实际退出码和信号进入任务／阶段日志。`.wout`、矩阵元和谱等原生数据保留科学程序规定名称。
旧 `bands.out`、`dos.out`、`epw.out` 只读兼容，已登记任务继续使用原输出。新名存在时优先核验新名，不因新日志失败而回退旧成功日志；不批量改名。见 [0051 Spec](../../specs/0051-matching-log-names/spec.md)。

## 公开程序及选项

默认读取执行目录的 `input.toml`，缺失时兼容 `inputepw.toml`；`-c/--config` 显式指定路径。普通入口只接受版本 2 八阶段原生配置，结构／赝势等相对 TOML 所在目录解析，不能直接混用 qeflow 配置。以下以当前编号入口为准，历史固定快照接口不代表当前默认行为。

| 程序或选项 | 实际行为与限制 |
| --- | --- |
| `epw_phonon --stage scf\|nscf\|electronic\|phonon` | 第 1～4 步，默认 phonon；只运行选定阶段，PH 支持 nosplit／coords／index |
| `epw_wannier --stage wannier\|coarse` | 第 5／6 步，默认 coarse；不执行细网格或 α²F |
| `epw_superconductivity --stage fine\|superconductivity` | 第 7／8 步，默认 superconductivity；矩阵与谱来源按所选路线检查 |
| [epw_clear](epw_clear.md) | 八日志布局无需清理；旧布局默认预览，`--yes` 清空三处历史 |
| 默认调用／`--prepare-only` | 准备当前目标的输入和脚本，不提交、不执行 |
| `-s/--submit`／`-r/--run` | 提交当前任务／阶段分配，或在已有 Slurm 分配内同步运行；gap 按方法分组 |
| `epw_phonon --all` | 仅准备就绪原生脚本和待办信息，不提交；不承诺自动 afterok 八阶段串联 |
| `--check [-w W] [--json]` | 当前编号入口检查配置、输入路径和并行条件；不执行科学程序或证明其运行版本 |
| `--status [-w W] [--refresh] [--json]` | 查询记录及作业状态；编号入口按工作区报告，不补算或出图 |
| `--resume -s/-r` | 修复后恢复当前失败目标，重验并复用相容成功子项；显式 SCF 恢复强制重算 |
| `--migrate-state` | 显式转换旧隐藏状态为八日志，保留现用科学数据；核对旧作业与锁，不提交 |
| `--import-work W`／`--import M` | 显式导入旧工作区科学数据／登记本阶段既有清单，不等同于状态迁移 |
| `--import-phonon SAVE` | 仅 epw_phonon；已有 SCF／NSCF 下接续外部原始 PH，支持检查、准备与恢复 |
| `--export-comparison S --output D` | 仅 epw_wannier；从既有来源导出比较数据，不补缺失科学计算 |
| `--collect -w W` | 后两程序收集已有比较／超导数据，不生成缺失科学输出 |
| `--plot [--results R]` | 无 R 时按支持的阶段现场重绘；带 R 时重绘既有包，可用 `--formats` 选择格式 |
| `--results-check --results R [--json]` | 核查既有结果数据和图；操作成功与科学完整性分开 |
| `--probe -s/-r` | 保留未接入／分配要求的显式报错，不能当作成功的版本探测 |

## 与 qeflow 的实际复用关系

| 能力 | 共用实现与当前范围 |
| --- | --- |
| 结构解析 | 共用 qf-crystal 的 POSCAR 解析，不维护第二套解析器 |
| SCF／NSCF 输入 | 共用 qf-input PW 渲染器，EPW 适配原生阶段配置、显式赝势、prefix 与原子质量 |
| NSCF 网格 | 共用完整均匀 crystal 点表生成器，按实际输入／XML 核对点数、坐标、权重与 FFT；不以 SCF 不可约点数替代完整网格 |
| PH 输入与执行 | 共用 qf-input namelist 与 qeflow 声子库；nosplit／coords／index 按所选模式归集，PH 使用 SCF 独立副本 |
| 完成检查 | 共用 qf-io 原生日志检查；正常结束和数值收敛分别判断，EPW 另查直接响应数据及矩阵格式 |

共用库不表示配置协议相同，也不能用普通 QE 声子成功标记跳过 EPW 响应数据、扰动模式、完整电子态和矩阵元检查。电荷密度／原始波函数的排除边界仍遵循 0067。

## 常见问题

**Q：旧工作区如何迁移？**

A：用 `--migrate-state` 转换旧编号状态；用 `--import-work` 导入旧科学数据。两者作用不同，迁移前须核对活动／未知作业。

**Q：如何判断某阶段是否需要重算？**

A：查看 `epw-state/<编号阶段名>.log` 的 `[Stage]`、`Status`、`Scientific status`、失败原因和下一步，再结合公开命令的实际检查；日志中的完成文字不能单独证明产物有效。

**Q：修改配置后哪些阶段会失效？**

A：目标阶段会自动检测其科学参数变化，并检查直接输入。当前不递归追踪整条上游来源，也不自动重建所有下游；按数据依赖依次执行受影响阶段，单改绘图范围只需重绘。

**Q：如何只重绘图表不重算？**

A：现场使用 `epw_wannier -c "input.toml" --plot`，第 3 步用 `epw_phonon --stage electronic --plot`；独立包用 `--plot --results "run/results"`。缺数据会报告，不启动科学补算。

---

**提示**：文档中的“当前／最新”基于 2026-11-17 源码状态；历史验收保留当时语境，不等于当前默认行为。源码、固定安装入口和真实科学验收分别核对。
