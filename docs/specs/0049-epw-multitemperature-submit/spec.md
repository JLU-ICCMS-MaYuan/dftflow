# 功能规格：超导双方法多温度提交

> **协作入口（2026-11-17）**：[GitHub Issue #49](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/49)。Issue 已在 GitHub 登记，协作状态以 GitHub 为准；本文件保留设计与历史验收，旧的“本地草案／线上自管／未发布”说明是迁移前语境。Spec 远程链接待发布及验证。

> 2026-11-17 最新约定：预算账本功能已按用户要求彻底撤销。本文件涉及账本、campaign 身份、预算预留／结算及全局两个作业限制的旧条款均已失效；当前计算只依赖工作流记录、Slurm 作业身份及科学核验。历史消耗与验收记录不改写。详见 [0058 最新需求](../0058-epw-remove-resource-limits/spec.md)。

**Issue**：[GitHub Issue #49](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/49)，远程链接待验证。**日期**：2026-10-22。
**状态**：本地实现与验证完成，线上登记仍由用户管理；详细技术产物为本轮落档，不冒充此前单独展示确认的 Spec。

## 原始需求记录

> epw_superconductivity -c "epwinput.toml" --stage superconductivity -s 我发现每次执行都只跑一个方法的一个温度点，而不是把所有方法（各向异性和各向同性）的所有温度点都跑了。你可以把各向同性的结果放到一个目录里面，各向异性的放到一个目录里，但是相同方法的各个温度点都放到同一个目录里面。 你看看那个/data/home/mayuan/code/dftflow/test_auto/MgB2/epwflow/Wed.9.Mori.pdf 他们都是这么设置温度的。我希望你用 issue 和 spec 来完成。

> 而且我希望各向异性和各向同性两个方法一起提交。

> 两个独立作业一起提交 (Recommended)

> 保留现场，显式恢复 (Recommended)

> 修改此版 Issue



> 认可这个计划。开始执行

## 附件与推断来源

Mori 讲义：`test_auto/MgB2/epwflow/Wed.9.Mori.pdf`。本地只读核对第 4 页显式列表、第 19 页 MgB₂ 的 nstemp=10／temps=10 55，第 5／21 页说明不收敛会停止后续温度。材料不是新增物理参数授权。`images: []`，无上传图片。

## 背景与目标

现有点计划完整，但逐次 -s 仅提交单点。用户需要一次命令开始全部方法和温度，目录直接表达方法与计算参数。

### 目录命名设计理念（2026-11-18 更新）

超导计算子目录命名必须完整反映影响超导结果的所有关键参数，支持参数扫描和结果对比：

**命名格式：** `{mode}-mu{mu_star}-Tc{T_min}-{T_max}-degaussq{degaussq}meV`

**设计原则：**
1. **完整性**：包含所有影响超导 Tc 的关键参数（求解模式、库仑赝势、温度范围、声子展宽）
2. **可读性**：添加 `Tc` 前缀明确温度含义，单位显式标注（meV）
3. **可扩展性**：参数不同时自动创建独立目录，避免结果覆盖
4. **可追溯性**：目录名即参数记录，无需查阅输入文件

**参数说明：**
- `mode`：`iso`（各向同性）或 `aniso`（各向异性）
- `mu_star`：库仑赝势参数（无量纲），缺省 0.13
- `Tc`：温度范围前缀，明确这是超导转变温度范围
- `T_min`、`T_max`：配置中的首末温度（单位 K）
- `degaussq`：声子展宽参数（单位 meV），**优先使用第 8 步 `iso`/`aniso` 配置中的值；如未设置，则从第 7 步通过 `spectrum_index` 选择**

**参数独立性与复用机制：**
- 第 7 步生成的 `.ephmat` 文件（电声耦合矩阵元）不依赖于 `degaussq`
- 第 8 步可以复用这些文件，通过不同的 `degaussq` 测试对超导 Tc 的影响
- **支持在第 8 步独立设置 `degaussq`，无需重新运行细网格插值**
- 如果第 8 步未设置 `degaussq`，回退到 `spectrum_index` 机制（从第 7 步的 `phonon_smearings_mev` 数组中选择）

**目录命名一致性保证：**
- 目录名显示的 `degaussq` 与超导计算实际使用的展宽值严格一致
- 不同 `degaussq` 值自动创建独立目录，避免结果覆盖

**示例：**
- `aniso-mu0.1-Tc20-40-degaussq0.5meV` — 各向异性，μ*=0.1，20-40K，0.5 meV 展宽（从第 7 步选择）
- `iso-mu0.13-Tc10-50-degaussq2.0meV` — 各向同性，μ*=0.13（默认），10-50K，2.0 meV 展宽（第 8 步独立设置）
- `aniso-mu0.1-Tc20-40-degaussq1.0meV` — 同一 μ* 和温度，测试不同 degaussq（第 8 步独立设置）

**参数扫描示例（无需重新运行细网格插值）：**
```bash
# 第一次：使用 degaussq = 0.5 meV
vim input.toml  # [stages."8.epw-superconductivity".iso] 不设置 degaussq（使用 spectrum_index=0）
epw_superconductivity -c input.toml --stage superconductivity -s
# 结果目录：aniso-mu0.1-Tc20-40-degaussq0.5meV

# 第二次：测试 degaussq = 2.0 meV 的影响
vim input.toml  # [stages."8.epw-superconductivity".iso] 添加 degaussq = 2.0
epw_superconductivity -c input.toml --stage superconductivity -s
# 结果目录：aniso-mu0.1-Tc20-40-degaussq2.0meV（复用第 7 步的 .ephmat 文件）
```

数值格式为普通十进制并去掉冗余零。原生上限 50 点来自当前 EPW readin.f90；不新增温度范围语法。

### 需求结论与变更取舍

采用上述目录命名规则，显式配置保留。仅指定温度模式变更原生批次，Tc 搜索维持既有契约。

## 用户场景与验收

### US1：一次提交所有方法与温度（P1）

用户配置 both、20／30／40 K，执行公开命令；独立验收为两个独立作业各执行一次原生 EPW，输入均包含三个温度。优先级依据：消除逐点手动推进。

### US2：按参数查看结果（P1）

用户从目录查看方法、μ*、完整温度列表首末项与有效 degaussq；显式值、回退值和不同方法参数独立，切换展宽保留旧结果。

### US3：保留证据并恢复（P1）

一方法中途失败时另一方法仍独立运行；显式恢复只提交该方法剩余点。独立验收为成功点不重算、输入日志现场保留、未知回执不重提。优先级依据：已有明确恢复取舍。

### 边界与异常场景

单方法／单温度有效；非等距列表与顺序原样传递；非法、重复、超过 50 点在提交前拒绝。部分提交／非零退出／未收敛／缺文件／错温度均分别记录。每方法使用配置资源，累计预算不重置。历史单点记录继续只读核验。

## 需求

- **FR-001**：gap 模式按所选方法一次提交全部独立作业，无 afterok 方法依赖。
- **FR-002**：每方法一份完整原生温度输入，一次 EPW 调用，不隐式搜索或补温度。
- **FR-003**：方法目录使用 `{mode}-mu{mu_star}-Tc{T_first}-{T_last}-degaussq{value}meV`；首末温度取完整配置列表，恢复剩余点不改变目录；不同展宽目录独立保存。
- **FR-004**：缺省 mu_star=0.13；显式值优先，输入与目录一致。目录名包含完整参数信息（模式、mu_star、温度范围、degaussq）。
- **FR-005**：每方法独立登记提交、预算和作业状态，部分失败不丢失其他方法。
- **FR-006**：逐温度核验实际模式、温度、频率、收敛与 Δ／Z；区分执行完成与数据有效。
- **FR-007**：提前停止保留现场，显式恢复只补未完成点；成功、活动、未知作业不重提。
- **FR-008**：旧单点清单可读，Tc 搜索、三个公开程序及其他阶段保持既有行为。
- **FR-009**：更新手册、Overview、公开进程回归与验收证据，不冒充部署或科学验证。

### 关键实体

固定点计划、方法批次、原生作业、逐点证据与结果包。点仍以方法＋温度标识；一个批次关联多个点。

## 成功标准

- **SC-001**：US1 的两个脚本实际执行，各只调用一次 EPW，温度集合与配置相同；覆盖 FR-001／002／005。
- **SC-002**：默认值、显式值、小数温度、单点及非等距列表目录与输入一致，目录名格式为 `{mode}-mu{mu_star}-Tc{T_min}-{T_max}-degaussq{degaussq}meV`；覆盖 FR-003／004。
- **SC-003**：错温度、缺点及非零退出不得冒充执行完成；未收敛、NaN 或缺 Δ／Z 不得冒充数据有效，执行、收敛与数据状态分别可验；覆盖 FR-006。
- **SC-004**：部分提交失败、未知回执与中途停止可重验，显式恢复不重算成功点；覆盖 FR-005／007。
- **SC-005**：旧 Tc、旧单点记录、结果归集／移动重绘及其他阶段回归通过；覆盖 FR-008／009。

## 假设与依赖

依赖有效细网格交付和现有 EPW 5.9、Slurm、预算机制。目录命名优先使用第 8 步的 `degaussq`（如设置），否则回退到 `cfg.interpolation.phonon_smearings_mev[spectrum_index]`。目录数值格式为普通十进制并去掉冗余零。缺少前置时沿用原阶段推进，不在登录节点运行科学程序。

第 8 步可以独立设置 `degaussq` 参数，复用第 7 步的 `.ephmat` 文件，无需重新运行细网格插值。这符合 EPW 的物理机制（电声耦合矩阵元不依赖于声子展宽参数）。

## 待确认决策

无阻碍已授权实现的高影响产品问题。线上 Issue／远程链接仍待用户管理；详细技术产物不冒充单独 Spec 确认。

## 范围外事项

不承诺合理 Tc、科学收敛或新增真实计算；不更新默认安装、不推送、不改已有显式 mu 配置。

## 澄清记录

已选择两个独立作业、保留现场显式恢复。目录命名采用完整参数格式（包含 mode、mu_star、温度范围、degaussq），替代早期的简化格式。实现只将相同方法的温度合并为原生批次。

## 最终草案确认

用户确认已展示的计划及目录修订并要求开始执行；据此实施。详细 Spec 本轮落档，未单独展示确认，不将该检查项冒充完成。


## 2026-10-22 联合真实验收补充

用户要求重新编译并在指定 test_auto／local_run 完成 0046～0050 全链，原文与授权边界见 [0050 补充需求](../0050-epw-stage-results/spec.md#真实全链验证补充授权)。本次是既有 Feature 的修复与验收，不另建同范围 Issue；线上 Gitee 仍由用户管理。

一次公开提交得到 1334469／1334470 两个独立作业；各执行一次 EPW 的 20／30／40 K 完整列表，六点均执行完成、收敛且数据有效。显式 μ*=0.05 保留，实际目录为旧格式 `iso/aniso-mu0.05-20-40`（2026-11-18 后更新为包含 degaussq 的新格式）；较新的 0050 规定作业内收尾，无需再次公开命令归集。

实际作业、制品、失败修复、完整回归、安装及示例清理状态统一见 [联合验收记录](../0050-epw-stage-results/real-chain-validation.md)。当前真实链已完成；部署及文档收尾以该记录的最终状态为准，不把旧替身证据改写成科学验证。

## 2026-10-27 原生脚本约束更新

用户再次明确第 5～8 步脚本不能调用 `epw_wannier`、`epw_superconductivity`，必须直接执行原生科学程序，所有新 `.sh` 必须有执行权限。该最新要求恢复 0026 原生任务边界，覆盖本文件历史的作业内公开 CLI 收尾及自动发布取舍：第 5～8 步由后续公开调用归集、核验、绘图与发布；gap 仍按方法独立批量提交，不在作业内回调公开 CLI。历史需求原文和历史验收记录不删除，现行修复与验证以 [0026 验收](../0026-epw-direct-native-submit/acceptance.md) 为准。

## 2026-11-18 目录命名完整参数化与 degaussq 独立设置

### 用户需求原文

> 我想知道如果我想测试一个 k16x16x8-q4x4x2-fsthick0.4eV-degaussw0.1eV，k8x8x4-q4x4x2-fsthick0.4eV-degaussw0.1eV 2 个细网格插值。我是不是应该修改两次 input.toml，然后跑两次细网格插值。

> 而且因为超导计算和 degaussq 有关系，我希望 aniso-mu0.1-20-40/ 的格式为：aniso-mu0.1-Tc20-40-degaussq0.5meV

> spectrum_index 的真正含义：我错误地认为它是用来选择 degaussq 的，实际上它只是用来选择第 7 步生成的哪个 a2f 谱文件作为参考。但是我并没有在/home/mayuan/code/dftflow/examples/MgB2/ubuntu/run/7.epw-fine里面看到多个可供选择的 a2f 啊？

> 我理想的情况是：在第 8 步设置不同的 degaussq 测试其对超导的影响。

> 认可。把旧的设计删掉，把我的新设计理念写到#49里面。

### 设计理念更新

用户明确要求超导计算子目录命名必须包含所有影响 Tc 的关键参数，以支持参数扫描和结果对比。旧的简化命名格式（`aniso-mu0.1-20-40`）已废弃，替换为完整参数化格式。

**核心理念：**
1. **目录即参数记录** — 无需查阅输入文件即可知晓计算参数
2. **避免结果覆盖** — 不同 degaussq 值自动创建独立目录
3. **可读性优先** — 添加单位和前缀标识（`Tc` 表示温度，`meV` 表示声子展宽）
4. **支持参数扫描** — 同一工作流下测试多组参数时结果自动分离

### 关键问题澄清：`spectrum_index` 与 `degaussq` 的关系

#### `spectrum_index` 的设计意图

从 EPW 工作流设计来看：
- 第 7 步（细网格插值）可以定义多个 `[stages."7.epw-fine".spectra.xxx]` 子任务
- 每个子任务用不同的 `degaussq` 计算不同的 Eliashberg 谱函数（a2f）
- `phonon_smearings_mev` 数组存储所有这些展宽值（如 `[0.5, 2.0]`）
- 第 8 步通过 `spectrum_index` 选择使用哪个已计算的 a2f 谱

**当前配置中的实际情况：**
- 用户的 input.toml 第 7 步只设置了 `degaussq = 0.5`（没有 `spectra` 子任务）
- 所以 `phonon_smearings_mev = [0.5]`（只有一个元素）
- `spectrum_index = 0` 选择的就是这唯一的 0.5 meV

#### EPW 物理机制：为什么第 8 步可以独立设置 degaussq

根据 Mori Tutorial (Wed.9.Mori.pdf) 第 4-5 页说明：

1. **第 7 步生成的 `.ephmat` 文件（电声耦合矩阵元）不依赖于 `degaussq`**
2. 这些文件只依赖于 k/q 网格，计算成本高，只需计算一次
3. **第 8 步超导计算时可以复用这些 `.ephmat` 文件**
4. 通过不同的 `degaussq` 和 `nqstep` 重新计算 a2f 谱和 Tc
5. 测试 `degaussq` 对超导的影响**不需要重新进行细网格插值**

**用户的正确理解：**
> 我记得测试 degaussq 对 Tc 的影响不需要重新进行细网格插值。我理想的情况是：在第 8 步设置不同的 degaussq 测试其对超导的影响。

这完全符合 EPW 的设计哲学和物理机制。

### 实施方案

#### 1. 允许第 8 步独立设置 `degaussq`

修改配置模式，支持在超导阶段独立设置声子展宽：

```toml
[stages."8.epw-superconductivity".iso]
degaussq = 2.0    # 新增：允许独立设置，测试不同展宽对 Tc 的影响
nqstep = 500      # 已有
# ... 其他参数
```

**配置优先级规则：**
- **优先使用**第 8 步 `iso`/`aniso` 配置中的 `degaussq`（如果设置）
- **回退使用**第 7 步通过 `spectrum_index` 选择的 `phonon_smearings_mev[spectrum_index]`

#### 2. 目录命名逻辑修正

修改 [`epwflow/src/superconductivity/batch.rs:48`](../../../epwflow/src/superconductivity/batch.rs#L48) 的 `directory_name` 函数：

```rust
pub fn directory_name(cfg: &Config, mode: Mode) -> String {
    let settings = cfg.solver_for(mode).expect("Configuration validated at public entry");
    let ts = &settings.gap_temperatures_k;

    // 优先使用第 8 步的实际 degaussq；如果未设置，则从第 7 步通过 spectrum_index 选择
    let degaussq = cfg.native_value::<f64>(&[
        crate::foundation::native_config::SUPER,
        method(mode),
        "epw",
        "INPUTEPW",
        "degaussq"
    ]).unwrap_or_else(|_| {
        cfg.interpolation.phonon_smearings_mev[settings.spectrum_index as usize]
    });

    format!(
        "{}-mu{}-Tc{}-{}-degaussq{}meV",
        method(mode),
        settings.mu_star,
        ts[0],
        ts[ts.len() - 1],
        degaussq
    )
}
```

**关键变更：**
- 添加 `Tc` 前缀明确温度含义
- 添加 `degaussq{value}meV` 声子展宽参数
- **优先从第 8 步读取 `degaussq`**，支持参数扫描而不需要重新运行细网格插值
- 回退到 `spectrum_index` 机制保持向后兼容

**为什么目录名显示的是正确值：**
- 目录命名直接读取超导计算**实际使用的 `degaussq` 配置值**
- 如果第 8 步设置了 `degaussq`，目录名显示该值
- 如果第 8 步未设置，则显示从第 7 步选择的值（通过 `spectrum_index`）
- 保证目录名与 EPW 输入文件中使用的展宽值严格一致

### 技术细节与一致性机制

#### `degaussq` 参数的数据流

**场景 1：使用第 7 步的 a2f 谱（旧方式）**
1. 第 7 步定义 `phonon_smearings_mev = [0.5, 2.0]`（多个展宽值）
2. 第 8 步设置 `spectrum_index = 0`（选择 0.5 meV 的 a2f 谱）
3. 第 8 步不设置 `degaussq`
4. 目录命名回退读取 `phonon_smearings_mev[0] = 0.5`
5. 结果：`aniso-mu0.1-Tc20-40-degaussq0.5meV`

**场景 2：第 8 步独立设置 `degaussq`（新方式，支持参数扫描）**
1. 第 7 步生成 `.ephmat` 文件（`degaussq = 0.5`，只计算一次）
2. 第 8 步设置 `degaussq = 2.0`（测试不同展宽对 Tc 的影响）
3. 超导计算复用第 7 步的 `.ephmat` 文件
4. 用新的 `degaussq = 2.0` 重新计算 a2f 谱和 Tc
5. 目录命名直接读取第 8 步的 `degaussq = 2.0`
6. 结果：`aniso-mu0.1-Tc20-40-degaussq2.0meV`

**多次测试不同 degaussq 不会覆盖：**
```
8.epw-superconductivity/
├── aniso-mu0.1-Tc20-40-degaussq0.5meV/   # 第一次运行
├── aniso-mu0.1-Tc20-40-degaussq2.0meV/   # 修改 degaussq 后第二次运行
└── iso-mu0.1-Tc20-40-degaussq0.5meV/
```

#### 配置验证与单位

- **`degaussq` 单位**：meV（毫电子伏特），定义在 [`epwflow/src/foundation/config.rs:74`](../../../epwflow/src/foundation/config.rs#L74) 的 `Interpolation.phonon_smearings_mev`
- **EPW 输入文件自动生成**：超导计算的 `epw.in` 文件会包含实际使用的 `degaussq` 值（见 [attempt.rs:309](../../../epwflow/src/superconductivity/attempt.rs#L309)）
- **一致性保证**：
  - 目录名中的 `degaussq` 与 EPW 输入文件中的值完全一致
  - 配置验证（[config.rs:184](../../../epwflow/src/foundation/config.rs#L184)）确保 `spectrum_index` 在合法范围内（回退场景）
  - 第 8 步独立设置的 `degaussq` 不受第 7 步的 `phonon_smearings_mev` 数组限制

#### 向后兼容性

- **旧配置**（未在第 8 步设置 `degaussq`）：自动回退到 `spectrum_index` 机制，行为不变
- **新配置**（第 8 步设置 `degaussq`）：支持参数扫描，无需修改第 7 步配置
- **目录名格式变更**：所有超导计算目录统一使用新格式，旧目录不自动迁移

### 未来扩展

当前改进解决单一 degaussq 参数扫描问题，且**允许第 8 步独立设置 `degaussq` 测试对 Tc 的影响**。更完整的支持需包括细网格参数（k/q 网格、fsthick、degaussw）分层目录：

```
8.epw-superconductivity/
├── k12x12x6-q4x4x2-fsthick0.4eV-degaussw0.1eV/
│   ├── aniso-mu0.1-Tc20-40-degaussq0.5meV/
│   ├── aniso-mu0.1-Tc20-40-degaussq2.0meV/   # 第 8 步独立设置不同 degaussq
│   └── iso-mu0.1-Tc20-40-degaussq0.5meV/
└── k16x16x8-q4x4x2-fsthick0.4eV-degaussw0.1eV/
    └── ...
```

该架构需从阶段 7 自动提取网格参数并创建嵌套目录，属于后续独立 Feature。

### 相关代码

- 目录命名：[epwflow/src/superconductivity/batch.rs:48](../../../epwflow/src/superconductivity/batch.rs#L48)
- 配置定义（Interpolation）：[epwflow/src/foundation/config.rs:74](../../../epwflow/src/foundation/config.rs#L74)
- 配置定义（Superconductivity）：[epwflow/src/foundation/config.rs:76-89](../../../epwflow/src/foundation/config.rs#L76-L89)
- 配置验证：[epwflow/src/foundation/config.rs:184](../../../epwflow/src/foundation/config.rs#L184)
- `phonon_smearings_mev` 构建逻辑：[epwflow/src/foundation/native_config.rs:607-615](../../../epwflow/src/foundation/native_config.rs#L607-L615)
- EPW 输入文件生成：[epwflow/src/superconductivity/attempt.rs:309](../../../epwflow/src/superconductivity/attempt.rs#L309)

### 验收标准

1. **编译通过**：`cargo build --release --bin epw_superconductivity`
2. **功能验证（回退模式）**：
   - 不在第 8 步设置 `degaussq`
   - 运行超导计算，目录名使用 `spectrum_index` 选择的值
   - 验证目录格式：`{mode}-mu{mu_star}-Tc{T_min}-{T_max}-degaussq{value}meV`
3. **功能验证（独立设置模式）**：
   - 在第 8 步设置 `degaussq = 2.0`
   - 运行超导计算，验证复用第 7 步的 `.ephmat` 文件
   - 验证目录名包含实际使用的 `degaussq = 2.0`
4. **参数扫描验证**：
   - 修改第 8 步的 `degaussq` 后重新运行
   - 验证新旧结果保存在独立目录中，互不覆盖

## 本轮实施确认

> 现在你来落地实现 #49

> 按照新的 #49 重新执行！

本轮遵循更新后的第 8 步显式 degaussq 优先、第 7 步 spectrum_index 回退规则，不采用更新前仅从第 7 步命名的实现。旧双方法、多温度和失败恢复能力继续保留。
