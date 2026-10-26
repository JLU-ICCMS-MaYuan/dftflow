# 按计算阶段填写原生参数

外部 TOML 仅支持 `schema_version = 2`。旧配置和科学别名会被拒绝，不提供迁移工具。三个计算 CLI 及其 `-c`、`--stage`、`--resume` 用法保持。

## 改哪个参数，影响哪个环节

| TOML 中 `stages` 后的位置 | 实际调用与输入 |
|---|---|
| `"1.qe-scf".pw` | SCF：`CONTROL/SYSTEM/ELECTRONS`、`ATOMIC_SPECIES/K_POINTS` |
| `"2.qe-nscf".pw` | NSCF：独立的 PW 参数与完整原生 `crystal` 点表 |
| `"3.qe-electronic".qe_eband.pw` | 路径电子能带 PW |
| `"3.qe-electronic".qe_eband.bands.BANDS` | `bands.x` 后处理 |
| `"3.qe-electronic".qe_eledos.dos.DOS` | 总电子 DOS |
| `"3.qe-electronic".qe_eledos.projwfc.PROJWFC` | 元素投影 DOS |
| `"4.qe-phonon-coords".ph.INPUTPH` | PH 初始化；目录后缀也可为 nosplit/index |
| 同阶段的 `q.ph.INPUTPH` | 拆分模式的每个 q 点响应 |
| 同阶段的 `q2r.INPUT/dispersion.matdyn.INPUT/dos.matdyn.INPUT` | 力常数、色散、声子 DOS |
| `"5.epw-wannier".wannier90/epw.INPUTEPW` | 内嵌 Wannier90 拟合及所属 EPW 输入 |
| 同阶段的 `electron_dos.postw90` | 重读该模型计算电子 DOS |
| `"6.epw-coarse".epw.INPUTEPW` | 粗网格 EPC |
| 同阶段的 `comparison` | 比较用 q2r、matdyn 和 EPW 重读 |
| `"7.epw-fine".epw.INPUTEPW` | 细网格矩阵、α²F、λ，自身的温度 |
| `"8.epw-superconductivity".iso.epw.INPUTEPW` | 独立 iso 求解 |
| `"8.epw-superconductivity".aniso.epw.INPUTEPW` | 独立 aniso 求解 |

例如，SCF 的截断能是 `stages."1.qe-scf".pw.SYSTEM.ecutwfc`，NSCF 在另一处单独填写。不会用一个公共字段同时改动两次科学调用。需要一致的数据前缀、网格和模型维数只做约束检查；冲突列出双方位置、值和原因，不自动改值。

## 单位、数组和卡片

QE 的 `ecutwfc/ecutrho/degauss/conv_thr` 使用 Ry；DOS/PROJWFC 的能量范围与步长使用 eV。EPW 的 `fsthick/degaussw/wscut/gap_edge` 为 eV，`degaussq` 为 meV，`eps_acoustic` 为 cm⁻¹，`temps` 为 K。Wannier90 能窗采用 QE 能量零点的绝对 eV，`conv_tol` 为 Å²。单位写在注释，参数名不加单位后缀。

`amass=[24.305,10.811]`、`temps=[20,30,40]` 生成原生索引数组；`nstemp` 必须与自身温度列表长度一致。`ATOMIC_SPECIES` 保存物种卡片正文，`K_POINTS` 保存类型及正文。原生卡片保留顺序与内容，并检查物种、赝势绑定及完整网格。

## 工作流与参数来源

每阶段 `workflow` 保存自动选窗、输入来源、路径采样、PH 拆分和搜索等设置。`workflow.derived` 明确登记运行路径、结构、费米能、自动选窗、内嵌 `wdata` 等目标的来源。同一目标同时手填和推导会被拒绝；生成器遇到既未配置也未登记的科学参数同样停止。

第 5、6 步分别填写路径采样数，并与复用的第 3 步参考点位检查一致。第 7 步温度独立设置；第 8 步两个方法分别填写 `muc/nsiter/broyden_beta/gap_edge/temps` 等全部求解参数。多温度恢复只从该方法的固定温度表选取未完成项。

实际 namelist 注释指向配置位置或推导规则。固定输入快照包含 `parameter-map.json`；后处理的 `*.parameters.json` 同时记录配置和实际输入哈希。阶段指纹及子任务回执读取所属有效配置，变更后按现有依赖规则更新对应环节。

完整配置见 [MgB₂ TOML](../../../examples/MgB2/sugon-amd/run/epwflow/epwinput.toml)，类型和单位明细见[参数契约](../../specs/0059-epw-native-stage-config/contracts/config.md)。源码输入检查、进程回归、默认安装和真实科学计算是不同证据，验收边界见 [0059 记录](../../specs/0059-epw-native-stage-config/acceptance.md)。

## 当前真实验证

2026-10-26，按版本 2 配置清理后完成 PBE MgB₂ 小网格八阶段重算；iso／aniso 的 20／30／40 K 六点均收敛且数据有效，结果包来源和图表检查通过。使用本轮固定的私有构建，默认安装未切换；该结果不构成 Tc 或网格收敛验收。见 [0059 验收](../../specs/0059-epw-native-stage-config/acceptance.md)。
