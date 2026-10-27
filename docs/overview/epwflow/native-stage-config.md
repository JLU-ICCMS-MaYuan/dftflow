# 按实际调用填写原生参数

外部 TOML 使用 `schema_version = 2`，科学参数仅写到 `[stages."编号.用途"]`。段内直接填写原生字段，程序内部负责区分 CONTROL、SYSTEM、ELECTRONS 等输入段；旧多层配置和科学别名明确拒绝，不提供迁移工具。三个计算 CLI 及现有阶段选择方式保持。

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

| 配置段名称 | 实际调用或设置 |
| --- | --- |
| `1.qe-scf` | SCF PW |
| `2.qe-nscf` | NSCF PW，显式完整 K_POINTS 卡片 |
| `3.qe-bands` | 能带 PW |
| `3.qe-bands-post` | bands.x 后处理 |
| `3.qe-dos/3.qe-pdos` | dos.x 总 DOS、projwfc.x 投影 DOS |
| `4.qe-phonon-nosplit/index/coords` | 选择一种模式，PH 完整调用或初始化 |
| `4.qe-phonon-index-q/4.qe-phonon-coords-q` | 对应模式的逐 q PH |
| `4.qe-q2r/4.qe-dispersion/4.qe-phdos` | q2r、色散 matdyn、DOS matdyn |
| `5.epw-wannier/5.wannier90/5.postw90-dos` | 驱动 EPW、内嵌 Wannier90、电子 DOS 重读 |
| `6.epw-coarse` | 粗网格 EPW |
| `6.qe-q2r/6.qe-matdyn/6.epw-comparison` | 比较所需的独立调用 |
| `7.epw-fine` | 细网格 EPW，自身温度独立填写 |
| `7.epw-fine-名称` | 可选额外谱展宽调用，完整独立填写 |
| `8.epw-iso/8.epw-aniso` | 两种独立求解，各自填写 temps、muc、nsiter 等参数 |

这些段名区分参数所有权，计算目录仍为八阶段。同名参数在每个调用中分别设置；例如改 `stages."3.qe-dos".degauss` 不会修改 PDOS 或能带 PW 的展宽。必需一致的数据前缀、网格、模型维数只做约束检查，冲突列出双方位置、值和原因，不自动改值。

## 单位、数组和卡片

QE 的 ecutwfc/ecutrho/degauss/conv_thr 使用 Ry；DOS/PROJWFC 的 Emin/Emax/DeltaE 使用 eV，其 degauss 仍为 Ry。EPW fsthick/degaussw/wscut/gap_edge 为 eV，degaussq 为 meV，eps_acoustic 为 cm⁻¹，temps 为 K。Wannier90 能窗为绝对 eV，conv_tol 为 Å²。单位写在注释，不加字段别名。

amass、temps 等数组保持原生名字，输入生成 Fortran 下标；nstemp 必须与自身 temps 长度一致。ATOMIC_SPECIES 保存卡片正文；SCF 的 K_POINTS 保存类型及正文。NSCF 的 K_POINTS 保存完整 crystal 卡片；0060 简写网格接续属于独立变更。

## 工作流与自动绑定

用户可编辑的设置单独放在顶层 workflow，例如 `[workflow."5.epw-wannier"]` 保存选窗和输入来源，`[workflow."8.epw-superconductivity"]` 保存 task、mode 和搜索设置。resources/programs 保留调度与程序设置。

TOML 无需填写 outdir、pseudo_dir、nat 等自动绑定项或 derived 表。程序内部登记有限规则，按结构、任务目录及核验后的数据生成；手填同一目标仍拒绝双重定义，未配置且未登记的生成参数同样停止。

科学输入不再前置批量参数来源注释。直接准备模式的 epw-state/parameter-map.json、冻结提交的 parameter-map.json 和后处理参数回执指向平层配置位置并记录内部绑定来源；输入回执绑定实际文件哈希。内部有效配置供输入、指纹、回执及恢复共用，不构成可公开输入的第二种格式。旧冻结配置与旧结果不会自动改写；格式验证不意味着旧现场已完成迁移。

完整示例见 [MgB₂ TOML](../../../examples/MgB2/sugon-amd/run/epwflow/epwinput.toml)，细节见 [0061 契约](../../specs/0061-epw-flat-stage-config/contracts/config.md)与[验收记录](../../specs/0061-epw-flat-stage-config/acceptance.md)。注释移除及来源追溯见 [0062](../../specs/0062-epw-clean-input-headers/origin.md)。源码、私有构建和默认安装分别核对，旧安装不能据此视为已支持新格式。

## 已有真实验证边界

0059 在 2026-10-26 使用当时的多层配置完成 PBE MgB₂ 小网格八阶段重算，iso/aniso 的 20/30/40 K 六点收敛且数据有效。这是历史科学证据，不是本轮平层格式重算；不构成 Tc 或网格收敛验收，见 [0059 验收](../../specs/0059-epw-native-stage-config/acceptance.md)。
