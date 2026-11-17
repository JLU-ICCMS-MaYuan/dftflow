# 平层配置契约

说明：前部保留最初平层版本的需求及验收记录；2026-10-28 已确认的八主阶段方案见本文后部追加章节，旧的“所有段禁止子表”限制已由该方案替代。

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

## 八主阶段契约（替代上文的无子表展示限制）

新配置只用八个主阶段名。SCF/NSCF 保持平铺；3.qe-electronic 父表为能带 PW，子段为 bands、dos、projwfc。PH 父表为主 PH，子段为 q、q2r、dispersion、dos；nosplit 不设置 q。5.epw-wannier 父表为 EPW，子段为 wannier90、postw90。6.epw-coarse 父表为 EPW，子段为 q2r、matdyn、comparison。7.epw-fine 父表为 EPW，可选 spectra.名称。8.epw-superconductivity 使用 iso、aniso 子段。

子段内仍直接填写原生参数，不再嵌套 pw/CONTROL/SYSTEM/INPUTEPW 等层级。workflow 使用八阶段主名，第 3 步为 3.qe-electronic。历史版本 2 平层调用配置继续读取并保留原格式序列化；新旧段或 workflow 名混写拒绝。科学视图、原生参数数值及身份不随展示格式变化。

## 2026-11-17 公共参数与网格数组契约（待实现）

本节是新增配置需求，覆盖前文对本次公共参数禁止继承、物种仅允许卡片正文的限制；不表示当前解析器已支持。以下为片段示例，未列出完整八阶段配置：

```toml
[baseparams]
ATOMIC_SPECIES = [
  ["Mg", 24.305, "mg_pbe_v1.4.UPF"],
  ["B", 10.811, "b_pbe_v1.4.uspp.F.UPF"],
]
prefix = "MgB2"
ecutwfc = 40.0 # Ry
ecutrho = 320.0 # Ry
Emin = -100.0 # eV，QE DOS／PROJWFC 原生能量轴
Emax = 100.0 # eV
DeltaE = 0.01 # eV

[stages."3.qe-electronic".dos]
DeltaE = 0.02 # 本调用显式覆盖；projwfc 未填写时仍用公共 0.01

[stages."4.qe-phonon-index"]
nq = [2, 2, 1]

[stages."4.qe-phonon-index".dos]
nk = [8, 8, 4]

[stages."6.epw-coarse"]
nk = [4, 4, 2]
nq = [2, 2, 1]
nkf = [4, 4, 2]
nqf = [2, 2, 1]
```

| 公共字段 | 消费边界 |
| --- | --- |
| `ATOMIC_SPECIES` | PW 物种卡片；不替代 `pseudos` 的文件绑定或改写结构 |
| `prefix` | 原生支持 `prefix` 的调用；原有必须一致的前缀仍核验 |
| `ecutwfc/ecutrho` | PW 的 SYSTEM；保持 Ry，不发给 PH／EPW 等不支持的段 |
| `Emin/Emax/DeltaE` | QE DOS／PROJWFC；保持 eV，不转成 postw90 或绘图参数 |

物种每行是 `[元素字符串, 有限正数质量, 非空赝势文件字符串]`；顺序保留。赝势路径仍遵循既有绑定和 QE 字符串引用规则，不新增文件发现机制。

`nk/nq/nkf/nqf` 只在对应原生调用支持完整方向字段时使用，各为三个正整数，分别展开为 `nk1/2/3`、`nq1/2/3`、`nkf1/2/3`、`nqf1/2/3`；不将简写直接写入 Fortran namelist。新示例统一写数组；数组与对应方向字段混写报错，不静默选取一方。

网格只能留在所属阶段／用途子段，`baseparams` 中的上述简写、方向字段以及原有 `K_POINTS/dos_kmesh` 均拒绝；不把 NSCF `K_POINTS` 或路径改名为 `nk`。不同调用继续使用自己的网格。公共表仅接受本次七项字段，不推广为任意参数继承；跨阶段科学约束保留。

按用户确认，阶段或用途子段的同名显式值优先，仅在未填写该字段时使用公共值；两处都未填时沿用既有必填或默认规则。物种数组整体覆盖，不逐行合并；阶段显式非法值直接报错，不回退到公共值。公共值与阶段值同时出现本身不算冲突，但展开后的有效值仍须满足科学一致性及自动绑定约束。

内部生成、指纹和恢复必须共用展开后的有效配置，诊断能指向公共或阶段来源。仅改变已被阶段值覆盖的公共字段，不应改变该阶段有效科学参数或触发其重算。原文配置快照继续保存用户实际原文，不用展开配置替换。
