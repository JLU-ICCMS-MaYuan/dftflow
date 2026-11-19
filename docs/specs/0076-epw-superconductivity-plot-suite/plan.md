# Issue #76: EPW 超导计算自动绘图套件

## 需求概述

在 EPW 超导计算完成后，自动绘制对标 Mori Tutorial (Phys. Rev. B 87, 024505) 的五类核心分析图，并按照计算目录层级镜像输出到 `results/8.epw-superconductivity/` 中。

## 参考论文图表分析

根据 `/home/mayuan/code/dftflow/examples/MgB2/Wed.9.Mori.pdf` 和用户提供的截图，需要绘制以下图表：

### 图 1：电声耦合强度（Fig. 6 in Mori）
- **左图**：各向异性电声耦合 λ_{nk,mk+q}(0) 分布
  - 数据来源：`MgB2.lambda_pairs` 文件（列 1:2）
  - X 轴：λ_{nk,mk+q}，Y 轴：ρ(λ_{nk,mk+q})
  
- **中图**：费米面上各向异性电声耦合 λ_{nk}(0)
  - 数据来源：`MgB2.lambda_k_pairs` 文件
  - X 轴：λ_{nk}，Y 轴：ρ(λ_{nk})
  
- **右图**：Eliashberg 谱函数与累计耦合强度
  - 数据来源：`MgB2.a2f` 文件（列 1:2 为 α²F(ω)，列 1:12 为累计 λ）
  - 左 Y 轴：α²F(ω)（黑线），右 Y 轴：λ(ω)（红线），X 轴：ω (meV)

### 图 2：虚频轴超导能隙（Fig. 7 in Mori）
- **左图**：虚频轴能隙 Δ_{nk}(iω)
  - 数据来源：`MgB2.imag_aniso_XXX.XX` 文件（列 1:4）
  - X 轴：iω (meV)，Y 轴：Δ_{nk}(iω) (meV)
  - 显示所有 k 点的黑色散点

- **右图**：实频轴能隙（Padé 近似）
  - 数据来源：`MgB2.pade_aniso_XXX.XX` 文件（列 1:5）
  - X 轴：ω (meV)，Y 轴：Δ_{nk}(ω) (meV)
  - 密集散点，约 70MB 数据量

### 图 3：各向异性费米面能隙随温度演化（Fig. 8 in Mori）
- **左右并排**：不同化学势 μ_c 下的能隙 - 温度曲线
  - 数据来源：跨温度的 `MgB2.imag_aniso_gap0_XXX.XX` 文件
  - X 轴：Temperature (K)，Y 轴：Δ_{nk}(T) (meV)
  - 每个 k 点一条蓝色曲线，显示各向异性

### 图 4：各向同性能隙随温度演化（Fig. 2 in Mori - 铅的例子）
- **左右并排**：不同参数下的 Δ₀(T) 曲线
  - 数据来源：从虚频轴 `MgB2.imag_iso_XXX.XX` 提取前导能隙
  - X 轴：Temperature (K)，Y 轴：Δ₀ (meV)
  - 蓝色点+虚线连接

### 图 5：超导态密度 N_S(ω)/N_F（Fig. 9 in Mori）
- **左右并排**：10 K 时的超导态密度
  - 数据来源：`MgB2.qdos_XX` 文件列 2，除以 `epw1.out` 中的费米面 DOS
  - X 轴：ω (meV)，Y 轴：N_S(ω)/N_F
  - 显示双能隙结构

## 数据文件映射

### EPW 5.9 输出文件清单
```
<work_dir>/
├── MgB2.lambda_pairs           # 所有 (k,k+q) 对的 λ 值
├── MgB2.lambda_k_pairs         # 费米面上各 k 点的 λ_k 值
├── MgB2.a2f                    # Eliashberg 谱函数（12 列 smearing）
├── MgB2.imag_aniso_XXX.XX      # 虚频轴各向异性能隙（温度 XXX.XX K）
├── MgB2.imag_aniso_gap0_XXX.XX # 虚频轴前导能隙（用于温度演化）
├── MgB2.pade_aniso_XXX.XX      # 实频轴能隙（Padé 近似，~70MB）
├── MgB2.imag_iso_XXX.XX        # 虚频轴各向同性能隙
├── MgB2.qdos_XX                # 各温度超导态密度
└── epw1.out                    # 费米面 DOS（归一化用）
```

