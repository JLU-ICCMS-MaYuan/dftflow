# 研究与决策：QE 三模式数据准备

**依据**：[确认 Spec](spec.md)、[实施计划](plan.md)、[总体规划](../../plan/epwflow-superconductivity.md)。

## 决策、理由、备选与证据

### D01

**决策**：使用 qf-crystal 的 POSCAR 解析，QE 输入由封闭配置适配器写出，保持 17 位有效精度；必要 qf-input 修复只影响错误配置段并回归旧入口。阶段为 SCF → snapshot → (PH/init 与 NSCF)，分 q 在 init 成功后展开；qe-data 汇合依赖所有所需节点。每一 PH 和 NSCF 保存树都从不可写 SCF 快照独立复制，禁止共享可写硬链接。

**理由**：满足本项 Spec 对有效数据、隔离、失败传播与可复查证据的要求。

**备选**：沿用已有宽松配置、存在性判断或手工中间提交；因不能证明全部 FR/SC 而拒绝。

### D02

**决策**：SCF 由日志收敛、终态和 XML 交叉确认；保存完整 outdir 文件树，不只 prefix.save。NSCF 显式生成无重复 crystal 网格，核对实际 XML k 点、权重、nbnd、WFC；网格闭合在分数坐标模整数下验证。标记晶格和 reciprocal convention，禁止以 automatic 生成的不可约集合代替完整集合。

**理由**：满足本项 Spec 对有效数据、隔离、失败传播与可复查证据的要求。

**备选**：沿用已有宽松配置、存在性判断或手工中间提交；因不能证明全部 FR/SC 而拒绝。

### D03

**决策**：only_init 正常退出产生 dyn0；严格读取网格、声明数量及坐标。沿用已有 qmap 转换数学，但校验 alat/倒格矢而不沿用旧命名错误。数字目录宽度 max(2,digits(nq))，index 全局 i；coords 局部 1。复制后生成任务时显式 only_init=false。

**理由**：满足本项 Spec 对有效数据、隔离、失败传播与可复查证据的要求。

**备选**：沿用已有宽松配置、存在性判断或手工中间提交；因不能证明全部 FR/SC 而拒绝。

### D04

**决策**：目标格式适配器读取各任务实际 outdir/_ph0，枚举根 Γ 与 prefix.q_i 布局、串行 dvscf/并行 dvscf1，验证关联 phsave patterns XML 的 q、扰动数和维度。局部 patterns 必须与该任务 dvscf 配对；复制全局控制元数据仅来自完整 init，修改局部编号时保持扰动基底。遇到需要未知基底转换、赝势附加数据或二进制布局时拒绝而非猜测。

**理由**：满足本项 Spec 对有效数据、隔离、失败传播与可复查证据的要求。

**备选**：沿用已有宽松配置、存在性判断或手工中间提交；因不能证明全部 FR/SC 而拒绝。

### D05

**决策**：归集保存 prefix.dyn_q<label>、prefix.dvscf_q<label>、prefix.phsave，label 使用目标 EPW 编号规则并以真实读取检查。验收按相同原子序比较 Cartesian 力常数和有符号频率；不会要求跨模式响应势字节相同。源数据不删除。

**理由**：满足本项 Spec 对有效数据、隔离、失败传播与可复查证据的要求。

**备选**：沿用已有宽松配置、存在性判断或手工中间提交；因不能证明全部 FR/SC 而拒绝。

### D06

**决策**：manifest 在全部校验后发布；旧目录 import 需要显式来源清单和程序身份，只有 qeflow JOB DONE 不够。独立最小 EPW 读取可使用手写底层输入，真实消费全部 q；该验证不依赖 F3 编排。

**理由**：满足本项 Spec 对有效数据、隔离、失败传播与可复查证据的要求。

**备选**：沿用已有宽松配置、存在性判断或手工中间提交；因不能证明全部 FR/SC 而拒绝。

## 事实来源与验证边界

现有源码：crates/qf-input/src/pw.rs、crates/qf-runner/src/lib.rs、qeflow/src/phonon_chain.rs、crates/qf-phonon/src/merge.rs、scripts/install-current.sh、scripts/verify-entries.sh。参考 external/epwpy 固定 b54bb53d4f5de665ed08958cb6194be8efdf645d。目标 QE 7.4.1 安装源码的 EPW/src/readin.f90、wannierization.f90、supercond_driver.f90、supercond_iso.f90、supercond_aniso.f90、io/io_supercond.f90 为版本适配事实来源；实际路径只记 docs/local。

源码声明 EPW 5.9 尚不等于实际程序身份。数值、节点兼容性和二进制读取必须用真实运行验证；网页工具本轮不可用，不能声称在线文档已核实。目标源码之外的新版本需独立适配。现有 qeflow 声子谱通过不能证明 EPW 输入完整。所有尚未运行的验收维持未完成。

## D07：优先复用 qeflow，薄适配器连接严格配置

用户明确要求避免重新实现 QE 流程。源码核查发现共享 qf-input 已有 PW/PH 写出、qf-crystal 已有结构解析；epwflow 的独立生成器却把三阶段 prefix 分别写成 scf/nscf/ph、原子质量写成 1，并把 PH 网格放在 namelist 外。决定移除这套重复实现，增加必要的共享纯渲染接口供两入口调用。

不能直接复用旧 generate_split_inputs（含逐 q SCF）或旧 build_nscf_pw_params（nosym 在 CONTROL）；复用对象以 0006 当前单 SCF 链路及修正后的共享生成器为准。真实 PH 产物的 EPW 配对验证由适配层补充，不放宽现有数据契约。

## D08：修复主路径与目标格式，而非保留伪布局

当前 SCF／NSCF 硬编码保存树、PH 人造路径及 omega 格式不符合真实 QE 7.4.1。采用已确认的完整 outdir 快照、实际 prefix 与原生 PH XML；文件复制核对哈希，缺电荷、波函数、q 映射或响应维度时明确拒绝。旧合成测试样本更新为实际格式，不在生产读取器内增加专门服务测试的兼容分支。来源与标准保持不变。

## 赝势绝对路径的 QE 卡片语法

真实 PBE 完整 NSCF 在 readpp 报告 file / not found，原输入 ATOMIC_SPECIES 的绝对路径未加引号。external/q-e/Modules/read_cards.f90 使用 Fortran 列表定向 READ，斜杠是输入终止符。适配层须用共享 QE 字符串序列化引用显式路径，保留同一文件身份；SCF／完整及参考 NSCF 共用修复，原物理参数不变。协议替身须拒绝未引用绝对路径，而不是忽略赝势卡片。

## 原生赝势文件名的 80 字节上限

加引号后 1296305 的错误路径延长到目录 pp 后截断，证实引用之外还有长度限制。external/q-e/Modules/input_parameters.f90 的 atom_pfile 为 CHARACTER(len=80)，pseudo_dir 为 len=256；READ 的局部 256 字符变量最终赋值到 80 字符卡片字段。采用所有已绑定赝势的共同父目录与短相对名，严格保留每个源路径，不进行文件猜选／复制改名；超长或不合法表示在适配前拒绝。共同目录支持不同子目录的显式文件；原生 IO fixture 核验长度和真实解析路径。
