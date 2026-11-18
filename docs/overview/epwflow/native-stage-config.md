# 按实际调用填写原生参数

外部 TOML 使用 `schema_version = 2`，`stages` 以八个计算目录名为主段。主段直接填写主调用的原生字段，其他调用放在用途子段；程序内部区分 CONTROL、SYSTEM、ELECTRONS 等输入段。三个计算 CLI 及阶段选择方式保持。

历史版本 2 的同层调用段仍可读取并按原格式输出；新旧主段混写、未知用途子段、科学别名以及程序／namelist 额外嵌套均拒绝。`Config::to_stage_toml()` 可导出八主阶段表达，格式变化不进入阶段科学标识。早期程序／namelist 多层文件仍只允许既有受控导入路径，不属于此次兼容范围。

```toml
[stages."1.qe-scf"]
calculation = "scf"
prefix = "MgB2"
ecutwfc = 40.0 # Ry
ecutrho = 320.0 # Ry
conv_thr = 1e-6 # Ry
```

以上是字段位置示例，实际运行需要完整配置。

## 改哪个参数，影响哪个环节

| 主阶段 | 主调用及用途子段 |
| --- | --- |
| `1.qe-scf` | SCF PW；原生字段直接放主段 |
| `2.qe-nscf` | NSCF PW；`K_POINTS=[A,B,C]` 简写网格 |
| `3.qe-electronic` | 主段为能带 PW；`.bands` 为 bands.x，`.dos` 为 dos.x，`.projwfc` 为 projwfc.x |
| `4.qe-phonon-nosplit/index/coords` | 只选实际模式；主段为 PH，`.q` 为 index/coords 逐 q PH，`.q2r/.dispersion/.dos` 为后处理 |
| `5.epw-wannier` | 主段为 EPW；`.wannier90` 为内嵌 Wannier90 参数，`.postw90` 为 DOS 重读 |
| `6.epw-coarse` | 主段为粗网格 EPW；`.q2r/.matdyn/.comparison` 为独立比较调用 |
| `7.epw-fine` | 主段为细网格 EPW；可选 `.spectra.名称` 为完整独立的额外展宽调用 |
| `8.epw-superconductivity` | `.iso/.aniso` 两个独立求解调用，各自填写 temps、muc、nsiter 等 |

例如 `[stages."5.epw-wannier".wannier90]` 的 `bands_plot=true` 与 `.postw90` 的 `bands_plot=false` 分别用于能带和 DOS。没有独立的 `5.wannier90` 主阶段。修改 `stages."3.qe-electronic".dos.degauss` 不会改变 `.projwfc` 或主段能带 PW 的展宽。必需一致的前缀、网格、模型维数仍做约束检查，冲突列出双方位置和值，不自动改值。

历史平层调用到用途子段的唯一映射在读取、导出和错误位置转换中复用，内部科学视图、输入生成和复用键不变；同名参数保留各自原生名。完整配置必须包含全部八个主阶段；不支持的子段即使为空也报错。

## 公共参数与阶段网格简写

`[baseparams]` 支持 `ATOMIC_SPECIES`、`prefix`、`ecutwfc`、`ecutrho`、`Emin`、`Emax`、`DeltaE` 及工作流字段 `fermi_source`。按原生调用的参数所有权回填：阶段显式值优先，未填写时才用公共值；非法显式值直接报错。物种可用 `[["Mg",24.305,"Mg.upf"],["B",10.811,"B.upf"]]`，整组取值，保留顺序、质量和文件名。历史卡片正文仍可读取。

`fermi_source = "f2"` 或 `"fine_fixed_electrons"` 仅回填第 5、6、7 步 workflow 的同名字段；阶段显式值优先，各阶段分别取值。公共和阶段都未填写时明确报错，无隐式默认；非法值不回退。序列化保留公共设置和显式覆盖，同一有效来源的全局／阶段表达不改变科学身份。该配置适配沿用现有科学链，不向原生 namelist 注入此字段。

`nk/nq/nkf/nqf` 可在支持对应三个方向字段的阶段或用途子段写成三正整数数组，例如 PH `nq=[2,2,1]`、EPW `nk=[4,4,2]`、`nkf=[8,8,4]`。同一数组不能与自身方向字段混写；所有网格都禁止放进 `[baseparams]`，包括 `K_POINTS`、`dos_kmesh` 和方向字段。SCF／NSCF 的 `K_POINTS` 与 postw90 的 `dos_kmesh` 不改名。

