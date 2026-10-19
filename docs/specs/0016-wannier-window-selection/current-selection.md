# 当前窗口选择与只读核验

本文保留首次调研时的迁移前代码／实际计算基线；现有原生输入和日志未改写。后续用户指定的共享默认组合选窗见 [采用说明](upstream-adoption.md) 与 [验收](acceptance.md)，下述旧自动算法不再作为当前生产选择。

核验日期：2026-10-19；时区：Asia/Shanghai。源码基线为 `b3922f2`，核验时工作区另有未提交的其他任务修改；这里只描述实际读取到的窗口相关代码及现存计算，不把工作区等同于该提交的全部内容。

## 实际选择方式

MgB₂ LDA 算例沿用用户此前指定的 Mori 2024 EPW 学校讲义，第 19 页输入设置 `dis_froz_max=8.8` eV，未给出另外三个窗口边界。当前 `019-lda-q6-phonon-wannier/coarse/epw.in` 同样只显式设置该上界。这里的 `frozen_win` 对应 Wannier90 的 `dis_froz_min/max`，不是额外一种窗口。

实际 `mgb2.wout` 报告 Wannier90 **3.1.0**、5 个 Wannier 函数、16 条输入 Bloch 带，以及如下 Outer／Inner：

| 参数 | 绝对能量／eV | 相对共同 E_F／eV | 来源 |
| --- | ---: | ---: | --- |
| `dis_win_min` | −4.90710 | −12.404369413 | 未显式指定；实际 Outer 下界 |
| `dis_win_max` | 32.23753 | 24.740260587 | 未显式指定；实际 Outer 上界 |
| `dis_froz_min` | −4.90710 | −12.404369413 | 已给冻结上界时，下界默认沿用外窗下界 |
| `dis_froz_max` | 8.80000 | 1.302730587 | Mori 示例与实际输入显式值 |

共同 E_F 为 **7.497269413025949 eV**，来自比较来源清单绑定的 F2 XML。表中外窗端点来自日志打印值，精度为五位小数，不能冒充本征值全精度。该选择尚未经过四边界优化。

## 当前代码能力

- `epwflow/src/foundation/config.rs`：支持 `outer_window_ev`、`frozen_window_ev` 或单独的 `frozen_max_ev`；检查区间递增、互斥和已有外窗的包含关系。尚无根据实际全部本征值进行逐 k 点态数预检的选窗器。
- `epwflow/src/interpolation/epw_config.rs`：`windows()` 按 `qe_zero` 或绑定的 `f2_fermi` 转换能量参考并渲染配置；负责转换，不计算最优边界。
- `epwflow/src/interpolation/comparison.rs`：从带哈希的 `.wout` 读取实际 Outer／Inner，用于比较数据和绘图；显示窗口不能替代选择算法。
- `wannier90flow/src/window.rs`：已有独立 `--auto nbnd1 num_wann nbnd3`。下界取指定起始带最低能量减 `delta`，内外窗下界相同；递减候选带号，直到冻结窗最大态数不超过 `num_wann`；再通过额外带数估计外窗上界。默认 `delta=0.02 eV`。这是一种能量／带号启发式，尚未接入 epwflow，亦未结合目标能区、投影质量、独立拟合误差选择窗口。

## 现存 NSCF 的逐点统计

只读解析 `014-lda-fsr-scf-nscf-phonon/electrons/tmp/mgb2.save/data-file-schema.xml`：216 个 k 点，每点 16 条带；当前算例没有排除带。Hartree 到 eV 使用 27.211386245988，统计端点比较容差为 10⁻⁸ eV。

| 窗口 | 每个 k 点态数的最小值 | 最大值 | 含义 |
| --- | ---: | ---: | --- |
| 当前实际外窗 | 16 | 16 | 使用全部已计算候选态 |
| 当前实际冻结窗 | 3 | 5 | 当前粗网格满足不超过五个冻结态 |
| 共同 E_F±0.5 eV | 0 | 2 | 目标窄窗不能直接充当五维子空间的外窗 |

另外统计：`max_k ε₅k=14.242639140 eV`，`min_k ε₆k=10.866040339 eV`。若外窗下界保留在所有输入态下方，则前者是外窗上界容纳至少五态的最低态数界；若冻结下界也保持在所有输入态下方，则冻结上界须小于后者，并留出数值余量，以免某点冻结第六态。这两条结论仅适用于当前无排除带的粗网格及固定下界，不能推广为任意四边界公式。

**两者均不是推荐的最优边界。** 最小态数可行外窗可能失去有用混合态及子空间平滑性；最大态数允许的冻结上界也可能冻结不合适的态。独立网格外点、更多 k 点及不同下界可能进一步收紧限制。

当前目录有 `.mmn`、`.chk`、`.win`，未发现 `.amn` 文件；不能根据本征值或 Wannier 中心推造规范化投影质量。已有矩阵是否足够恢复所需投影是后续适配问题，本轮没有实现或声称恢复。

## 来源身份

以下均相对本地忽略数据根 `test_auto/MgB2/local_run/001-epw-validation/`，不把机器路径或原始计算数据纳入版本控制。

| 文件 | SHA-256 |
| --- | --- |
| `014-lda-fsr-scf-nscf-phonon/electrons/tmp/mgb2.save/data-file-schema.xml` | `f33ee9fba3a42ccf2132f2531aed23cb81392a7f85444b6065eef2d01b6c58ce` |
| `019-lda-q6-phonon-wannier/coarse/mgb2.wout` | `d8d6e7703ac53ecaaa327b41afdb51bdd57e17f3246870cd5a983b65d162bb53` |
| `019-lda-q6-phonon-wannier/coarse/epw.in` | `c620ea09cd3b8e6b6c71786ef55cb442eebcd760981e7d4a254bcc3821014aa5` |
| `091-reference-literature/Wed.9.Mori.pdf` | `ee857a8a59f216889c54d2636a6cf7954f269d339d913e42a7f331ea60c4bab9` |

## 复核方法与限制

1. 读取输入中的 `nbndsub`、`proj`、`dis_*`；核对日志 Release、Wannier／Bloch 数量及实际 Outer／Inner。
2. 从上述 XML 的每个 `ks_energies/eigenvalues` 读取本征值，换算 eV；按区间闭端点逐点计数，再取计数最小／最大值。没有将 k 点权重或自旋简并乘进单点态数。
3. 各点按能量排序，分别取第五带最大与第六带最小；今后有排除带或自旋通道时，须先按实际传给 Wannier 的态集合处理，不能直接复用这些带号。
4. 重新计算上述文件 SHA-256，与比较清单的 F2 XML／Wannier 日志身份核对。

本轮没有运行科学程序、提交作业或修改来源文件。冻结态约束作用在输入采样点上的子空间，不能因此保证任意网格外插值精确。此前少量电子参考点的通过也不证明整个外窗覆盖。
