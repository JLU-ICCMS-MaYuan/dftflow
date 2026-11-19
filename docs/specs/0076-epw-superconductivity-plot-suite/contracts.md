# 0076：EPW 超导计算自动绘图关键契约

**关联 Spec**: [spec.md](spec.md)  
**最后更新**: 2026-11-18

## 1. 数据文件格式契约

### 1.1 MgB2.lambda_pairs

**功能**: 所有 (k, k+q) 电声耦合对的 λ 值  
**格式**:
```
# Column 1: lambda_{nk,mk+q}(0)
# Column 2: count/weight (optional)
0.5432
0.8901
...
```

**契约**:
- 每行一个浮点数（λ 值）或两列（λ + 权重）
- 值域: [0, +∞)
- 空文件或缺失: 跳过图 1 左图，记录警告

**解析器签名**:
```rust
fn parse_lambda_pairs(path: &Path) -> Result<Vec<f64>>
```

### 1.2 MgB2.lambda_k_pairs

**功能**: 费米面上各 k 点的电声耦合强度  
**格式**:
```
# ik   lambda_k(0)
1      0.8765
2      0.5432
...
```

**契约**:
- 两列: k 点索引 + λ_k 值
- 值域: λ_k ∈ [0, +∞)
- 顺序: 按 k 点索引升序

**解析器签名**:
```rust
fn parse_lambda_k_pairs(path: &Path) -> Result<Vec<f64>>
```

### 1.3 MgB2.a2f

**功能**: Eliashberg 谱函数与累计耦合强度  
**格式**:
```
# ω(meV)  α²F(ω)  λ(ω,smear1) ... λ(ω,smear12)
0.00      0.0000   0.0000      ... 0.0000
1.23      0.1234   0.0567      ... 0.0578
...
```

**契约**:
- 列数: 2 + N_smearing（通常 2 + 12 = 14 列）
- 列 1: 频率 ω (meV)，单调递增
- 列 2: α²F(ω)，非负
- 列 3-14: 不同 smearing 下的累计 λ(ω)，单调递增
- 绘图使用: 列 1-2（α²F）、列 1 + 列 12（累计 λ，最大 smearing）

**解析器签名**:
```rust
struct A2fData {
    omega_mev: Vec<f64>,
    a2f: Vec<f64>,
    lambda_cumulative: Vec<f64>,
}

fn parse_a2f(path: &Path) -> Result<A2fData>
```

### 1.4 MgB2.imag_aniso_XXX.XX

**功能**: 虚频轴各向异性超导能隙  
**格式**:
```
#  iω(eV)       E_nk-E_F(eV)    Z(iω)           Δ(iω)(eV)
5.414e-03      0.328           1.292           -3.353e-08
5.414e-03      0.328           1.292           -3.353e-08
5.414e-03     -0.252           2.730            0.0168
...
```

**契约**:
- 4 列: iω, E_nk-E_F, Z, Δ
- 列 1: Matsubara 频率 iω (eV)，对应温度由文件名决定（XXX.XX K）
- 列 4: 能隙 Δ (eV)，可正可负（符号物理）
- 行数: N_frequencies × N_states_on_Fermi_surface
- 排序: 频率优先，状态次之

**解析器签名**:
```rust
struct GapData {
    temperature_k: f64,
    frequencies_mev: Vec<f64>,
    gaps_mev: Vec<Vec<f64>>,  // [freq][state]
}

fn parse_imag_aniso(path: &Path) -> Result<GapData>
```

### 1.5 MgB2.pade_aniso_XXX.XX

**功能**: 实频轴各向异性超导能隙（Padé 近似）  
**格式**:
```
#  ω(eV)    ReZ(ω)    ImZ(ω)    ReΔ(ω)(eV)  ImΔ(ω)(eV)
0.0000     1.000     0.000      0.0100      0.0000
0.0001     1.001     0.001      0.0099      0.0001
...
```

**契约**:
- 5 列: ω, ReZ, ImZ, ReΔ, ImΔ
- 列 1: 实频率 ω (eV)
- 列 4-5: 能隙实部/虚部 (eV)
- 文件大小: 通常 50-100 MB（数百万点）
- 采样策略: 每 N 点取 1 点（N = max(1, total_lines / 10000)）