### 目录层级镜像规则
```
8.epw-superconductivity/
├── 01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/
│   ├── aniso-mu0.1-Tc20-40-degaussq0.5meV/
│   │   ├── MgB2.imag_aniso_020.00
│   │   └── ... (EPW 原始输出)
│   └── ...
└── ...

results/8.epw-superconductivity/
├── 01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/
│   ├── aniso-mu0.1-Tc20-40-degaussq0.5meV/
│   │   ├── fig1_coupling_strength.svg      # 三联图
│   │   ├── fig2_gap_imaginary_real.svg     # 虚频 + 实频
│   │   ├── fig3_aniso_gap_vs_T.svg         # 各向异性温度演化
│   │   ├── fig4_iso_gap_vs_T.svg           # 各向同性温度演化
│   │   ├── fig5_superconducting_dos.svg    # 超导态密度
│   │   └── summary_all.svg                 # 可选：组合图
│   └── ...
└── ...
```

## 技术架构

### 模块分层
```
epwflow/src/superconductivity/
├── plot/
│   ├── mod.rs                  # 公共绘图接口
│   ├── coupling.rs             # 图 1：电声耦合三联图
│   ├── gap_frequency.rs        # 图 2：虚频/实频能隙
│   ├── gap_temperature.rs      # 图 3-4：能隙 - 温度曲线
│   ├── dos.rs                  # 图 5：超导态密度
│   └── summary.rs              # 可选：五图组合版
└── ...

crates/qf-plot/src/
├── superconductivity.rs        # 底层绘图原语（复用 phonon 样式）
└── ...
```

### 数据流水线
```
1. 检测：扫描 manifest.json，识别已完成的 attempt
2. 解析：读取对应 MgB2.* 文件，转换为 Rust 结构体
3. 归并：按温度分组，构建温度序列
4. 绘图：调用 qf-plot，生成五类图表
5. 镜像：保持相对路径，写入 results/ 对应层级
6. 格式：支持 SVG/PNG/PDF（根级 [plot].formats 控制）
```

## 关键设计决策

### 1. 触发时机
- **自动触发**：`epw_superconductivity` CLI 完成 Tc 求解后，检测到 `complete: true` 自动绘图
- **手动触发**：提供 `epw_superconductivity --plot-only` 参数，跳过计算直接绘图
- **增量更新**：检测 results/ 中已有图表的时间戳，仅重绘需要更新的图

### 2. 数据解析策略
- **容错**：缺失的数据文件跳过对应图表，记录警告但不中断
- **验证**：检查列数、温度标签一致性，拒绝明显损坏的文件
- **缓存**：解析后的数据结构序列化到临时 `.plot_cache.json`，避免重复读取大文件

### 3. 绘图样式
- **复用现有风格**：参照 `qf-plot/src/electronic.rs` 的线宽、颜色、字体
- **对标论文**：
  - 虚频能隙：黑色密集散点（无连线）
  - 温度演化：蓝色实线+点标记
  - 实频能隙：灰色半透明散点（数据量大）
  - Eliashberg：黑线（左轴）+ 红线（右轴）

### 4. 性能优化
- **并行绘图**：五类图表独立，使用 rayon 并行生成
- **大文件处理**：实频轴能隙（~70MB）采样后绘制，避免渲染百万点
- **增量输出**：先生成单张图，最后可选组合为 summary

## 实施步骤

### 阶段一：数据解析（2 天）
- [ ] 定义 Rust 结构体表示五类数据
- [ ] 实现 `MgB2.*` 文件解析器（支持多列格式）
- [ ] 编写单元测试（复用 ubuntu/run 的真实文件）

### 阶段二：核心绘图（3 天）
- [ ] 实现图 1：电声耦合三联图（复用 phonon 直方图）
- [ ] 实现图 2：虚频/实频能隙（散点图 + 采样优化）
- [ ] 实现图 3-4：温度演化（折线图 + 多曲线）
- [ ] 实现图 5：超导态密度（归一化处理）

### 阶段三：集成与镜像（1 天）
- [ ] 在 `epw_superconductivity` CLI 集成绘图入口
- [ ] 实现目录层级镜像逻辑
- [ ] 支持 `--plot-only` 和增量更新

### 阶段四：测试与优化（1 天）
- [ ] 在 MgB2 ubuntu 真实数据上验证
- [ ] 性能测试（70MB 文件 + 多温度点）
- [ ] 文档更新（README + native-stage-config.md）

## 风险与约束

### 已知风险
1. **实频轴文件过大**（~70MB）：采样策略可能影响细节
   - 缓解：固定采样密度（如每 100 点取 1 点），保留局部特征
   
2. **温度点缺失或不连续**：用户可能只算了部分温度
   - 缓解：容错设计，缺失温度点在图例中标注

3. **费米面 DOS 提取不稳定**：epw1.out 格式可能因版本变化
   - 缓解：正则匹配 + 多版本兼容，失败时使用默认值 1.0

