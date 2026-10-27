# 阶段原生参数契约

根版本为 `schema_version = 2`。旧版和混合旧结构直接拒绝，不提供迁移工具。

## 所有权和输入位置

每个表完整保存本次调用自己的科学参数。下表中的路径均位于 `stages."编号目录"` 内；重复名称独立填写，修改值不会复制到其他阶段。

| 编号目录 | 子任务、程序和输入段 | 影响的实际输入 |
|---|---|---|
| `1.qe-scf` | `pw.CONTROL`、`SYSTEM`、`ELECTRONS` 和原生卡片 | SCF `scf.in` |
| `2.qe-nscf` | `pw.CONTROL`、`SYSTEM`、`ELECTRONS` 和原生卡片 | 完整网格 `nscf.in` |
| `3.qe-electronic` | `qe_eband.pw`、`qe_eband.bands.BANDS` | 路径 PW 与 bands 输入 |
| 同上 | `qe_eledos.dos.DOS`、`qe_eledos.projwfc.PROJWFC` | 总 DOS 和投影 DOS 输入 |
| `4.qe-phonon-nosplit/index/coords` | `ph.INPUTPH`、拆分模式的 `q.ph.INPUTPH` | 初始化或完整 PH、每个 q 点 PH |
| 同上 | `q2r.INPUT`、`dispersion.matdyn.INPUT`、`dos.matdyn.INPUT` | 力常数、色散和声子 DOS 输入 |
| `5.epw-wannier` | `epw.INPUTEPW`、`wannier90` | EPW 和内嵌 Wannier90 |
| 同上 | `electron_dos.postw90` | 重读同一模型的 DOS |
| `6.epw-coarse` | `epw.INPUTEPW` | 复用已验证 Wannier 规范的粗 EPC |
| 同上 | `comparison.q2r.INPUT`、`comparison.matdyn.INPUT`、`comparison.epw.INPUTEPW` | 比较声子后处理和 EPW 重读 |
| `7.epw-fine` | `epw.INPUTEPW` | 矩阵和谱；`temps` 独立填写 |
| 同上，可选 | `spectra."子任务名".epw.INPUTEPW` | 额外展宽；每个子任务完整设置，不继承主任务 |
| `8.epw-superconductivity` | `iso.epw.INPUTEPW`、`aniso.epw.INPUTEPW` | 两个独立求解作业 |

## 名称、单位和数据类型

| 程序 | 原生参数 | 单位或表示 |
|---|---|---|
| PW | `ecutwfc`、`ecutrho`、`degauss`、`conv_thr` | Ry；原值传递 |
| DOS / PROJWFC | `degauss` | Ry；不再读取第五步 postw90 的展宽 |
| DOS / PROJWFC | `Emin`、`Emax`、`DeltaE` | eV |
| PH | `tr2_ph`、`amass` | 原生响应阈值、原子质量单位 |
| EPW | `fsthick`、`degaussw`、`wscut`、`gap_edge` | eV |
| EPW | `degaussq`、`delta_qsmear` | meV |
| EPW | `eps_acoustic` | cm⁻¹ |
| EPW | `temps`、`nstemp` | K 数组及数组长度 |
| EPW | `muc`、`nsiter`、`conv_thr_iaxis` | 原生库仑参数、求解迭代数、收敛阈值 |
| Wannier90 | `num_iter`、`dis_num_iter`、`conv_window` | 原生迭代设置 |
| Wannier90 | `conv_tol` | Å² |
| Wannier90 | `dis_win_min/max`、`dis_froz_min/max` | 绝对 eV，采用 QE 的能量零点 |
| postw90 | `dos_smr_fixed_en_width`、`dos_energy_step` | eV |

不接受 `ecutwfc_ry`、`mu_star`、`max_iterations` 等科学别名。仅支持当前执行路径已接入的原生参数；未知字段有完整配置位置的报错。

数组保留原生变量基本名，例如 `amass = [24.305, 10.811]` 和 `temps = [20.0, 30.0, 40.0]`；namelist 生成 `amass(1)`、`temps(1)` 等从 1 开始的原生下标。物种顺序对应结构和 `ATOMIC_SPECIES`。

## 卡片与派生值

`pw.ATOMIC_SPECIES` 是不含标题行的原生卡片正文，元素、质量和赝势文件均明确填写。相对赝势文件必须实际对应根 `pseudos` 绑定的文件。冻结计算输入时保留卡片文件名和目录关系。

`pw.K_POINTS` 包含原生类型和正文。SCF 可用 `automatic`；NSCF 直接保存完整 `crystal` 点表，检查点数、坐标、权重、直积完整性、重复点以及零位移。不会用 `automatic` 静默替换显式点表。路径 PW 的点由该阶段 `workflow.points_per_edge` 与已绑定 KPATH 生成。

`CELL_PARAMETERS`、`ATOMIC_POSITIONS`、`ibrav/nat/ntyp` 由已绑定 POSCAR 得出。运行路径、固定输出文件名、已验证 NSCF 费米能、自动能窗和当前请求的温度子集均须有明确推导来源。

`workflow.derived` 使用如 `"epw.INPUTEPW.fermi_energy"` 的目标名登记规则。手填同一目标与声明推导同时存在时拒绝；生成器发现未配置且未登记的原生科学参数时也拒绝。内嵌 `wdata`、`proj`、`bands_skipped` 明确追溯本阶段 Wannier90 原生参数或已绑定的路径/本征值。

## 校验、身份和恢复

- 必须一致的前缀、粗细网格、Wannier 维数及模式开关在执行前比较，报错列出双方配置路径、值与原因；不会自动修正一侧。
- 第七步与第八步的温度相互独立；iso 与 aniso 使用各自 `muc`、`wscut`、`nsiter`、混合、温度和初值。
- 恢复多温度任务只从该方法固定温度表选取未完成温度；Tc 搜索则按已固定搜索规则构造当前请求。不会读取另一方法的温度。
- 阶段科学指纹包含所属原生配置。拟合后的 DOS、粗网格比较后处理有自己的参数与回执，配置改变必须使对应回执失效。
- 实际 namelist 逐参数注释指向 TOML 路径或推导规则；提交快照的 `parameter-map.json` 列出配置值和来源。后处理输入另有 `*.parameters.json`，绑定实际输入哈希。

`programs`、`resources`、`budget` 与每阶段 `workflow` 为调度、记账、来源、拆分、选窗或搜索设置。它们不是另一组科学参数默认值。原共享消费记录保留；0058 独立取消的时间和存储限制不在本项恢复。

## 0060 对 NSCF 点表契约的替代

上述 NSCF 完整点表是 0059 的历史设计。用户随后要求简写，现行契约由 [0060](../../0060-epw-nscf-grid-contract/spec.md) 替代：`stages."2.qe-nscf".K_POINTS = [A, B, C]`（层级已由 0061 平层格式接续），实际原生卡片由 Rust 展开，SCF 同维度且零位移。NSCF 同时支持 `nscf` 与 `bands`，仍由 pw.x 计算；SYSTEM 允许完整正整数密度／平滑 FFT 三元组。其他阶段独立配置原则保留。
