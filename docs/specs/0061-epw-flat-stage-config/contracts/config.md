# 平层配置契约

根字段继续使用 `schema_version=2`、work_dir、outdir、structure、pseudos、programs、resources、budget、可选 layout/kpath；科学字段仅允许放在下表对应的同层段内。

| 科学配置段 | 实际调用或设置 |
| --- | --- |
| `1.qe-scf` | SCF PW，程序内部区分 CONTROL/SYSTEM/ELECTRONS 和卡片 |
| `2.qe-nscf` | NSCF PW；0060 简写 K_POINTS=[A,B,C] |
| `3.qe-bands` | 能带 PW |
| `3.qe-bands-post` | bands.x |
| `3.qe-dos` | dos.x |
| `3.qe-pdos` | projwfc.x |
| `4.qe-phonon-nosplit/index/coords` | 选择其中一种，PH 完整调用或初始化 |
| `4.qe-phonon-index-q/4.qe-phonon-coords-q` | 所选拆分模式的逐 q PH |
| `4.qe-q2r` | 声子 q2r |
| `4.qe-dispersion` | 色散 matdyn |
| `4.qe-phdos` | DOS matdyn |
| `5.epw-wannier` | 驱动 Wannier 化的 EPW |
| `5.wannier90` | 内嵌 Wannier90 的原生参数，与 EPW 的 num_iter 分别填写 |
| `5.postw90-dos` | Wannier 模型电子 DOS 重读 |
| `6.epw-coarse` | 粗网格 EPW |
| `6.qe-q2r/6.qe-matdyn/6.epw-comparison` | 比较所需的三次独立调用 |
| `7.epw-fine` | 细网格 EPW，温度独立填写 |
| `7.epw-fine-名称` | 可选额外谱展宽调用，完整独立填写 |
| `8.epw-iso/8.epw-aniso` | 两个独立求解器，分别填写全部原生参数 |

这些名称划分配置所有权，不新增或重命名八个计算目录。

段内标量、数组和原生卡片直接填写，禁止任何子表。例如 `ecutwfc` 直接归入对应 PW 的 SYSTEM，`conv_thr` 归入 ELECTRONS。数组 `amass/temps` 保持原生名字，实际输入使用 Fortran 下标；`ATOMIC_SPECIES` 和 SCF `K_POINTS` 仍为卡片正文，NSCF 沿用 0060 三整数网格。未知键或无法唯一确定所属输入段时拒绝。

QE `ecutwfc/ecutrho/degauss/conv_thr` 为 Ry；DOS/PROJWFC `degauss` 仍为 Ry，Emin/Emax/DeltaE 为 eV。EPW fsthick/degaussw/wscut/gap_edge 为 eV，degaussq 为 meV，temps 为 K，eps_acoustic 为 cm⁻¹。Wannier 能窗为绝对 eV，conv_tol 为 Å²。单位写入注释，不改参数名。

可编辑设置移到顶层 `[workflow."用途"]`：3.qe-bands 管理路径点数，所选 PH 段管理模式所属路径和绘图设置，5.epw-wannier 管理选窗和来源，6.epw-coarse 管理比较，7.epw-fine 管理来源，8.epw-superconductivity 管理 task/mode、谱选择及原有搜索设置。调度仍由 resources/programs 管理。

无公开 derived 表。outdir、pseudo_dir、结构维数、受控输出名、绑定费米能、EPW 内嵌 Wannier 数据和自动能窗等仅按代码中的有限规则生成；手填同一目标仍报双重定义。用户填写的 Wannier90 能窗不等于手填 EPW 的自动传递目标，二者有明确的不同调用归属。

输入注释与参数回执引用真实平层字段。自动值标为“内部推导规则”；parameter-map.json 保留实际配置位置和内部绑定来源，不要求用户在 TOML 中声明。旧多层输入拒绝，历史冻结现场不自动修改。

## 0062 对输入说明方式的替代

[0062](../../0062-epw-clean-input-headers/spec.md) 删除输入头部批量来源注释；平层来源与内部绑定仅记录在独立 JSON 中，实际输入哈希仍绑定回执。上文输入注释条款保留为原版本记录，不再代表当前生成行为。