**解析器签名**:
```rust
fn parse_pade_aniso(path: &Path) -> Result<GapData>
```

### 1.6 MgB2.imag_aniso_gap0_XXX.XX

**功能**: Mori Fig. 8 的费米面能隙分布随温度变化。
**格式**: 原生五列分布，与四列 `MgB2.imag_aniso_XXX.XX` 不同。

**契约**:
- 第 1 列为 `T + 归一化分布`，第 2 列为能隙（meV），第 3 列为 T（K），第 4／5 列为归一化／原始分布。
- 每个温度单独绘制第 1 列对第 2 列的蓝色线；不跨温度连接分箱，不把分布误作逐 k 点身份。
- 校验五列、有限值、文件名温度和分布偏移；缺失或损坏时报告，不用完整 imag_aniso 冒充同一种图。

**解析器签名**:
```rust
fn parse_gap0_distribution(path: &Path) -> Result<Vec<(f64, f64)>>
// 返回原生 (T + 分布, gap_mev)，不重新归一化或重采样。
```

### 1.7 MgB2.qdos_XX

**功能**: 超导态密度  
**格式**:
```
# ω(meV)    N_S(ω)    ...
-10.0       0.0000
 -9.5       0.0123
...
```

**契约**:
- 列 1: 频率 ω (meV)
- 列 2: 超导态密度 N_S(ω)（未归一化）
- 归一化: N_S(ω) / N_F（N_F 从 epw1.out 提取）

**解析器签名**:
```rust
struct DosData {
    temperature_k: f64,
    omega_mev: Vec<f64>,
    dos_raw: Vec<f64>,
}

fn parse_qdos(path: &Path) -> Result<DosData>
```

### 1.8 epw1.out

**功能**: EPW 主输出，提取费米面态密度 N_F  
**格式**: 自由格式文本

**契约**:
- 提取目标: `DOS(states/spin/eV/Unit Cell) = <value>`
- 备用模式: `Fermi level (eV) = ... DOS = <value>`
- 默认值: 若无法提取，使用 1.0 并警告

**解析器签名**:
```rust
fn extract_fermi_dos(path: &Path) -> Result<f64>
```

---

## 2. 图表布局契约

### 2.1 图 1: 电声耦合三联图

**尺寸**: 1800×600 px（3×1 布局）  
**子图**:
- 左 (600×600): λ_{nk,mk+q} 分布直方图
- 中 (600×600): λ_{nk} 分布直方图
- 右 (600×600): α²F(ω) + λ(ω) 双 Y 轴

**样式**:
```rust
LeftSubplot {
    x_label: "λ_{nk,mk+q}",
    y_label: "ρ(λ_{nk,mk+q})",
    color: RGB(0, 0, 0),        // 黑色
    bin_width: auto,             // Freedman-Diaconis rule
}

MiddleSubplot {
    x_label: "λ_{nk}",
    y_label: "ρ(λ_{nk})",
    color: RGB(0, 0, 0),
    bin_width: auto,
}

RightSubplot {
    x_label: "ω (meV)",
    y_left: "α²F(ω)",
    y_right: "λ",
    color_a2f: RGB(0, 0, 0),      // 黑色
    color_lambda: RGB(220, 20, 60), // 红色（Crimson）
    line_width: 1.0,
}
```

### 2.2 图 2: 虚频/实频能隙

**尺寸**: 1200×600 px（2×1 布局）  
**子图**:
- 左 (600×600): Δ(iω) 散点
- 右 (600×600): Δ(ω) 散点（采样后）

**样式**:
```rust
LeftSubplot {
    x_label: "iω (meV)",
    y_label: "Δ_{nk}(iω) (meV)",
    marker: Circle { size: 2.0 },
    color: RGB(0, 0, 0),
    alpha: 1.0,
}

RightSubplot {
    x_label: "ω (meV)",
    y_label: "Δ_{nk}(ω) (meV)",
    marker: Circle { size: 1.5 },
    color: RGB(128, 128, 128),  // 灰色
    alpha: 0.3,                  // 半透明
}
```

