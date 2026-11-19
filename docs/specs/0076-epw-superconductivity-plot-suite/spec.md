# 0076：EPW 超导计算自动绘图套件

**关联 Issue**: [#76](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/76)  
**状态**: 专项回归通过；全量保留 109 项基线失败，真实完整五图待数据补齐  
**创建时间**: 2026-11-18  
**最后更新**: 2026-11-18

## 1. 目标

为 EPW 超导计算工作流提供自动绘图功能，在 Tc 求解完成后生成五类对标 Mori Tutorial (Phys. Rev. B 87, 024505) 的物理分析图表，并按照原始计算目录结构镜像输出到 `results/8.epw-superconductivity/`。

## 2. 背景

### 2.1 当前状态

- EPW 超导计算已完成 Tc 求解流程（Issue #61, #75）
- 计算输出包含丰富的物理数据文件（λ 分布、能隙、态密度等）
- 用户需手动使用 gnuplot 或 Python 绘图
- 缺乏标准化的可视化输出

### 2.2 参考论文

Mori Tutorial: [Phys. Rev. B 87, 024505 (2013)](https://journals.aps.org/prb/abstract/10.1103/PhysRevB.87.024505)  
PDF 位置: `examples/MgB2/Wed.9.Mori.pdf`

论文给出了 EPW 超导计算的五类标准图表：
- Fig. 6: 电声耦合强度三联图
- Fig. 7: 虚频/实频能隙对比
- Fig. 8: 各向异性能隙温度演化（MgB₂）
- Fig. 2: 各向同性能隙温度演化（Pb）
- Fig. 9: 超导态密度

## 3. 需求定义

### 3.1 功能需求

#### FR-001: 自动触发绘图
- **描述**: `epw_superconductivity` CLI 完成 Tc 求解后自动绘图
- **触发条件**: `manifest.json` 中 `complete: true`
- **输出位置**: `results/8.epw-superconductivity/<相对路径>`

#### FR-002: 手动绘图模式
- **描述**: 提供 `--plot-only` 参数跳过计算直接绘图
- **用例**: 用户调整绘图参数或补绘缺失图表
- **行为**: 读取已有数据文件，不触发 EPW 计算

#### FR-003: 图 1 - 电声耦合三联图
- **数据源**:
  - 左图: `MgB2.lambda_pairs` (列 1:2)
  - 中图: `MgB2.lambda_k_pairs`
  - 右图: `MgB2.a2f` (列 1:2 为 α²F, 列 1:12 为累计 λ)
- **布局**: 水平三联图，共享 X 轴类型（能量/频率）
- **样式**:
  - 左/中: 黑色直方图（bin size 自适应）
  - 右: 双 Y 轴，黑线（α²F）+ 红线（λ）

#### FR-004: 图 2 - 虚频/实频能隙
- **数据源**:
  - 左图: `MgB2.imag_aniso_XXX.XX` (列 1:4)
  - 右图: `MgB2.pade_aniso_XXX.XX` (列 1:5)
- **布局**: 水平双联图
- **样式**:
  - 左: 黑色密集散点（无连线）
  - 右: 灰色半透明散点（采样后绘制，避免百万点）
- **性能约束**: 右图文件 ~70MB，需采样（如每 100 点取 1 点）

#### FR-005: 图 3 - 各向异性能隙温度演化
- **数据源**: 跨温度的 `MgB2.imag_aniso_gap0_XXX.XX` 文件
- **布局**: 左右并排（不同化学势 μ_c）
- **样式**: 蓝色实线 + 圆点标记，每条线对应一个 k 点
- **X 轴**: Temperature (K)，**Y 轴**: Δ_{nk}(T) (meV)

#### FR-006: 图 4 - 各向同性能隙温度演化
- **数据源**: `MgB2.imag_iso_XXX.XX` 提取前导能隙
- **布局**: 左右并排（不同参数组合）
- **样式**: 蓝色点 + 虚线连接
- **X 轴**: Temperature (K)，**Y 轴**: Δ₀ (meV)

#### FR-007: 图 5 - 超导态密度
- **数据源**: 
  - `MgB2.qdos_XX` 列 2
  - `epw1.out` 中的费米面 DOS（归一化因子）
- **布局**: 左右并排（10 K 时的不同参数）
- **样式**: 黑色实线
- **X 轴**: ω (meV)，**Y 轴**: N_S(ω)/N_F
- **归一化**: qdos 第 2 列 / epw1.out 中的 DOS 值

#### FR-008: 多格式输出
- **支持格式**: SVG（默认）、PNG、PDF
- **配置**: 根级 `[plot].formats = ["svg", "png"]`（复用现有声子配置）
- **命名规则**:
  - `fig1_coupling_strength.svg`
  - `fig2_gap_imaginary_real.svg`
  - `fig3_aniso_gap_vs_T.svg`
  - `fig4_iso_gap_vs_T.svg`
  - `fig5_superconducting_dos.svg`

#### FR-009: 目录层级镜像
- **输入**: `8.epw-superconductivity/01.k8x8x4-q4x4x4-.../aniso-mu0.1-.../`
- **输出**: `results/8.epw-superconductivity/01.k8x8x4-q4x4x4-.../aniso-mu0.1-.../`
- **行为**: 保持相对路径一致，自动创建中间目录

### 3.2 非功能需求

#### NFR-001: 性能
- **目标**: MgB₂ 完整数据集（10+ 温度点）绘图 < 30 秒
- **约束**: 70MB 实频文件不触发 OOM（< 2GB 内存峰值）
- **策略**: 
  - 流式解析大文件
  - 采样策略：每 N 点取 1 点（N 根据文件大小自适应）
  - 并行绘图：五类图表独立，使用 `rayon`

#### NFR-002: 容错性
- **缺失文件**: 跳过对应图表，记录警告，不中断整体流程
- **损坏数据**: 检查列数、温度标签一致性，拒绝明显错误的文件
- **部分温度**: 温度演化图容许不连续温度点

#### NFR-003: 可维护性
- **模块化**: 五类图表独立模块，便于后续扩展
- **测试**: 使用 `examples/MgB2/ubuntu/run` 真实数据编写集成测试
- **文档**: 更新 `epwflow/README.md` 和 `native-stage-config.md`

## 4. 架构设计

### 4.1 模块结构

```
epwflow/src/superconductivity/
├── plot/
│   ├── mod.rs              # 公共接口：PlotTask, PlotConfig
│   ├── coupling.rs         # FR-003: 电声耦合三联图
│   ├── gap_frequency.rs    # FR-004: 虚频/实频能隙
│   ├── gap_temperature.rs  # FR-005, FR-006: 能隙温度演化
│   ├── dos.rs              # FR-007: 超导态密度
│   └── io.rs               # 数据文件解析器
└── ...

crates/qf-plot/src/
├── superconductivity.rs    # 底层绘图原语（复用 phonon 样式）
└── ...
```

### 4.2 数据结构

```rust
// epwflow/src/superconductivity/plot/mod.rs

pub struct PlotTask {
    pub work_dir: PathBuf,
    pub result_dir: PathBuf,
    pub formats: Vec<PlotFormat>,
    pub data_files: DataFiles,
}

pub struct DataFiles {
    pub lambda_pairs: Option<PathBuf>,
    pub lambda_k_pairs: Option<PathBuf>,
    pub a2f: Option<PathBuf>,
    pub imag_aniso: Vec<(f64, PathBuf)>,  // (temperature_K, path)
    pub pade_aniso: Vec<(f64, PathBuf)>,
    pub imag_iso: Vec<(f64, PathBuf)>,
    pub qdos: Vec<(f64, PathBuf)>,
    pub epw1_out: Option<PathBuf>,
}

pub enum PlotFormat {
    Svg,
    Png,
    Pdf,
}

// epwflow/src/superconductivity/plot/io.rs

pub struct CouplingData {
    pub lambda_pairs: Vec<f64>,
    pub lambda_k: Vec<f64>,
    pub a2f_omega_mev: Vec<f64>,
    pub a2f_values: Vec<f64>,
    pub lambda_cumulative: Vec<f64>,
}

pub struct GapData {
    pub temperature_k: f64,
    pub frequencies_mev: Vec<f64>,
    pub gaps_mev: Vec<Vec<f64>>,  // [freq][state]
}

pub struct DosData {
    pub temperature_k: f64,
    pub omega_mev: Vec<f64>,
    pub dos_normalized: Vec<f64>,
    pub fermi_dos: f64,
}
```

### 4.3 数据流

```
┌─────────────────────┐
│ epw_superconductivity│
│ CLI 主流程           │
└──────────┬──────────┘
           │ Tc 求解完成
           ▼
┌─────────────────────┐
│ plot::scan_data()   │
│ 扫描 manifest.json  │
│ 识别已完成 attempt  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ plot::io::parse_*() │
│ 解析 MgB2.* 文件    │
│ 转换为 Rust 结构体  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ rayon::par_iter()   │
│ 并行绘制五类图表    │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ qf_plot::render()   │
│ 生成 SVG/PNG/PDF    │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ 写入 results/ 镜像  │
│ 保持相对路径一致    │
└─────────────────────┘
```

### 4.4 CLI 集成

```rust
// epwflow/src/bin/epw_superconductivity.rs

#[derive(Parser)]
struct Args {
    // ... 现有参数 ...
    
    /// 仅绘图，不执行 Tc 求解
    #[arg(long)]
    plot_only: bool,
    
    /// 跳过绘图（即使 Tc 求解完成）
    #[arg(long)]
    skip_plot: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    if args.plot_only {
        // 直接读取已有数据绘图
        plot::execute_plot_only(&args.config)?;
        return Ok(());
    }
    
    // ... 现有 Tc 求解逻辑 ...
    
    if !args.skip_plot && manifest.complete {
        plot::auto_plot(&manifest, &args.config)?;
    }
    
    Ok(())
}
```

## 5. 实施细节

### 5.1 大文件采样策略

```rust
// epwflow/src/superconductivity/plot/io.rs

const LARGE_FILE_THRESHOLD: u64 = 10 * 1024 * 1024; // 10 MB
const TARGET_POINTS: usize = 10_000;

fn parse_pade_aniso(path: &Path) -> Result<GapData> {
    let metadata = fs::metadata(path)?;
    let file_size = metadata.len();
    
    if file_size > LARGE_FILE_THRESHOLD {
        // 采样模式：估算总行数，计算采样步长
        let estimated_lines = file_size / 50; // 假设每行 ~50 字节
        let sample_step = (estimated_lines / TARGET_POINTS as u64).max(1);
        
        parse_with_sampling(path, sample_step as usize)
    } else {
        parse_all_lines(path)
    }
}
```

### 5.2 费米面 DOS 提取

```rust
// epwflow/src/superconductivity/plot/io.rs

fn extract_fermi_dos(epw1_out: &Path) -> Result<f64> {
    let content = fs::read_to_string(epw1_out)?;
    
    // 匹配多种格式
    let patterns = [
        r"DOS\(states/spin/eV/Unit Cell\) = \s*([\d.]+)",
        r"Fermi level \(eV\) = \s*[\d.]+\s+DOS = \s*([\d.]+)",
    ];
    
    for pattern in &patterns {
        if let Some(caps) = Regex::new(pattern)?.captures(&content) {
            return Ok(caps[1].parse()?);
        }
    }
    
    // 默认值 + 警告
    eprintln!("Warning: Could not extract Fermi DOS, using default 1.0");
    Ok(1.0)
}
```

### 5.3 绘图样式配置

```rust
// crates/qf-plot/src/superconductivity.rs

pub struct ScStyle {
    pub line_width: f64,
    pub point_size: f64,
    pub colors: ScColors,
}

pub struct ScColors {
    pub primary: RGB,       // 黑色 #000000
    pub secondary: RGB,     // 红色 #DC143C（累计 λ）
    pub aniso_gap: RGB,     // 蓝色 #1E90FF（温度演化）
    pub real_gap_alpha: f64, // 实频能隙透明度 0.3
}

impl Default for ScStyle {
    fn default() -> Self {
        Self {
            line_width: 1.0,  // 复用电子能带宽度
            point_size: 2.0,
            colors: ScColors {
                primary: RGB::new(0, 0, 0),
                secondary: RGB::new(220, 20, 60),
                aniso_gap: RGB::new(30, 144, 255),
                real_gap_alpha: 0.3,
            },
        }
    }
}
```

## 6. 测试策略

### 6.1 单元测试

```rust
// epwflow/src/superconductivity/plot/io.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_lambda_pairs() {
        let path = Path::new("test_data/MgB2.lambda_pairs");
        let data = parse_lambda_pairs(path).unwrap();
        assert!(data.len() > 0);
        assert!(data.iter().all(|x| x.is_finite()));
    }
    
    #[test]
    fn test_parse_large_pade_file() {
        // 测试 70MB 文件采样
        let path = Path::new("examples/MgB2/ubuntu/run/.../MgB2.pade_aniso_020.00");
        let data = parse_pade_aniso(path).unwrap();
        assert!(data.gaps_mev.len() <= TARGET_POINTS);
    }
    
    #[test]
    fn test_fermi_dos_extraction() {
        let path = Path::new("test_data/epw1.out");
        let dos = extract_fermi_dos(path).unwrap();
        assert!(dos > 0.0 && dos < 100.0); // 合理范围
    }
}
```

### 6.2 集成测试

```rust
// epwflow/tests/superconductivity_plot.rs

#[test]
fn test_full_plot_pipeline() {
    let work_dir = PathBuf::from("examples/MgB2/ubuntu/run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV");
    let result_dir = PathBuf::from("/tmp/test_plot_output");
    
    let task = PlotTask::from_work_dir(&work_dir, result_dir.clone()).unwrap();
    task.execute().unwrap();
    
    // 检查五类图表是否生成
    assert!(result_dir.join("fig1_coupling_strength.svg").exists());
    assert!(result_dir.join("fig2_gap_imaginary_real.svg").exists());
    assert!(result_dir.join("fig3_aniso_gap_vs_T.svg").exists());
    assert!(result_dir.join("fig4_iso_gap_vs_T.svg").exists());
    assert!(result_dir.join("fig5_superconducting_dos.svg").exists());
}

#[test]
fn test_plot_only_mode() {
    // 测试 --plot-only 不触发计算
    let output = Command::new("epw_superconductivity")
        .arg("--plot-only")
        .arg("--config").arg("examples/MgB2/ubuntu/input.toml")
        .output()
        .unwrap();
    
    assert!(output.status.success());
    // 验证没有新的 Slurm 作业提交
}
```

### 6.3 视觉回归测试

```bash
# 手动验收步骤
cd examples/MgB2/ubuntu
epw_superconductivity --plot-only --config input.toml

# 对比生成的图表与 Wed.9.Mori.pdf 中的参考图
evince results/8.epw-superconductivity/*/fig*.svg &
evince Wed.9.Mori.pdf &
```

## 7. 文档更新

### 7.1 README.md

```markdown
## 超导计算自动绘图

EPW 超导计算完成后会自动生成五类物理分析图：

1. **电声耦合三联图**: λ 分布、费米面 λ_k、Eliashberg 谱
2. **虚频/实频能隙**: Δ(iω) 和 Δ(ω) 对比
3. **各向异性能隙温度演化**: 多 k 点的 Δ_nk(T)
4. **各向同性能隙温度演化**: Δ₀(T) 曲线
5. **超导态密度**: N_S(ω)/N_F 归一化谱

图表输出到 `results/8.epw-superconductivity/` 并保持原始目录层级。

### 手动绘图

跳过计算直接绘图（用于调整参数或补绘）：

```bash
epw_superconductivity --plot-only --config input.toml
```

### 配置格式

在根级配置图片格式（默认 SVG）：

```toml
[plot]
formats = ["svg", "png"]  # 同时输出 SVG 和 PNG
```
```

### 7.2 native-stage-config.md

在 "Tc 求解" 章节后新增：

```markdown
## 自动绘图

完成 Tc 求解后，系统自动生成对标 Mori Tutorial (Phys. Rev. B 87, 024505) 的五类图表：

- `fig1_coupling_strength`: 电声耦合分析（三联图）
- `fig2_gap_imaginary_real`: 虚频/实频能隙对比
- `fig3_aniso_gap_vs_T`: 各向异性能隙温度演化
- `fig4_iso_gap_vs_T`: 各向同性能隙温度演化（如有 iso 计算）
- `fig5_superconducting_dos`: 超导态密度

图表自动镜像到 `results/8.epw-superconductivity/`，保持与计算输出相同的目录层级。

若需要跳过绘图（如调试 Tc 求解），使用 `--skip-plot` 参数。
```

## 8. 风险与缓解

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| 70MB 实频文件 OOM | 高 | 中 | 采样策略 + 流式解析 |
| epw1.out 格式变化 | 中 | 低 | 多版本正则 + 默认值 |
| 温度点缺失 | 低 | 中 | 容错跳过 + 警告提示 |
| 绘图时间过长 | 中 | 低 | 并行绘图 + 性能测试 |
| 论文图表不匹配 | 高 | 低 | 视觉回归 + 用户反馈 |

## 9. 验收标准

### 9.1 功能完整性

- [ ] 五类图表全部绘制成功（使用 MgB₂ ubuntu 真实数据）
- [ ] 目录层级与 `8.epw-superconductivity` 完全一致
- [ ] 支持 `--plot-only` 跳过计算
- [ ] 支持 `--skip-plot` 禁用绘图
- [ ] 支持 SVG/PNG/PDF 多格式输出

### 9.2 数据准确性

- [ ] 图 1 与 Mori Fig. 6 视觉对照吻合（λ 峰值位置、α²F 形状）
- [ ] 图 2 虚频能隙单调递减
- [ ] 图 3-4 温度演化曲线单调性符合物理预期（温度升高能隙减小）
- [ ] 图 5 超导态密度归一化正确（N_F 读取准确）

### 9.3 性能要求

- [ ] MgB₂ ubuntu 全数据（10+ 温度点）绘图 < 30 秒
- [ ] 70MB 实频文件不触发 OOM（内存峰值 < 2GB）
- [ ] 五类图表并行绘制（CPU 多核利用）

### 9.4 鲁棒性

- [ ] 缺失 `lambda_pairs` 时跳过图 1 左图，不中断流程
- [ ] 缺失 `epw1.out` 时使用默认 N_F=1.0 并警告
- [ ] 仅有部分温度点时正常绘制图 3-4

### 9.5 代码质量

- [ ] 通过所有单元测试（数据解析、采样逻辑）
- [ ] 通过集成测试（完整绘图流程）
- [ ] 代码审查无阻塞问题
- [ ] 文档更新完整（README + native-stage-config）

## 10. 时间线

| 阶段 | 任务 | 预计时间 | 负责人 |
|------|------|----------|--------|
| Phase 1 | 数据解析模块 + 单元测试 | 2 天 | GPT |
| Phase 2 | 核心绘图实现（五类图） | 3 天 | GPT |
| Phase 3 | CLI 集成 + 目录镜像 | 1 天 | GPT |
| Phase 4 | 集成测试 + 文档 | 1 天 | GPT |
| 验收 | 视觉回归 + 性能测试 | 1 天 | Claude |

**总计**: 约 7-8 天

## 11. 参考资料

### 11.1 外部资源

- 论文: [Phys. Rev. B 87, 024505 (2013)](https://journals.aps.org/prb/abstract/10.1103/PhysRevB.87.024505)
- Tutorial PDF: `examples/MgB2/Wed.9.Mori.pdf`
- EPW 官方文档: [EPW v5.9](https://docs.epw-code.org/)

### 11.2 内部代码

- 现有绘图模块: `crates/qf-plot/src/electronic.rs`（能带对比）
- 声子绘图: `epwflow/src/delivery/native_plot.rs`
- 格式配置: `crates/qf-plot/src/formats.rs`
- 超导求解: `epwflow/src/superconductivity/`

### 11.3 测试数据

- MgB₂ 完整数据集: `examples/MgB2/ubuntu/run/8.epw-superconductivity/`
- 包含文件:
  - `MgB2.lambda_pairs`
  - `MgB2.lambda_k_pairs`
  - `MgB2.a2f`
  - `MgB2.imag_aniso_*.00`
  - `MgB2.pade_aniso_*.00` (~70MB)
  - `MgB2.qdos_*`
  - `epw1.out`

## 12. 后续扩展

### 短期（不在本 Spec 范围）

- 可选组合图 `summary_all.svg`（五图合并）
- 支持用户自定义颜色主题
- 各向异性能隙的 3D 费米面投影

### 长期

- 交互式 HTML 版本（使用 plotly.js）
- 与 Jupyter Notebook 集成
- 自动生成 LaTeX 论文插图代码

---

**Spec 作者**: Claude Opus 5.5  
**审核**: 待定  
**批准**: 待定

## 13. 本轮回测边界（2026-11-18）

本轮授权为“你来完成 #76的剩余回测”。保留前文设计与原验收门，实际测试结果及缺口见 [acceptance.md](acceptance.md)；任务以 [tasks.md](tasks.md) 的技术证据为准，不将合成样本通过写成真实 MgB₂ 五图通过。

原生 EPW 的三列 λ 分布与五列 gap0 分布和示意样本不同。λ 分布保留既有密度，逐状态温度曲线从完整四列 imag_aniso 提取首 Matsubara 频率，不把分布箱当成 k 点。手动绘图允许检查已有部分数据；自动触发依赖编号第 8 步的科学完成核验，不凭文件存在或进程零退出码放行。

本轮不新增科学计算，不修改原始数据、输入参数或质量标准。Plan 中的增量时间戳和解析缓存尚未实现，不作为当前功能；本轮重绘直接读取已有文本，不新增缓存机制。

## 14. 方案 B：完整五类图补充计算（2026-11-19）

### 14.1 当前数据状态盘点

**✅ 已有数据**（第 7 步和第 8 步已生成，无需重新计算）：

1. **图 1 电声耦合三联图数据**（第 7 步 epw-fine 已生成）：
   - `run/7.epw-fine/.../MgB2.lambda_pairs` - λ_{nk,mk+q} 分布（44 KB）
   - `run/7.epw-fine/.../MgB2.lambda_k_pairs` - 费米面 λ_k（15 KB）
   - `run/8.epw-superconductivity/.../source.a2f` - α²F(ω) 和累计 λ

2. **图 3 各向异性能隙温度演化**（第 8 步已生成）：
   - `MgB2.imag_aniso_gap0_020.00` - 20K 各向异性能隙分布
   - `MgB2.imag_aniso_gap0_030.00` - 30K 各向异性能隙分布
   - `MgB2.imag_aniso_gap0_040.00` - 40K 各向异性能隙分布

3. **图 4 各向同性能隙温度演化**（第 8 步已生成）：
   - `MgB2.imag_iso_020.00` - 20K 各向同性能隙
   - `MgB2.imag_iso_030.00` - 30K 各向同性能隙
   - `MgB2.imag_iso_040.00` - 40K 各向同性能隙

**❌ 缺失数据**（需要重新计算第 8 步）：

1. **图 2 实频能隙**（需要 Padé 解析延拓）：
   - `MgB2.pade_aniso_020.00` - 20K 实频能隙（6 列：ω, ε_nk, Re[Z], Im[Z], Re[Δ], Im[Δ]）
   - `MgB2.pade_aniso_030.00` - 30K 实频能隙
   - `MgB2.pade_aniso_040.00` - 40K 实频能隙

2. **图 5 超导态密度**（可选，需要 qdos 输出）：
   - `MgB2.qdos_020` - 20K 超导态密度
   - `MgB2.qdos_030` - 30K 超导态密度
   - `MgB2.qdos_040` - 40K 超导态密度

### 14.2 补充计算参数说明

根据 **Mori PDF 第 23-27 页**的说明，需要在第 8 步配置中添加以下参数：

#### EPW 输入参数

**`lpade = .true.`**（启用 Padé 解析延拓）
- **功能**：将虚频轴（imaginary axis）上的能隙 Δ(iω_n) 解析延拓到实频轴（real axis）Δ(ω)
- **原理**：使用 Padé 近似（有理函数逼近）将 Matsubara 频率点延拓到连续实频
- **输出**：生成 `MgB2.pade_aniso_XXX.XX` 和 `MgB2.pade_iso_XXX.XX` 文件

**`npade = 50`**（Padé 延拓采样比例）
- **功能**：使用 50% 的 Matsubara 频率点进行 Padé 延拓
- **取值范围**：通常 30-70%
- **建议值**：50（平衡精度和稳定性）

**qdos 自动输出**
- **功能**：EPW 5.9 在 `lpade` 成功后调用 `dos_quasiparticle`
- **输出**：生成 `MgB2.qdos_XX` 文件（首行为 `w [eV] N_S/N_F`，第二列已归一化）
- **注意**：不添加 `qdos_k`；绘图不再重复除以 `N_F`

### 14.3 具体操作步骤

**参考文档**：[operation-manual.md](operation-manual.md)（完整操作手册，包含详细命令和问题排查）

#### 步骤 1：修改配置文件

编辑 `examples/MgB2/ubuntu/input.toml`：

```toml
# 在 [stages."8.epw-superconductivity".iso] 部分（约第 321 行）
lpade = true          # 从 false 改为 true
npade = 50            # 新增这一行

# 在 [stages."8.epw-superconductivity".aniso] 部分（约第 360 行）
lpade = true          # 从 false 改为 true
npade = 50            # 新增这一行

# 图 5 不需要额外开关；lpade 成功后 EPW 5.9 自动生成 qdos_*
```

#### 步骤 2：清理旧数据并重新计算

```bash
cd /home/mayuan/code/dftflow/examples/MgB2/ubuntu

# 清理旧的第 8 步数据（因为输入参数变化）
rm -rf run/8.epw-superconductivity

# 重新提交第 8 步超导计算
epw_superconductivity --stage superconductivity -c input.toml -s
```

**预计运行时间**：
- 各向同性（iso）：约 5-10 分钟（12 MPI 进程）
- 各向异性（aniso）：约 15-30 分钟（12 MPI 进程）

#### 步骤 3：监控和验证

```bash
# 监控作业状态
squeue -u $USER

# 查看日志
tail -f run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/slurm-epw.log

# 计算完成后，验证新生成的 Padé 文件
ls -lh run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/MgB2.pade_aniso_*

# 预期输出：
# MgB2.pade_aniso_020.00  (~1-10 MB，取决于采样点数)
# MgB2.pade_aniso_030.00
# MgB2.pade_aniso_040.00
```

#### 步骤 4：自动绘图或手动触发

计算完成后自动触发绘图，或手动执行：

```bash
# 手动触发绘图（只读模式，不重新计算）
epw_superconductivity --plot-only -c input.toml
```

#### 步骤 5：验证五类图完整性

```bash
# 检查生成的图表文件
ls -lh results/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/*/fig*.{svg,png,pdf}

# 应该看到五类图（每类 SVG/PNG/PDF）：
# fig1_coupling_strength.*           - 电声耦合三联图
# fig2_gap_imaginary_real.*          - 虚频/实频能隙双联图（实频部分不再为空）
# fig3_aniso_gap_vs_T.*              - 各向异性能隙温度演化
# fig4_iso_gap_vs_T.*                - 各向同性能隙温度演化
# fig5_superconducting_dos.*         - 超导态密度（如果已有 qdos 数据）

# 对比 Mori PDF 进行视觉验证
evince ../../../examples/MgB2/Wed.9.Mori.pdf &
eog results/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/fig*.png &
```

### 14.4 科学验证标准

**图 2 实频能隙**（对比 Mori Fig. 7）：
- [ ] 左图：虚频能隙 Δ(iω) 随频率单调递减
- [ ] 右图：实频能隙 Re[Δ(ω)] 在费米面附近有明显结构
- [ ] 右图：Im[Δ(ω)] 在能隙边缘有峰值
- [ ] 实频面板不再为空（之前缺失 Padé 数据）

**图 5 超导态密度**（对比 Mori Fig. 9，如果生成）：
- [ ] N_S(ω)/N_F 在 ω=0 附近为零（能隙特征）
- [ ] 在能隙边缘有 BCS 型相干峰
- [ ] 高能部分趋近于 1（正常态）

### 14.5 后续任务

完成五类图的补充计算和验证后，需要：

1. **更新 acceptance.md**：记录完整五类图的真实数据验收结果
2. **修复 109 项测试失败**：更新测试夹具以适配 Issue #51 的日志格式变更（schema_version=3）
3. **更新 tasks.md**：标记 T024（真实完整五图验证）为完成
4. **提交 #76**：完成全部五类图的功能实现和真实数据验收

### 14.6 常见问题

**Q1: Padé 延拓失败，提示 "Not enough Matsubara points"**

A: 增加 `nqstep` 参数（Matsubara 频率点数）或减小 `npade` 参数：
```toml
nqstep = 1000  # 从 500 增加到 1000
npade = 30     # 从 50 减小到 30
```

**Q2: qdos 文件未生成**

A: 检查 EPW 版本（需要 5.8+），或跳过图 5 的生成：
```bash
grep "Program EPW" run/8.epw-superconductivity/.../epw.out
```

**Q3: 绘图失败但科学计算成功**

A: 查看绘图警告并检查数据文件格式：
```bash
epw_superconductivity --plot-only -c input.toml 2>&1 | grep -i "warning\|error"
head -5 run/8.epw-superconductivity/.../MgB2.pade_aniso_020.00
```