公共字段只用于支持它的调用，不把 QE DOS 的 `Emin/Emax/DeltaE` 注入 postw90 或 Wannier90；相同有效参数的科学标识不依赖表内字段顺序。Ubuntu MgB₂ 当前输入已采用八主阶段及此简写，原科学数值保留。来源定位与数组表达仅作为内存元数据，不参与科学身份或生成独立参数记录。

## 单位、数组和卡片

QE 的 ecutwfc/ecutrho/degauss/conv_thr 使用 Ry；DOS/PROJWFC 的 Emin/Emax/DeltaE 使用 eV，其 degauss 仍为 Ry。EPW fsthick/degaussw/wscut/gap_edge 为 eV，degaussq 为 meV，eps_acoustic 为 cm⁻¹，temps 为 K。Wannier90 能窗为绝对 eV，conv_tol 为 Å²。单位写在注释，不加字段别名。

amass、temps 等数组保持原生名字，输入生成 Fortran 下标；nstemp 必须与自身 temps 长度一致。ATOMIC_SPECIES 支持卡片正文或上述物种二维数组；SCF 的 K_POINTS 支持三正整数网格或六整数网格与偏移数组，后三项只能为 0/1。旧 SCF 卡片继续读取，automatic 卡片序列化为数组，crystal 卡片保持原类型。NSCF 沿用 0060 三整数网格，由 Rust 展开，完整 EPW 工作流仍要求与 SCF 同维度且零位移，完成及复用时核对实际输入和输出，详见 [NSCF 网格与 FFT](nscf-grid.md)。独立 QE 声子工作流的 `[qe_phonon.scf].kpoints` 使用同样的三／六元素数组并允许非零偏移；独立 k_shift 字段明确报错并提示迁移。

## 工作流与自动绑定

用户可编辑的设置单独放在顶层 workflow，例如 `[workflow."5.epw-wannier"]` 保存选窗和输入来源，`[workflow."8.epw-superconductivity"]` 保存 task、mode 和搜索设置。调度文件头使用 `[slurm].header` 多行字符串，程序启动使用 programs；整个 `[resources]` 已移除。MPI 启动参数仅由 launcher 指定，Slurm 分配由用户 header 决定，两者不强制相等。

TOML 无需填写 outdir、pseudo_dir、nat 等自动绑定项或 derived 表。程序内部登记有限规则，按结构、任务目录及核验后的数据生成；手填同一目标仍拒绝双重定义，未配置且未登记的生成参数同样停止。

科学输入不再前置批量参数来源注释，也不再保存 `parameter-map.json` 或展开后的固定配置。每次配置驱动 CLI 比较原文字节，变化后在解析后的 `work_dir/epw-state/` 保存递增的 `NNNN-config.toml`；阶段日志分别指向当前输入版本和本阶段实际计算使用的版本。内部有效配置仅作为解析原文和生成原生输入的内存对象。

快照按最大编号原文字节比较，相同不新增；恢复历史内容仍保存下一编号。原始目录和 SHA-256 随每版写入第 1 步日志的 `配置快照` 小节，重跑 SCF 保留这些记录；读取快照时，结构、赝势、程序和工作区等相对路径仍按原输入目录解析，排队任务继续读取其冻结版本。

原输入旁的旧编号快照会先核验工作区归属、目标内容及旧任务终态，独立写入状态目录、更新日志引用后才移除旧位置文件。冲突、缺来源记录或活动／未知旧任务时保留原文件并明确报错，不覆盖科学输入，不自动重算；迁移中断可重试。细节与验收见 [0064](../../specs/0064-epw-readable-state-logs/acceptance.md)。

八主阶段示例见 [SiP₂ TOML](../../../examples/SiP2/epwinput.toml)，历史平层示例见 [MgB₂ TOML](../../../examples/MgB2/sugon-amd/run/epwflow/epwinput.toml)，细节见 [0061 契约](../../specs/0061-epw-flat-stage-config/contracts/config.md)与[验收记录](../../specs/0061-epw-flat-stage-config/acceptance.md)。注释移除及来源追溯见 [0062](../../specs/0062-epw-clean-input-headers/origin.md)。源码、私有构建和默认安装分别核对，旧安装不能据此视为已支持新格式。

## 已有真实验证边界