### 2.3 图 3: 各向异性能隙温度演化

**尺寸**: 600×600 px（单面板）
**面板**: 当前方法、当前参数目录的单个 μ*；不生成第二参数占位。

**样式**:
```rust
SubplotStyle {
    x_label: "Temperature (K)",
    y_label: "Δ_{nk} (meV)",
    line_style: Solid,
    marker: None,
    color: RGB(30, 144, 255),   // DodgerBlue
    line_width: 1.0,
    per_temperature_distribution: true, // 每个温度一条原生 gap0 分布线
}
```

### 2.4 图 4: 各向同性能隙温度演化

**尺寸**: 600×600 px（单面板）

**样式**:
```rust
SubplotStyle {
    x_label: "Temperature (K)",
    y_label: "Δ₀ (meV)",
    line_style: Dashed,
    marker: Circle { size: 4.0 },
    color: RGB(30, 144, 255),
    line_width: 1.0,
}
```

### 2.5 图 5: 超导态密度

**尺寸**: 600×600 px（单面板）

**样式**:
```rust
SubplotStyle {
    x_label: "ω (meV)",
    y_label: "N_S(ω)/N_F",
    line_style: Solid,
    color: RGB(0, 0, 0),
    line_width: 1.0,
}
```

---

## 3. 文件命名契约

### 3.1 输出文件名

```
coupling_strength.<ext>       # 电声耦合三联图
gap_imaginary_real.<ext>      # 虚频/实频能隙
aniso_gap_vs_T.<ext>          # 各向异性温度演化
iso_gap_vs_T.<ext>            # 各向同性温度演化
superconducting_dos.<ext>     # 超导态密度
```

### 3.2 扩展名

根据 `[plot].formats` 配置：
- `svg`: 默认，矢量图
- `png`: 300 DPI 光栅图
- `pdf`: 可嵌入 LaTeX

---

## 4. 错误处理契约

### 4.1 数据文件缺失

| 文件 | 影响 | 行为 |
|------|------|------|
| `lambda_pairs` | 图 1 左图 | 跳过子图，保留中/右图 |
| `lambda_k_pairs` | 图 1 中图 | 跳过子图，保留左/右图 |
| `a2f` | 图 1 右图 | 跳过整个图 1 |
| `imag_aniso_*.00` | 图 2 左图 | 跳过图 2 |
| `pade_aniso_*.00` | 图 2 右图 | 跳过子图，保留左图 |
| `imag_aniso_gap0_*.00` | 图 3 | 跳过图 3 |
| `imag_iso_*.00` | 图 4 | 跳过图 4 |
| `qdos_*` | 图 5 | 跳过图 5 |
| `epw1.out` | 图 5 归一化 | 使用 N_F=1.0 + 警告 |

### 4.2 数据损坏

**检测条件**:
- 列数不匹配
- 非有限值（NaN, Inf）
- 温度标签与频率不一致

**处理**:
```rust
enum DataError {
    ColumnMismatch { expected: usize, got: usize },
    NonFiniteValue { line: usize, column: usize },
    TemperatureMismatch { filename_k: f64, matsubara_k: f64 },
}

// 遇到错误：跳过该文件 + 记录到 plot.log
```

---

## 5. 性能契约

### 5.1 内存上限

- **目标**: 峰值内存 < 2 GB
- **约束**: 70 MB 实频文件采样后占用 < 100 MB

### 5.2 时间上限

- **单个图表**: < 5 秒
- **全部五图（并行）**: < 30 秒（MgB₂ 10 温度点数据）

### 5.3 采样策略

```rust
fn adaptive_sampling_step(file_size: u64) -> usize {
    const TARGET_POINTS: usize = 10_000;
    const AVG_LINE_BYTES: u64 = 50;
    
    let estimated_lines = file_size / AVG_LINE_BYTES;
    ((estimated_lines as usize) / TARGET_POINTS).max(1)
}
```

---

## 6. 单元测试契约

### 6.1 数据解析测试

