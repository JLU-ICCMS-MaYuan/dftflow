# 当前目录 PBE 超导运行计划

本计划承接 [0013 T049](../0013-epwflow-superconductivity-tc/tasks.md) 和本项 T034；此前完整 Tc 任务 T048/T033 已撤销。目标仅为 `examples/MgB2/sugon-amd/run/epwflow/epwinput.toml` 绑定的 PBE 计算；不再要求 LDA 输入、LDA 质量门或其他 PH 来源组合。历史 LDA 与 `test_auto/MgB2/local_run/010-public-pbe` 结果不作为当前 PBE 的数据或验收证据。

## 已确认的用户范围

用户原话：

> 所以我现在能使用/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow里面的输入文件 epwinput.toml，并且按照 REAMD.md里面的内容，一条条执行命令，就最终算出来 MgB2的各向异性和各向同性超导转变温度了吗？使用 issue 和 spec 来帮我完成。

> 完全放弃 LDA 的要求，并且删除那些要求。只完成当前目录的 PBE 的要求。

下述完整 Tc 要求是此前范围；用户最新指令已将本次目标改为固定温度点执行，详见末节。原共享预算继续有效。

## 当前现场证据

当前真实清单为 `examples/MgB2/sugon-amd/run/epwflow/8.epw-superconductivity/manifest.json`，`run_id=9a88971103391af8ef962e6f784992f569d0b2cba48d5c27d7f27fc8332cd752`，模式为 `both`，`converged=false`、`located=false`、`tc=null`。

| 求解 | 当前证据 | 结论 |
| --- | --- | --- |
| iso 线性搜索 | 10 个有效点建立 75.89～76.16 K 区间，宽 0.27 K | 只完成线性化搜索，不是完整 iso Tc 验收 |
| iso 非线性能隙 | 10 K、1 meV 初值、1000 次后 `Convergence was not reached`；第 209 次起日志出现 NaN | 求解失败，不得当作正常态或有效能隙 |
| aniso 80 K、1 meV | 首次尝试因旧解析器不接受省略 `E` 的指数失败；后续尝试被记录为 valid | 仅此初值／温度有效，不构成双初值或搜索结果 |
| aniso 80 K、10 meV | 1000 次后未收敛；第 756 次起日志出现 NaN | 双初值一致性和 aniso 分类均失败 |

失败日志分别位于 `8.epw-superconductivity/iso/attempt-0012/epw.out` 与 `8.epw-superconductivity/aniso/attempt-0003/epw.out`。不要原地重跑或覆盖现场；相同输入预计重现相同失败。

当前绑定参数可从 `epwinput.toml` 和清单复核：PBE USPP；SCF／NSCF 4×4×2；PH 2×2×1 nosplit；5 个 Wannier 函数；插值 k/q 网格 8×8×4／4×4×2；μ*=0.05、截断 0.5 eV；收敛残差 10⁻⁸、最多 1000 次、混合系数 0.7、历史长度 8、初值 1／10 meV；Tc 搜索范围 10～80 K、目标区间宽度 0.5 K。这里仅记录现场，不表示这些参数已经证明足以收敛。

## 当前 PBE 的只读源码诊断

核对安装目录 QE 7.4.1／EPW 5.9 配套源码及当前三个 `attempt.json`，没有修改外部程序。配套源码本身不能替代二进制的构建来源证明。

- `epwflow/src/superconductivity/attempt.rs::render` 将 1／10 meV 正确转换为 `gap_edge=0.001/0.01` eV；`a_gap0` 是初始频率形状参数，不能把它当作初值单位错误。iso 首初值失败后停止，尚未执行第二初值是当前引擎的失败传播结果。
- iso 第 208 次最低频率能隙为 −1.076117×10⁻¹⁵² meV，第 209 次起 NaN；全程最小日志相对变化约 0.005929136，未达到 10⁻⁸。原生 FSR 方程存在全零能隙解，Broyden 的趋零轨迹不能证明获得正常态。
- `EPW/src/utilities/utilities.f90::mix_broyden` 先将 `DNRM2` 结果平方再开根号，然后取倒数；极小差分可下溢为零。IEEE 双精度只读数值演示 `sqrt((1e-165)^2)=0` 可以复现这一风险。iso 轨迹与此相符，但没有逐态内部历史，尚不能断言它是此次或 aniso NaN 的唯一根因。
- aniso 80 K／1 meV 在第 589 次达到原生变化量 6.702311×10⁻⁹，日志最低频率标量为 −5.329286 meV；整体符号允许翻转。80 K／10 meV 在第 433 次达到最小变化量 2.110648×10⁻⁷，仍大于门限，之后再次变化，第 756 次 NaN。该模式的日志标量由汇总得到，不能代替完整逐态分布或双初值检查。
- 安装配套源码 `mix_wrap` 明确提供负 `broyden_beta` 的原生线性混合分支；当前 Rust 配置与 Request 验证仅接受正系数。启用该算法需要有依据的数值设置变更及生产契约适配，不能暗改符号后仍称使用原固定配置。

因此没有证据支持相同参数的重复提交或仅提高迭代次数；也不能先把 NaN 改判为正常态。80 K 的单初值有限能隙还提示原搜索上界可能不足，须待双初值有效后再判断。

## 最新执行计划：固定温度点

用户已明确不需要先做有界诊断或验证最合理 Tc，原混合修改与扩大搜索上界提案撤销，不再待用户批准。只在 20、30、40 K 分别执行 iso/aniso 共六次 epw.x。

1. 复用本目录已完成的 PBE 细网格数据，独立复制重读文件，保持原数据及失败现场不变。
2. 从已有原生输入派生单温度输入，保持 μ*、截断、网格、1000 次上限、10⁻⁸、Broyden 0.7／8 和初始形状，统一选既有 10 meV 初值；不执行 Tc 线性化搜索或双初值。
3. 在一个已登记共享预算的 Slurm 分配中顺序执行六次，每次独立目录、epw.in、epw.out、epw.err、exit-code.txt。某点失败也记录后继续其余独立温度点，不覆盖历史。
4. 检查每次退出码与原生完整结束计时，无 MPI 异常或中途终止；另列收敛／未收敛与 NaN 状态。执行验收与物理正确性分开，不伪造 Tc、能隙或收敛声明。
5. 结算实际 Slurm 节点秒，更新本地 Issue／Spec、结果表及示例说明；不自动改安装或推送。

当前执行与结果见 [固定温度点运行验收](temperature-points-validation.md)。此前要求完整双模式 Tc 区间、10／15／20 K 双初值和独立参考的本次任务已取消，不作为这六次执行的关闭门槛。
