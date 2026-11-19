# Issue #76 方案 B 更新汇总

**更新时间**：2026-11-19  
**更新范围**：将方案 B（完整五类图补充计算）整合到 #76 的所有文档

---

## 已更新的文档

### 1. ✅ spec.md

**位置**：`docs/specs/0076-epw-superconductivity-plot-suite/spec.md`

**更新内容**：
- 添加了 **第 14 节：方案 B：完整五类图补充计算（2026-11-19）**
- 包含内容：
  - 14.1 当前数据状态盘点（已有数据 vs 缺失数据）
  - 14.2 补充计算参数说明（lpade、npade、qdos_k）
  - 14.3 具体操作步骤（6 个步骤）
  - 14.4 科学验证标准
  - 14.5 后续任务清单
  - 14.6 常见问题（Q1-Q3）

**关键亮点**：
- 明确说明了图 1、3、4 的数据已存在于第 7 步和第 8 步
- 只需要补充图 2（Padé 实频）和图 5（qdos）的数据
- 提供了清晰的配置修改示例和命令行操作步骤

### 2. ✅ tasks.md

**位置**：`docs/specs/0076-epw-superconductivity-plot-suite/tasks.md`

**更新内容**：
- 添加了 **阶段五：补充 Padé 和 qdos 数据**（T028-T034）
- 添加了 **阶段六：测试基础设施修复**（T035-T038）
- 添加了 **阶段七：最终验收与提交**（T039-T044）

**新增任务清单**：
```
T028: 修改 input.toml 添加参数
T029: 清理并重新提交第 8 步计算
T030: 验证 Padé 实频能隙文件
T031: 验证 qdos 文件（可选）
T032: 触发绘图并验证图 2
T033: 视觉对比 Mori PDF
T034: 定量验证关键数值
T035-T038: 测试夹具修复
T039-T044: 最终验收与提交
```

### 3. ✅ plan.md

**位置**：`docs/specs/0076-epw-superconductivity-plot-suite/plan.md`

**更新内容**：
- 添加了 **方案 B：完整五类图实施计划（2026-11-19）**
- 包含内容：
  - 当前状态诊断
  - 补充计算方案（四个阶段）
  - 时间线（总计 4.5-5.5 小时）
  - 参考文档链接

**关键信息**：
- 明确了四个阶段的预计时间
- 提供了清晰的步骤分解
- 链接到完整操作手册

### 4. ✅ operation-manual.md

**位置**：`docs/specs/0076-epw-superconductivity-plot-suite/operation-manual.md`

**创建时间**：2026-11-19（新文件）

**包含内容**：
- 完整的操作手册（九个章节）
- 详细的命令行示例
- 问题排查指南
- 验收清单
- 对比验证方法

---

## 核心修改：需要在 input.toml 中添加的参数

### 位置 1：[stages."8.epw-superconductivity".iso]（约第 321 行）

```toml
lpade = true          # 从 false 改为 true
npade = 50            # 新增这一行
```

### 位置 2：[stages."8.epw-superconductivity".aniso]（约第 360 行）

```toml
lpade = true          # 从 false 改为 true
npade = 50            # 新增这一行
```

### 图 5 不需要额外开关：

```toml
# 当前 EPW 5.9 在 lpade 成功后自动生成 qdos_*
```

---

## 下一步操作

### 立即开始的命令

```bash
cd /home/mayuan/code/dftflow/examples/MgB2/ubuntu

# 1. 编辑配置文件
vim input.toml  # 或 code input.toml

# 2. 修改两处 lpade 和 npade（见上方）

# 3. 清理旧数据
rm -rf run/8.epw-superconductivity

# 4. 重新提交计算
epw_superconductivity --stage superconductivity -c input.toml -s

# 5. 监控作业（约 15-30 分钟）
squeue -u $USER

# 6. 计算完成后验证
ls -lh run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/MgB2.pade_aniso_*

# 7. 手动触发绘图（如果需要）
epw_superconductivity --plot-only -c input.toml

# 8. 查看生成的图表
ls -lh results/8.epw-superconductivity/*/*/fig*.{svg,png,pdf}
```

---

## 文档位置索引

| 文档 | 路径 | 用途 |
|------|------|------|
| **完整操作手册** | [operation-manual.md](operation-manual.md) | 详细的逐步指南 |
| **需求规格** | [spec.md](spec.md) | 完整需求定义 + 方案 B 说明 |
| **任务清单** | [tasks.md](tasks.md) | 技术任务分解 |
| **实施计划** | [plan.md](plan.md) | 时间线和风险管理 |
| **验收记录** | [acceptance.md](acceptance.md) | 待更新（计算完成后） |
| **契约定义** | [contracts.md](contracts.md) | 数据格式契约 |

---

## 关键理解

### 为什么不需要备份数据？

你说得对，不需要备份。原因：
1. **第 7 步的 λ 分布数据不会被覆盖**：它们在 `run/7.epw-fine/` 目录中
2. **第 8 步重新计算是安全的**：
   - 只是添加了 `lpade` 参数，科学结果不会改变
   - 虚频能隙数据会重新生成（与之前相同）
   - 只是额外生成了 Padé 实频数据

### 为什么只需要修改两处参数？

因为：
1. **λ 分布数据已存在**：第 7 步（epw-fine）已经计算了 `lambda_pairs` 和 `lambda_k_pairs`
2. **虚频能隙数据已存在**：第 8 步已经计算了 `imag_aniso_gap0` 和 `imag_iso`
3. **只缺少实频能隙**：需要 `lpade = true` 启用 Padé 解析延拓

### 为什么是方案 B？

- **方案 A**：只画图 3 和图 4（你最初的核心需求：虚轴能隙温度演化）
- **方案 B**：完成全部五类图（对标 Mori Tutorial 的完整图集）

你选择了方案 B，所以我们要补充 Padé 和 qdos 数据，完成全部五类图。

---

## 预期时间线

| 步骤 | 预计时间 | 说明 |
|------|----------|------|
| 修改配置 | 5 分钟 | 编辑 input.toml 两处参数 |
| 清理数据 | 1 分钟 | `rm -rf run/8.epw-superconductivity` |
| 提交计算 | 1 分钟 | 运行 `epw_superconductivity -s` |
| **等待计算** | **15-30 分钟** | **Slurm 作业运行（iso + aniso）** |
| 验证数据 | 5 分钟 | 检查 Padé 文件 |
| 绘图 | 1 分钟 | 自动触发或手动执行 |
| 视觉验证 | 10 分钟 | 对比 Mori PDF |
| **总计** | **约 30-45 分钟** | **实际操作时间** |

---

## 参考论文

- **Mori Tutorial**：`/home/mayuan/code/dftflow/examples/MgB2/Wed.9.Mori.pdf`
  - 第 23-25 页：五类图的详细说明
  - Fig. 6：电声耦合三联图
  - Fig. 7：虚频/实频能隙对比
  - Fig. 8：各向异性能隙温度演化（MgB₂）
  - Fig. 2：各向同性能隙温度演化（Pb）
  - Fig. 9：超导态密度

---

**更新完成**。所有文档已同步方案 B 的内容，你可以立即开始操作。

## 2026-11-19 执行修订

- input.toml 只需在 iso/aniso 两处设置 lpade = true、npade = 50。
- 当前 EPW 5.9 不使用 qdos_k 配置；Padé 成功后自动写出 qdos，qdos 第二列已经是 N_S/N_F。
- 绘图结果写入 examples/MgB2/ubuntu/results/8.epw-superconductivity/。