### 技术约束
- Rust 绘图库限制：复杂图例需要手动布局
- 内存：70MB 文件需要流式解析或 mmap
- 并行 I/O：需要确保 results/ 目录创建的线程安全性

## 验收标准

1. **功能完整性**
   - [ ] 五类图表全部绘制成功
   - [ ] 目录层级与 8.epw-superconductivity 一致
   - [ ] 支持 SVG/PNG/PDF 多格式输出

2. **数据准确性**
   - [ ] 图 1 与 Mori Fig. 6 视觉对照吻合
   - [ ] 温度演化曲线单调性与物理预期一致
   - [ ] 超导态密度归一化正确（N_F 读取准确）

3. **性能要求**
   - [ ] MgB2 ubuntu 全数据（10+ 温度点）绘图 < 30 秒
   - [ ] 70MB 实频文件不触发 OOM

4. **用户体验**
   - [ ] `--plot-only` 无需重算即可重绘
   - [ ] 缺失文件不中断整体流程
   - [ ] 图表文件名清晰、易识别

## 参考资料

- 论文：[Phys. Rev. B 87, 024505 (2013)](https://journals.aps.org/prb/abstract/10.1103/PhysRevB.87.024505)
- Tutorial PDF：`/home/mayuan/code/dftflow/examples/MgB2/Wed.9.Mori.pdf`
- 真实数据：`/home/mayuan/code/dftflow/examples/MgB2/ubuntu/run/8.epw-superconductivity/`
- 现有绘图模块：
  - `crates/qf-plot/src/electronic.rs`（能带对比）
  - `epwflow/src/delivery/native_plot.rs`（声子三类图）

## 后续扩展

### 短期（与本需求无关，不在本 Issue 范围）
- 交互式 HTML 版本（使用 plotly.js）
- 各向异性能隙的 3D 费米面投影

### 长期
- 与 Jupyter Notebook 集成
- 自动生成 LaTeX 论文插图代码

## 2026-11-18 回测说明

实际实现、原生格式补充及待验收边界见 [acceptance.md](acceptance.md) 与 [tasks.md](tasks.md)。本规划中的解析缓存、按时间戳增量出图和可选汇总图尚未实现，不表示当前行为；本轮仅复用已有科学输出重绘，不补算缺失数据。

## 方案 B：完整五类图实施计划（2026-11-19）

### 当前状态诊断

**✅ 已完成**：
- 绘图功能完整实现（36 项专项回归通过）
- 图 1、3、4 的数据已存在（λ 分布、虚频能隙温度演化）

**❌ 待完成**：
- 图 2 实频能隙缺失（需要 Padé 延拓数据）
- 图 5 超导态密度缺失（需要 qdos 数据，可选）
- 109 项测试失败（Issue #51 日志格式变更的遗留问题，与绘图功能无关）

### 补充计算方案

#### 阶段一：数据补充（预计 1 小时）

**步骤**：
1. 修改 `input.toml` 添加 `lpade = true` 和 `npade = 50`（iso 和 aniso 两个部分）
2. 清理旧数据：`rm -rf run/8.epw-superconductivity`
3. 重新提交：`epw_superconductivity --stage superconductivity -c input.toml -s`
4. 等待完成（约 15-30 分钟）
5. 验证数据：检查 `MgB2.pade_aniso_*.00` 文件

#### 阶段二：绘图验证（预计 30 分钟）

**步骤**：
1. 自动绘图或手动触发：`epw_superconductivity --plot-only -c input.toml`
2. 检查五类图文件
3. 视觉对比 Mori PDF（Fig. 6, 7, 8, 2, 9）
4. 数值验证（累计 λ、最大能隙、Tc）

#### 阶段三：测试修复（预计 2-3 小时）

**步骤**：
1. 更新测试夹具 `direct_submit.rs` 为 schema_version=3（合并日志格式）
2. 逐批验证修复
3. 确认 109 项失败降至 0

#### 阶段四：文档与提交（预计 1 小时）

**步骤**：
1. 更新 `acceptance.md` 记录完整五类图验收结果
2. 更新 `README.md` 和 `native-stage-config.md`
3. 更新 `tasks.md` 标记任务完成
4. 提交代码并关闭 Issue #76

### 时间线

**总计**：约 4.5-5.5 小时（1 个工作日内完成）

### 参考文档

- **完整操作手册**：[operation-manual.md](operation-manual.md)
- **Mori PDF**：`examples/MgB2/Wed.9.Mori.pdf` 第 23-27 页
- **EPW 参数说明**：`lpade`、`npade`、`qdos_k` 的详细说明