```rust
#[test]
fn test_parse_lambda_pairs_valid() {
    // 正常文件：1000 行，值域 [0, 2]
}

#[test]
fn test_parse_lambda_pairs_empty() {
    // 空文件：返回 Ok(vec![])
}

#[test]
fn test_parse_a2f_column_mismatch() {
    // 损坏文件：列数不一致，返回 Err
}

#[test]
fn test_parse_pade_aniso_large_file() {
    // 70 MB 文件：采样后 <= 10000 点
}

#[test]
fn test_extract_fermi_dos_multiple_formats() {
    // 测试两种正则模式
}
```

### 6.2 绘图测试

```rust
#[test]
fn test_plot_coupling_triple() {
    // 输入：模拟 CouplingData
    // 输出：SVG 包含 3 个 <g> 子图
}

#[test]
fn test_plot_gap_temperature_empty_states() {
    // 输入：空状态列表
    // 输出：跳过，无错误
}
```

---

## 7. 集成测试契约

### 7.1 完整流程测试

```rust
#[test]
fn test_full_pipeline_mgb2_ubuntu() {
    let work_dir = PathBuf::from("examples/MgB2/ubuntu/run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV");
    let result_dir = temp_dir();
    
    let task = PlotTask::from_work_dir(&work_dir, result_dir.clone()).unwrap();
    task.execute().unwrap();
    
    // 断言：五个文件存在 + 文件大小 > 0
    assert!(result_dir.join("coupling_strength.svg").exists());
    assert!(result_dir.join("gap_imaginary_real.svg").metadata().unwrap().len() > 1000);
    // ...
}
```

### 7.2 CLI 测试

```bash
# 测试 --plot-only
epw_superconductivity --plot-only --config examples/MgB2/ubuntu/input.toml
assert_exists results/8.epw-superconductivity/*/*.svg

# 测试 --skip-plot
epw_superconductivity --skip-plot --config examples/MgB2/ubuntu/input.toml
assert_not_exists results/8.epw-superconductivity/*/*.svg
```

---

## 8. 文档契约

### 8.1 必需文档

- [ ] `epwflow/README.md` 新增 "自动绘图" 章节
- [ ] `docs/overview/epwflow/native-stage-config.md` 补充绘图配置
- [ ] `docs/specs/0076-epw-superconductivity-plot-suite/acceptance.md`

### 8.2 代码注释

每个解析函数必须包含：
- 文件格式说明
- 列索引含义
- 异常情况处理

---

**契约版本**: v1.0  
**创建时间**: 2026-11-18  
**审核状态**: 待审核

## 9. 原生格式实测补充（2026-11-18）

- 带 `distribution` 头的原生 λ 文件为三列分箱密度，直接绘制第 1／2 列，不再次统计箱中心。
- 2026-11-19 修正：原生 gap0 五列分布正是 Mori Fig. 8 的数据源，直接绘制第 1／2 列；此前改读完整 imag_aniso 首频率状态的解释错误，已撤回。完整 imag_aniso 仍用于图 2。
- 原生 Padé 六列包含第 2 列电子能量，实部能隙是第 5 列；五列契约样本取第 4 列。两种均由 eV 转为 meV。
- 原生 iso 首行 `w [eV] znorm(w) delta(w) [eV]` 没有注释符，明确识别该列头后读取第 3 列能隙。
- `source.a2f` 为链接时，缺失的 λ 分布只从已选链接目标同前缀伴随文件读取，不搜索其他参数网格。
- 费米面 DOS 优先读取 `epw1.out`，其次读取当前成对日志 `epw.out`；每组参数独立归一化。
- 自动触发的实际生产边界为编号第 8 步的 `Scientific status = completed`；手动 `--plot-only` 允许输出部分诊断。

真实数据、合成样本、性能测量及待满足项分别记录于 [验收记录](acceptance.md)。


## 2026-11-19 单 μ* 展示契约

μ* 列表实现暂缓。两类能隙温度图与 superconducting_dos 均只包含一个 600×600 面板，当前目录的 μ* 显示在标题中；不读取兄弟方法或其他参数目录，不创建第二个 μ* 占位子图。电声耦合三联和虚／实频双联保留。qdos 图仍使用简洁名称 superconducting_dos，数据来自各自的原生 qdos 文件。