0059 在 2026-10-26 使用当时的多层配置完成 PBE MgB₂ 小网格八阶段重算，iso/aniso 的 20/30/40 K 六点收敛且数据有效。这是历史科学证据，不是本轮配置格式重算；不构成 Tc 或网格收敛验收，见 [0059 验收](../../specs/0059-epw-native-stage-config/acceptance.md)。

## 组合图显示范围

input.toml 的顶层 [plot] 提供 electronic_energy_min_ev 与 electronic_energy_max_ev，独立默认 −3／＋3，单位 eV、相对 E_F。两值必须有限且严格递增，第 3、5 步共用；不参与科学阶段身份，不改变任何原生程序输入或实际拟合窗。

第 3、5 步 plot/ 各生成两张电子组合图；全范围取 QE 路径能带极值。第 4、6 步 plot/ 为 0 到最高路径频率的一张声子组合图。自包含 .plot.json 支持独立重绘。

两张电子组合图的 DOS 横轴都用 `[plot]` 能窗与当前可见能窗交集内所有待绘曲线的最大值，加 10% 留白并向上整理刻度。全范围图仍显示全部 QE 路径能级，深能级尖峰只在图上裁剪，用箭头及真实峰值标注；重叠标记显示最大峰，不修改 DOS 数组、积分或科学来源。参考窗无正 DOS 时注明并回退可见数据，全零时保留有效正刻度；声子范围规则不变。

例如 E_F±3 eV 内最大 DOS 为 4.24，则横轴上限由 4.664 向上整理为 5 states/eV/cell；不受 −42 eV 附近更高峰影响。无需新增 TOML 参数，修改现有能窗后使用 `epw_phonon -c "input.toml" --stage electronic --plot` 或 `epw_wannier -c "input.toml" --plot` 原位重绘。自包含图参保存独立的 DOS 参考窗；旧独立包缺字段时只读同目录近费米图的边界，缺失则默认 ±3 eV。

## 外部程序路径与命令行参数

全部外部程序在一个 `[programs]` 内平铺配置，路径与参数分别使用 `*_x`、`*_args`；不使用 `[programs.pw]` 等子表。每个后处理程序必须有自己的显式路径，不从 pw／ph 目录或 PATH 推导。

```toml
[programs]
launcher = ["mpirun", "-np", "12"]
environment_script = "/path/to/environment.sh"
pw_x = "/path/to/qe/pw.x"
pw_args = []
ph_x = "/path/to/phonon/ph.x"
ph_args = []
epw_x = "/path/to/epw/epw.x"
epw_args = ["-npool", "12"]
bands_x = "/path/to/postprocessing/bands.x"
bands_args = []
dos_x = "/path/to/postprocessing/dos.x"
dos_args = []
projwfc_x = "/path/to/postprocessing/projwfc.x"
projwfc_args = []
q2r_x = "/path/to/phonon/q2r.x"
q2r_args = []
matdyn_x = "/path/to/phonon/matdyn.x"
matdyn_args = []
postw90_x = "/path/to/wannier/postw90.x"
postw90_args = []
```

路径示例须替换为实际安装位置，程序可以分布在不同目录。可选独立程序使用 `wannier90_x`／`wannier90_args` 和 `pw2wannier90_x`／`pw2wannier90_args`；EPW 内嵌 Wannier90 不需要独立可执行文件。未使用的可选程序可不填路径；实际消费时缺少路径会返回对应字段名，有非空 args 而无路径则直接拒绝。所有已配置路径按配置文件所在位置解析并检查可执行性。

空列表或省略 args 表示不追加额外命令行参数；不会生成空占位参数，不继承其他程序 args，也不再自动添加 pencil 的 `-pd .true.`。如需该选项，用户在对应程序的 args 中明确填写。launcher 和计算必需的输入／种子参数仍保留，原生程序并行限制仍适用。

EPW 从 Wannier、粗网格到细网格、超导求解全部禁用多 image；省略 image 或显式 `-nimage 1` 可用，`-ni`／`-nimage`／`-nimages` 多 image 均拒绝。PH image 参数保留自身规则。`-nk`／`-npool`／`-npools` 为 pool 参数别名，只能选择一个填写。当前单 image EPW 仍要求每 pool 一个 MPI 进程。

旧程序子表需要用户改为平铺字段；不自动改写历史配置快照、已生成脚本或计算数据，不保证旧工作区透明续算。源码、回归及安装边界见 [0065 验收](../../specs/0065-slurm-header-launcher/acceptance.md)。
