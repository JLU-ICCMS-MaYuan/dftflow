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

## 单位、数组和卡片

QE 的 ecutwfc/ecutrho/degauss/conv_thr 使用 Ry；DOS/PROJWFC 的 Emin/Emax/DeltaE 使用 eV，其 degauss 仍为 Ry。EPW fsthick/degaussw/wscut/gap_edge 为 eV，degaussq 为 meV，eps_acoustic 为 cm⁻¹，temps 为 K。Wannier90 能窗为绝对 eV，conv_tol 为 Å²。单位写在注释，不加字段别名。

amass、temps 等数组保持原生名字，输入生成 Fortran 下标；nstemp 必须与自身 temps 长度一致。ATOMIC_SPECIES 保存卡片正文；SCF 的 K_POINTS 保存类型及正文。NSCF 沿用 0060 三整数网格，由 Rust 展开，要求与 SCF 同维度且零位移，完成及复用时核对实际输入和输出，详见 [NSCF 网格与 FFT](nscf-grid.md)。

## 工作流与自动绑定

用户可编辑的设置单独放在顶层 workflow，例如 `[workflow."5.epw-wannier"]` 保存选窗和输入来源，`[workflow."8.epw-superconductivity"]` 保存 task、mode 和搜索设置。resources/programs 保留调度与程序设置。

TOML 无需填写 outdir、pseudo_dir、nat 等自动绑定项或 derived 表。程序内部登记有限规则，按结构、任务目录及核验后的数据生成；手填同一目标仍拒绝双重定义，未配置且未登记的生成参数同样停止。

科学输入不再前置批量参数来源注释，也不再保存 `parameter-map.json` 或展开后的固定配置。每次配置驱动 CLI 比较原文字节，变化后在解析后的 `work_dir/epw-state/` 保存递增的 `NNNN-config.toml`；阶段日志分别指向当前输入版本和本阶段实际计算使用的版本。内部有效配置仅作为解析原文和生成原生输入的内存对象。

快照按最大编号原文字节比较，相同不新增；恢复历史内容仍保存下一编号。原始目录和 SHA-256 随每版写入第 1 步日志的 `配置快照` 小节，重跑 SCF 保留这些记录；读取快照时，结构、赝势、程序和工作区等相对路径仍按原输入目录解析，排队任务继续读取其冻结版本。

原输入旁的旧编号快照会先核验工作区归属、目标内容及旧任务终态，独立写入状态目录、更新日志引用后才移除旧位置文件。冲突、缺来源记录或活动／未知旧任务时保留原文件并明确报错，不覆盖科学输入，不自动重算；迁移中断可重试。细节与验收见 [0064](../../specs/0064-epw-readable-state-logs/acceptance.md)。

八主阶段示例见 [SiP₂ TOML](../../../examples/SiP2/epwinput.toml)，历史平层示例见 [MgB₂ TOML](../../../examples/MgB2/sugon-amd/run/epwflow/epwinput.toml)，细节见 [0061 契约](../../specs/0061-epw-flat-stage-config/contracts/config.md)与[验收记录](../../specs/0061-epw-flat-stage-config/acceptance.md)。注释移除及来源追溯见 [0062](../../specs/0062-epw-clean-input-headers/origin.md)。源码、私有构建和默认安装分别核对，旧安装不能据此视为已支持新格式。

## 已有真实验证边界

0059 在 2026-10-26 使用当时的多层配置完成 PBE MgB₂ 小网格八阶段重算，iso/aniso 的 20/30/40 K 六点收敛且数据有效。这是历史科学证据，不是本轮配置格式重算；不构成 Tc 或网格收敛验收，见 [0059 验收](../../specs/0059-epw-native-stage-config/acceptance.md)。
