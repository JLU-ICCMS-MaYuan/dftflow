# 研究与决策：两种超导求解与受限 Tc 搜索

**依据**：[确认 Spec](spec.md)、[实施计划](plan.md)、[总体规划](../../plan/epwflow-superconductivity.md)。

## 决策、理由、备选与证据

### D01

**决策**：温度用整数 centikelvin，避免浮点键和文件名舍入冲突。每个 mode/type/T/seed/attempt 独立目录，输入固定 nstemp=1 与一个 temps。所有求解只读 F3 数据，原始 EPW 文件保留；不跨温度 imag_read 热启动。

**理由**：满足本项 Spec 对有效数据、隔离、失败传播与可复查证据的要求。

**备选**：沿用已有宽松配置、存在性判断或手工中间提交；因不能证明全部 FR/SC 而拒绝。

### D02

**决策**：iso 搜索调用目标 crit_temp_iso 的 LAPACK 路径，解析温度、本征值、频率数和成功返回，记录未提供非线性 residual；常规非线性能隙另跑两初值。不得因 LAPACK 行的迭代数为 1 而杜撰残差。目标源码存在版本相关边界，真实运行探测与固定输出 fixture 必须验证输出语法和失败传播。

**理由**：满足本项 Spec 对有效数据、隔离、失败传播与可复查证据的要求。

**备选**：沿用已有宽松配置、存在性判断或手工中间提交；因不能证明全部 FR/SC 而拒绝。

### D03

**决策**：aniso 每分类点独立跑 1/10 meV 两初值，严格要求原生 converged 和 finite residual<1e-8。读取完整 imag_aniso 数组，按 freq→k→band 及 F3 的 fsthick mask 还原索引，核对记录数、频率和能量，不按行号猜物理身份。Δ/Z 输出来自原始精度；仅比较整体符号，不逐点取绝对值掩盖解差异。

**理由**：满足本项 Spec 对有效数据、隔离、失败传播与可复查证据的要求。

**备选**：沿用已有宽松配置、存在性判断或手工中间提交；因不能证明全部 FR/SC 而拒绝。

### D04

**决策**：Tc 搜索保存两模式独立 interval/attempt map，正常二分；中点分类不确定时按 1/4、3/4 有界探测，不用无效点更新端点。每次决策写入 journal 后才安排新作业，未知提交保留次数。原生失败终止受影响模式，显式 resume 才重试；反常温度分类标 inconsistent。次数包括 seed/linear/gap/retry，不等于温度数。

**理由**：满足本项 Spec 对有效数据、隔离、失败传播与可复查证据的要求。

**备选**：沿用已有宽松配置、存在性判断或手工中间提交；因不能证明全部 FR/SC 而拒绝。

### D05

**决策**：风险：原生相对残差在零解附近未必收敛；本项不能用小能隙补救成 normal。真实验收必须取得两个可靠侧及独立邻近温度，不满足则保持未完成，不改科学求解器。提前进行低成本正常侧验证以防在全部组合上浪费预算，但不改变容差。

**理由**：满足本项 Spec 对有效数据、隔离、失败传播与可复查证据的要求。

**备选**：沿用已有宽松配置、存在性判断或手工中间提交；因不能证明全部 FR/SC 而拒绝。

### D06

**决策**：结果按温度保留数值状态和分类状态；located 只有区间宽度<=0.5K且端点有效。gap_temperatures 中的必需解失败，即使 Tc located 也只记部分结果。F5 消费结构化文件，不重新推断临界温度。

**理由**：满足本项 Spec 对有效数据、隔离、失败传播与可复查证据的要求。

**备选**：沿用已有宽松配置、存在性判断或手工中间提交；因不能证明全部 FR/SC 而拒绝。

## 事实来源与验证边界

现有源码：crates/qf-input/src/pw.rs、crates/qf-runner/src/lib.rs、qeflow/src/phonon_chain.rs、crates/qf-phonon/src/merge.rs、scripts/install-current.sh、scripts/verify-entries.sh。参考 external/epwpy 固定 b54bb53d4f5de665ed08958cb6194be8efdf645d。目标 QE 7.4.1 安装源码的 EPW/src/readin.f90、wannierization.f90、supercond_driver.f90、supercond_iso.f90、supercond_aniso.f90、io/io_supercond.f90 为版本适配事实来源；实际路径只记 docs/local。

源码声明 EPW 5.9 尚不等于实际程序身份。数值、节点兼容性和二进制读取必须用真实运行验证；网页工具本轮不可用，不能声称在线文档已核实。目标源码之外的新版本需独立适配。现有 qeflow 声子谱通过不能证明 EPW 输入完整。所有尚未运行的验收维持未完成。
