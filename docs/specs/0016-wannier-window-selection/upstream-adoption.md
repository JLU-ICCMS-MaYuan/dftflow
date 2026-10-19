# 指定子仓库的选窗行为与接入约束

## 来源与当前完成范围

用户指定的仓库已作为 Git 子模块引入：

- 路径：`external/aiida-wannier90-workflows`。
- 远端：`https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows.git`。
- 固定提交：`1c0115cb7c77541a0bd549bc05005df435d651b6`。
- 复现命令：`git submodule update --init -- "external/aiida-wannier90-workflows"`。
- 子仓库内容与许可保持原样；不批量更新其他子模块，不增加 AiiDA 生产运行依赖。

本次完成子模块引入、指定版本源码核查及两套 flow 的差距审计，并据此接入共享默认策略。以下解释上游规则与适配边界；实际实现验证见 [验收记录](acceptance.md)，不把源码接入称为新窗口科学质量通过。用户已明确采用上游默认组合模式并要求完成实施、弃用旧方式；接入状态以验收记录为准。

## 一手源码入口

所有链接固定到上述提交，避免将主分支后续变化当作本次行为。

| 内容 | 已核查的源码 |
| --- | --- |
| 模式枚举 | [common/types.py](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/common/types.py) |
| 默认选择、覆盖顺序、运行时平移与上界限制 | [workflows/base/wannier90.py](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/workflows/base/wannier90.py#L236) |
| 投影求和、阈值选能量、SCDM 拟合 | [utils/scdm.py](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/utils/scdm.py) |
| HOMO／LUMO、排除带 | [utils/bands/__init__.py](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/utils/bands/__init__.py#L8) |
| 协议与阈值默认值 | [workflows/protocols/base/wannier90.yaml](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/workflows/protocols/base/wannier90.yaml) |

## “完全采用”的具体含义

以指定提交的选窗行为为基线，两套 flow 应共享同一套规则及证据结构，不能继续分别维护经验算法后只冠以 AiiDA 名称。生产实现遵守项目既有 Rust／Bash 约束；子模块作为可复核来源，不直接启动 AiiDA 数据库或守护进程。

上游没有统一自动算出四个数值边界。它主要决定冻结方式、冻结上界、能量平移及态数上限；未设置的外窗／下界由原生 Wannier90 默认语义处理。接入时保留“未显式设置”，运行后从实际日志记录有效四界。不能把现有 `wannier_window --auto` 的带号加余量方案混入上游算法。

### 五种冻结模式

以下以 `WannierDisentanglementType.SMV` 为前提。

| 模式 | 构建阶段行为 | 运行阶段行为 |
| --- | --- | --- |
| `ENERGY_FIXED` | 设置相对 `dis_froz_max=2.0 eV`，启用平移 | 按下节平移，再限制冻结上界 |
| `ENERGY_AUTO` | 启用 `auto_energy_windows` | 根据真实投影质量阈值求绝对冻结上界，再限制上界；不同时启用平移 |
| `PROJECTABILITY` | `dis_proj_min=0.01`、`dis_proj_max=0.95` | 由投影解纠缠选择；不自动增加能量平移 |
| `FIXED_PLUS_PROJECTABILITY` | 同时使用 `dis_froz_max=2.0` 与 `dis_proj_min/max=0.01/0.95`，启用平移 | 组合能量冻结与投影解纠缠；这是构建器默认模式 |
| `NONE` | 删除四个能量边界 | 不添加能量冻结窗 |

解纠缠类型为 `NONE` 是另一维设置：上游令 `dis_num_iter=0` 并删除四窗，不等同于上述 SMV 下的冻结 `NONE`。

用户参数覆盖发生在协议默认值生成之后、运行时平移／限制之前。所谓手工覆盖不意味着绕过运行时态数上限。不得先把绝对值写入参数，又重复叠加 E_F。

### 能量平移

记所有输入 Bloch 本征值为 `E[k,b]`，共同 Fermi 能为 `E_F`。上游先在原始能带集合上求：

```text
HOMO = max(E[k,b] <= E_F)
LUMO = min(E[k,b] >  E_F)
shift = LUMO，若 LUMO - HOMO > 0.001 eV；否则 shift = E_F
```

对已经存在的四个能量边界分别加 `shift`；不存在的键保持不存在。没有占据态或未占据态时，上游辅助函数报错，不能补造带边。

这是一项按有限采样能谱判断的启发式，不等于已经严格判定绝缘体。金属的离散 k 网格也可能产生大于 1 meV 的采样间隙。采用时应在记录中保留 HOMO、LUMO、平移值和分支原因，不能暗中改成另一套判据后声称与上游一致。

### `ENERGY_AUTO` 的投影阈值

`get_projectability_arrays` 将全部选用原子轨道的投影权重相加，得到与本征值同形状的 `p[k,b]`。输入是投影模平方，不能再次平方；DOS、绘图 PDOS 积分、Wannier 中心或模式编号都不能代替逐态权重。

```text
requested_frozen_max = max { E[k,b] | p[k,b] >= 0.9 }
```

源码先展平并按能量排序，选最后一个满足阈值的态。因此它不是“第一次跌破 0.9 的能量”，也不是误差函数拟合结果；高能孤立的高投影态仍会参与。没有满足阈值的态应给出明确无候选结果。

SCDM 是另一条路径：拟合 `0.5*erfc((E-mu)/sigma)`，默认取 `scdm_mu=mu_fit-3*sigma`。不能把这项拟合混进 `ENERGY_AUTO`。

### 冻结上界限制

运行时存在 `dis_froz_max` 且有本征值时，上游按此顺序执行：

1. 若指定共线自旋通道，选择对应通道；不混合两通道。
2. 将配置中从 1 开始的 `exclude_bands` 转为从 0 开始的索引，去重后删除对应带列。
3. 记剩余带数为 `B`、Wannier 数为 `N`，初始上限为 `max_k E[k,min(N,B)]`，此处带号从 1 开始。
4. 若 `B>N`，再取与 `min_k E[k,N+1]-0.0001 eV` 的较小值。
5. 最终冻结上界是请求值与该上限的较小值。

即 `B>N` 时：

```text
cap = min(max_k E[k,N], min_k E[k,N+1] - 0.0001 eV)
dis_froz_max = min(requested_frozen_max, cap)
```

不能省略第 N 条带的最大值项，也不能因为用户设置了较高冻结下界而自行放宽此上限。HOMO／LUMO 平移和投影阈值计算在上述排除带限制之前执行；若改变顺序，应明确是偏离上游的改动。

DFTFlow 仍需要在此之后检查空数据、有限数、剩余带数≥N、逐 k 点外窗态数≥N、冻结态数≤N、窗口包含关系和目标覆盖。这些是项目的拒绝条件，不应重新计算一套不同的“上游窗口”。

## 两个入口的数据接入要求

| 数据／职责 | epwflow | wannierflow |
| --- | --- | --- |
| 本征值、k 点与单位 | 复用绑定到 F2 的 NSCF XML 解析与来源身份；`EpwConfig` 已增加 F2 本征值绑定，缺数据时拒绝生成 | 初始化使用共享严格 QE XML 解析，检查命令的旧数据分析也经过形状与有限数检查 |
| 投影 | 默认组合模式由原生 W90 使用实际投影矩阵；ENERGY_AUTO 才另需逐态投影估能适配 | 原生 W90 使用匹配的实际投影；不能把任意路径能带投影当作均匀网格投影 |
| 能量参考 | 现有 `qe_zero`／`fermi` 转换只能执行一次；新增平移必须有唯一责任方 | 明确输入是绝对能量还是上游相对边界，不从绘图默认 Fermi 值反推 |
| 输出 | 先保存请求／平移／限制后的参数，再生成独立 W90 或 EPW 内嵌 W90 输入 | 同一结果用于检查命令及 `.win` 生成，避免只打印建议但运行仍使用旧值 |
| 能力检查 | 检查实际 EPW 内嵌 W90；外部 W90 版本不能代表内嵌版本 | 检查实际配置的独立 W90 和 pw2wannier90，包含所需投影通路 |

当前 MgB₂ 原生日志为 W90 3.1.0；进一步只读检查本机实际 W90／EPW 二进制和配套 W90 参数源码，都没有发现三项投影参数。此次能力结论基于实际文件，不是仅凭版本号。

指定仓库构建器显式设置 `dis_proj_min/max`，没有显式写入 `dis_froz_proj`。进一步核对 [标准 W90 的参数读取](https://github.com/wannier-developers/wannier90/blob/develop/src/readwrite.F90)，其投影冻结开关默认关闭，显式阈值会关闭自动阈值估计。因此 DFTFlow 的标准原生接口额外写入 `dis_froz_proj=true`，用于落实同一组合方法；不改变上游 0.01／0.95 数值规则，也不静默使用 W90 的其他自动阈值。该接口适配单独说明，不能宣称是上游构建器原文。

执行前以实际程序文件检查这三项关键字。检查是保守必要条件：没有关键字时拒绝，不把存在关键字当作完整原生成功；采用自定义动态库但无法这样核验时同样保守拒绝，不凭版本猜测能力。

保留现有五维 MgB₂ 基底不等于已经具备上游默认的 PAO 投影条件。上游完整协议还会从赝势轨道推导 `num_wann`、排除半芯态并设置自动投影，这可能与两 B:pz 加三键中心 s 的基底不同；当前迁移不能无声明地改变物理基底。

## 验证要求

生产接入前应与固定提交进行差分验证，覆盖：五种模式、手工覆盖顺序、E_F／LUMO 两个平移分支、非单调投影、无合格投影、排除带映射、上下界重合、N 与带数的边界情况，以及实际程序能力不足时明确拒绝。能量差分容差与实际输出精度应在正式接口契约中确定。

仅算法相同不能证明两套入口接通：还需验证最终原生输入、实际 Outer／Inner 日志与选择记录相符，以及前序失败不会继续启动后序程序。真实质量验证继续使用既定独立参考和电子双误差门；不改变声子门及累计预算。

## 许可与引用

[原始 LICENSE.txt](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/LICENSE.txt) 标题为学术非商业最终用户许可，包含分发及代码使用限制，不是 MIT／BSD 许可。原许可及引用要求保留；本次仅登记原始 Git 子模块和研究结论，不将其源码换许可证复制到生产目录，也不发布或推送派生实现。后续移植方式须保留明确来源及适用许可依据。

## 最新用户决定与实施边界

用户已明确选择上游默认 `FIXED_PLUS_PROJECTABILITY`，并要求完成接入、弃用旧方式。原来的分阶段候选扫描建议被这个明确方向替代；不再等待模式选择，也不以旧带号算法作为兼容回退。

共享模块复现该模式的默认值与运行时规则；`ENERGY_AUTO`、SCDM 和阈值扫描是已研究的其他上游能力，不额外实现为本次默认。显式能量覆盖仍经上游态数上限处理；epwflow 的历史绝对／相对 E_F 配置通过适配转换，不意味着保留旧选择算法。

项目源码接入与实际科学程序能力分别验收。没有授权升级／覆盖本机 QE 或执行新科学作业；现有程序不支持时必须拒绝执行并保留原因，不自动退回旧方法。
