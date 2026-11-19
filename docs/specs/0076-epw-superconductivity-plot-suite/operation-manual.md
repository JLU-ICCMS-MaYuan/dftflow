# 方案 B：完整五类超导能隙图操作手册

**目标**：生成全部五类超导能隙图，对标 Mori Tutorial (PRB 87, 024505)

**日期**：2026-11-19  
**关联文档**：[spec.md](spec.md) | [acceptance.md](acceptance.md)

---

## 一、当前数据状态盘点

### ✅ 已有数据（无需重新计算）

1. **图 1 电声耦合三联图数据**（第 7 步已生成）：
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

### ❌ 缺失数据（需要重新计算第 8 步）

1. **图 2 实频能隙**（需要 Padé 解析延拓）：
   - `MgB2.pade_aniso_020.00` - 20K 实频能隙
   - `MgB2.pade_aniso_030.00` - 30K 实频能隙
   - `MgB2.pade_aniso_040.00` - 40K 实频能隙
   - `MgB2.pade_iso_020.00` - 20K 各向同性实频能隙（可选）

2. **图 5 超导态密度**：
   - `MgB2.qdos_020` - 20K 超导态密度
   - `MgB2.qdos_030` - 30K 超导态密度
   - `MgB2.qdos_040` - 40K 超导态密度

---

## 二、配置修改步骤

### 步骤 1：备份当前配置和数据

```bash
cd /home/mayuan/code/dftflow/examples/MgB2/ubuntu

# 备份第 8 步的计算结果
cp -r run/8.epw-superconductivity run/8.epw-superconductivity.backup-before-pade

# 备份当前配置
cp input.toml input.toml.backup-before-pade

# 验证备份
ls -lh run/8.epw-superconductivity.backup-before-pade/
```

### 步骤 2：修改 input.toml 添加 Padé 和 qdos 参数

根据 **Mori PDF 第 23-27 页**的说明，需要在第 8 步的 iso 和 aniso 配置中添加以下参数：

#### 2.1 修改各向同性（iso）配置

在 `[stages."8.epw-superconductivity".iso]` 部分（约第 306-343 行）：

**需要修改的参数**：
```toml
lpade = true          # 从 false 改为 true，启用 Padé 解析延拓
npade = 50            # 新增，使用 50% 的 Matsubara 点进行 Padé 延拓
```

**超导态密度说明**：
```toml
# 在 [stages."8.epw-superconductivity".iso] 的末尾添加
当前 EPW 5.9 会自动生成 qdos_*，不添加额外开关
```

#### 2.2 修改各向异性（aniso）配置

在 `[stages."8.epw-superconductivity".aniso]` 部分（约第 345-383 行）：

**需要修改的参数**：
```toml
lpade = true          # 从 false 改为 true，启用 Padé 解析延拓
npade = 50            # 新增，使用 50% 的 Matsubara 点进行 Padé 延拓
```

**超导态密度说明**：
```toml
# 在 [stages."8.epw-superconductivity".aniso] 的末尾添加
当前 EPW 5.9 会自动生成 qdos_*，不添加额外开关
```

#### 2.3 完整修改示例

使用以下命令进行修改：

```bash
cd /home/mayuan/code/dftflow/examples/MgB2/ubuntu

# 方法 1：使用 sed 自动修改
sed -i 's/^lpade = false/lpade = true/' input.toml
sed -i '/^\[stages\."8\.epw-superconductivity"\.iso\]/,/^\[stages\."8\.epw-superconductivity"\.aniso\]/ s/^lpade = false/lpade = true\nnpade = 50/' input.toml
sed -i '/^\[stages\."8\.epw-superconductivity"\.aniso\]/,/^$/ s/^lpade = false/lpade = true\nnpade = 50/' input.toml

# 方法 2：手动编辑（推荐，更安全）
vim input.toml
# 或
code input.toml
```

**手动编辑时的具体位置**：

```toml
# 约在第 321 行，iso 部分
[stages."8.epw-superconductivity".iso]
# ... 其他参数 ...
limag = true
lreal = false
lpade = true          # ← 将 false 改为 true
npade = 50            # ← 新增这一行
lacon = false
# ... 其他参数 ...

# 约在第 360 行，aniso 部分
[stages."8.epw-superconductivity".aniso]
# ... 其他参数 ...
limag = true
lreal = false
lpade = true          # ← 将 false 改为 true
npade = 50            # ← 新增这一行
lacon = false
# ... 其他参数 ...
```

### 步骤 3：验证配置修改

```bash
# 检查修改后的配置
grep -A 5 "lpade" input.toml

# 应该看到：
# lpade = true
# npade = 50

# 验证配置文件的完整性
epw_superconductivity check -c input.toml
```

---

## 三、重新运行第 8 步超导计算

### 步骤 4：清理旧的第 8 步数据

**重要**：由于修改了计算参数，需要清理旧的第 8 步计算结果，否则会因为输入变化而被拒绝。

```bash
cd /home/mayuan/code/dftflow/examples/MgB2/ubuntu

# 删除旧的第 8 步工作目录（已备份）
rm -rf run/8.epw-superconductivity

# 删除旧的第 8 步结果目录（如果存在）
rm -rf results/8.epw-superconductivity
```

### 步骤 5：重新提交第 8 步超导计算

```bash
cd /home/mayuan/code/dftflow/examples/MgB2/ubuntu

# 提交第 8 步超导计算
epw_superconductivity --stage superconductivity -c input.toml -s
```

**预期输出**：
```
Submitting superconductivity stage...
Submitted batch job 1234567 (iso)
Submitted batch job 1234568 (aniso)
```

### 步骤 6：监控作业状态

```bash
# 查看 Slurm 作业状态
squeue -u $USER

# 查看作业日志
tail -f run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/iso-mu0.1-Tc20-40-degaussq0.5meV/slurm-epw.log
tail -f run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/slurm-epw.log
```

### 步骤 7：等待计算完成并验证数据

**预计运行时间**：
- **各向同性（iso）**：约 5-10 分钟（12 MPI 进程）
- **各向异性（aniso）**：约 15-30 分钟（12 MPI 进程）

**计算完成后，验证新生成的数据文件**：

```bash
cd /home/mayuan/code/dftflow/examples/MgB2/ubuntu

# 检查 Padé 实频能隙文件
ls -lh run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/MgB2.pade_aniso_*
ls -lh run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/iso-mu0.1-Tc20-40-degaussq0.5meV/MgB2.pade_iso_*

# 应该看到：
# MgB2.pade_aniso_020.00  - 20K 各向异性实频能隙
# MgB2.pade_aniso_030.00  - 30K 各向异性实频能隙
# MgB2.pade_aniso_040.00  - 40K 各向异性实频能隙
# MgB2.pade_iso_020.00    - 20K 各向同性实频能隙
# MgB2.pade_iso_030.00    - 30K 各向同性实频能隙
# MgB2.pade_iso_040.00    - 40K 各向同性实频能隙

# 检查 qdos 文件（如果已有 qdos 数据）
ls -lh run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/*/MgB2.qdos_*

# 查看生成的文件格式
head -20 run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/MgB2.pade_aniso_020.00
```

**预期文件格式**（Padé）：
```
# Pade aniso T = 20.00 K
# ω(eV)  ε_nk(eV)  Re[Z(ω)]  Im[Z(ω)]  Re[Δ(ω)](eV)  Im[Δ(ω)](eV)
-1.0000  0.1234    0.9876    0.0123    0.0012        0.0001
-0.9900  0.1234    0.9875    0.0124    0.0013        0.0001
...
```

---

## 四、自动绘图和手动验证

### 步骤 8：自动绘图（计算完成后自动触发）

如果第 8 步计算成功完成，绘图会自动触发：

```bash
# 检查自动绘图结果
ls -lh results/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/

# 应该看到五类图（SVG/PNG/PDF）：
# fig1_coupling_strength.*           - 电声耦合三联图
# fig2_gap_imaginary_real.*          - 虚频/实频能隙双联图
# fig3_aniso_gap_vs_T.*              - 各向异性能隙温度演化
# fig4_iso_gap_vs_T.*                - 各向同性能隙温度演化
# fig5_superconducting_dos.*         - 超导态密度（如果有 qdos 数据）
```

### 步骤 9：手动触发绘图（如果需要）

如果自动绘图失败或需要重新绘图：

```bash
cd /home/mayuan/code/dftflow/examples/MgB2/ubuntu

# 只读模式重新绘图（不重新计算）
epw_superconductivity --plot-only -c input.toml

# 检查绘图日志
epw_superconductivity --plot-only -c input.toml 2>&1 | tee plot.log

# 查看绘图结果
ls -lh results/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/*/
```

### 步骤 10：验证五类图的数据完整性

```bash
cd /home/mayuan/code/dftflow/examples/MgB2/ubuntu

# 检查每类图的数据来源
for dir in results/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/*/; do
    echo "=== $dir ==="
    ls -lh "$dir"/*.svg 2>/dev/null || echo "No SVG files"
    ls -lh "$dir"/*.png 2>/dev/null || echo "No PNG files"
    echo ""
done

# 查看绘图报告（JSON 格式）
cat results/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/*.plot.json | jq
```

---

## 五、数据对比和科学验证

### 步骤 11：对比 Mori PDF 参考图

使用图像查看器打开生成的图表，与 Mori PDF 进行对比：

```bash
cd /home/mayuan/code/dftflow/examples/MgB2/ubuntu

# 打开参考 PDF
evince ../../../examples/MgB2/Wed.9.Mori.pdf &

# 打开生成的图表
eog results/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/fig*.png &
```

**对比检查清单**：

1. **图 1 电声耦合三联图**（对比 Mori Fig. 6）：
   - [ ] 左图：λ_{nk,mk+q} 分布形状是否合理
   - [ ] 中图：λ_k 费米面分布是否有明显峰值
   - [ ] 右图：α²F(ω) 是否有两个主峰（对应 MgB₂ 的 E₂g 和 B₁g 模式）
   - [ ] 右图：累计 λ(ω) 是否单调递增

2. **图 2 虚频/实频能隙双联图**（对比 Mori Fig. 7）：
   - [ ] 左图：虚频能隙 Δ(iω) 是否随频率单调递减
   - [ ] 右图：实频能隙 Re[Δ(ω)] 是否在费米面附近有明显结构
   - [ ] 右图：Im[Δ(ω)] 是否在能隙边缘有峰值

3. **图 3 各向异性能隙温度演化**（对比 Mori Fig. 8）：
   - [ ] 是否有多条曲线（对应不同 k 点）
   - [ ] 能隙是否随温度升高而减小
   - [ ] 是否有两个明显的能隙值群（对应 σ 和 π 带）
   - [ ] 20K 时最大能隙约 6-7 meV

4. **图 4 各向同性能隙温度演化**（对比 Mori Fig. 2，Pb 的图）：
   - [ ] 能隙是否随温度升高而单调减小
   - [ ] 20K、30K、40K 三个温度点是否清晰可见
   - [ ] 能隙数值是否在合理范围（2-5 meV）

5. **图 5 超导态密度**（对比 Mori Fig. 9）：
   - [ ] N_S(ω)/N_F 在 ω=0 附近是否为零（能隙特征）
   - [ ] 在能隙边缘是否有 BCS 型相干峰
   - [ ] 高能部分是否趋近于 1（正常态）

### 步骤 12：提取关键数值进行定量对比

```bash
cd /home/mayuan/code/dftflow/examples/MgB2/ubuntu

# 提取 20K 时的最大能隙值
grep "020.00" run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/MgB2.imag_aniso_gap0_020.00 | \
  awk '{print $2}' | sort -g | tail -5

# 提取累计 λ 的最终值
tail -1 run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/source.a2f | \
  awk '{print "Total λ =", $12}'
```

**预期数值范围**（与 Mori 对比）：
- **累计 λ**：约 0.6-0.8（MgB₂）
- **最大能隙（20K，各向异性）**：约 6-7 meV（π 带）
- **最小能隙（20K，各向异性）**：约 2-3 meV（σ 带）
- **各向同性能隙（20K）**：约 3-4 meV

---

## 六、问题排查指南

### 常见问题 1：Padé 延拓失败

**症状**：`MgB2.pade_aniso_*.00` 文件未生成，或 `epw.out` 中出现错误：
```
Error in subroutine pade:
Not enough Matsubara points for Pade continuation
```

**解决方法**：
1. 增加 `nqstep` 参数（Matsubara 频率点数）：
   ```toml
   nqstep = 1000  # 从 500 增加到 1000
   ```
2. 减小 `npade` 参数：
   ```toml
   npade = 30     # 从 50 减小到 30
   ```

### 常见问题 2：qdos 文件未生成

**症状**：计算完成但没有 `MgB2.qdos_*` 文件

**可能原因**：
1. 未添加额外 qdos 开关
2. EPW 版本不支持 qdos 输出（需要 EPW 5.8+）

**解决方法**：
```bash
# 检查 EPW 版本
grep "Program EPW" run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/epw.out

# 如果版本 < 5.8，考虑升级或跳过图 5
```

### 常见问题 3：绘图失败但科学计算成功

**症状**：`epw.out` 显示计算成功，但绘图报错

**排查步骤**：
```bash
# 查看绘图警告
epw_superconductivity --plot-only -c input.toml 2>&1 | grep -i "warning\|error"

# 检查缺失的数据文件
find run/8.epw-superconductivity -name "MgB2.*" -type f | sort

# 手动验证数据文件格式
head -5 run/8.epw-superconductivity/01.k8x8x4-q4x4x4-fsthick0.4eV-degaussw0.1eV/aniso-mu0.1-Tc20-40-degaussq0.5meV/MgB2.pade_aniso_020.00
```

### 常见问题 4：作业因资源不足而失败

**症状**：Slurm 作业状态为 `FAILED`，日志显示内存或时间超限

**解决方法**：
```bash
# 增加 Slurm 资源请求
vim input.toml

# 修改 [slurm] 部分
[slurm]
header = """
#SBATCH --partition=local
#SBATCH --nodes=1
#SBATCH --ntasks=12
#SBATCH --mem=32G           # 增加内存限制
#SBATCH --time=02:00:00     # 增加时间限制
"""
```

---

## 七、完整验收清单

### 数据文件验收

- [ ] **第 7 步 λ 分布数据**（已有）：
  - [ ] `MgB2.lambda_pairs` 存在且文件大小 > 10 KB
  - [ ] `MgB2.lambda_k_pairs` 存在且文件大小 > 5 KB
  - [ ] `source.a2f` 存在于每个方法目录

- [ ] **第 8 步虚频能隙数据**（已有）：
  - [ ] `MgB2.imag_aniso_gap0_020.00/030.00/040.00` 全部存在
  - [ ] `MgB2.imag_iso_020.00/030.00/040.00` 全部存在

- [ ] **第 8 步实频能隙数据**（新增）：
  - [ ] `MgB2.pade_aniso_020.00/030.00/040.00` 全部存在
  - [ ] 每个文件包含 6 列数据（ω, ε_nk, Re[Z], Im[Z], Re[Δ], Im[Δ]）
  - [ ] 文件大小合理（约 1-10 MB，取决于采样点数）

- [ ] **第 8 步超导态密度数据**（可选新增）：
  - [ ] `MgB2.qdos_020/030/040` 存在（EPW 5.9 在 Padé 成功后自动生成）
  - [ ] 每个文件包含 2 列数据（ω, N_S(ω)/N_F）

### 绘图结果验收

- [ ] **图 1 电声耦合三联图**：
  - [ ] SVG/PNG/PDF 三格式全部生成
  - [ ] 三个面板（λ 分布、λ_k、α²F）全部正常显示
  - [ ] 没有数据缺失或空白面板

- [ ] **图 2 虚频/实频能隙双联图**：
  - [ ] SVG/PNG/PDF 三格式全部生成
  - [ ] 左右两个面板（虚频、实频）全部正常显示
  - [ ] 实频面板不再为空（之前缺失 Padé 数据）

- [ ] **图 3 各向异性能隙温度演化**：
  - [ ] SVG/PNG/PDF 三格式全部生成
  - [ ] 三个温度点（20K、30K、40K）全部显示
  - [ ] 多条 k 点曲线清晰可见

- [ ] **图 4 各向同性能隙温度演化**：
  - [ ] SVG/PNG/PDF 三格式全部生成
  - [ ] 三个温度点（20K、30K、40K）全部显示
  - [ ] 温度演化趋势合理

- [ ] **图 5 超导态密度**：
  - [ ] SVG/PNG/PDF 三格式全部生成（如果有 qdos 数据）
  - [ ] 能隙特征（ω=0 附近为零）清晰可见
  - [ ] 相干峰位置合理

### 科学验证

- [ ] 五类图的数值范围与 Mori PDF 基本一致
- [ ] 能隙温度演化趋势物理合理（随温度升高而减小）
- [ ] λ 分布和 α²F 形状符合 MgB₂ 的已知特征
- [ ] 超导态密度显示明显的 BCS 型相干峰

---

## 八、后续工作：修复 109 项测试失败

完成五类图的生成和验证后，需要修复测试基础设施问题。详见：

- [测试修复方案](test-fixture-update-plan.md)（待创建）
- 根本原因：Issue #51 日志格式变更后，测试夹具未同步更新
- 修复方向：更新测试夹具为 schema_version=3 的合并日志格式

---

## 九、参考资料

1. **Mori Tutorial PDF**：`examples/MgB2/Wed.9.Mori.pdf`
   - 第 23-25 页：五类图的详细说明
   - 第 6、7、8、2、9 图：对应的参考图表

2. **EPW 官方文档**：
   - `lpade` 参数：Padé 解析延拓开关
   - `npade` 参数：Matsubara 点采样比例（%）
   - EPW 5.9 的 qdos 自动输出：第二列为 `N_S/N_F`

3. **相关 Issue 和 Spec**：
   - Issue #76：EPW 超导计算自动绘图
   - [spec.md](spec.md)：完整需求定义
   - [acceptance.md](acceptance.md)：验收记录

---

**手册版本**：v1.0  
**最后更新**：2026-11-19  
**维护者**：Claude (Opus 5.5)

## 十、2026-11-19 实际执行结果与格式修订

本次已按两处配置修改执行：iso 和 aniso 均设置 lpade = true、npade = 50；没有添加 qdos_k。EPW 5.9 源码中的 supercond_iso.f90 和 supercond_aniso.f90 在 Padé 成功后都会调用 dos_quasiparticle，实际生成 MgB2.qdos_020.00、030.00、040.00。

实际结果目录是配置目录下的 results/8.epw-superconductivity/，不是 run/results/。aniso 方法目录生成五类图各 SVG/PNG/PDF 一份，共 15 个文件；iso 方法目录生成其可用的图 1、3、4、5，共 12 个文件。图 2 使用 Mori Fig. 7 对应的各向异性虚频与 Padé 数据，iso 目录没有各向异性输入时留空属于预期。

EPW qdos 首行是未注释的 w [eV] N_S/N_F，第二列已经是归一化值；绘图解析器会跳过该列头并避免再次除以费米面 DOS。原生 imag_aniso_gap0_* 和 pade_aniso_gap0_* 是分布／辅助文件，不会被误当作逐状态温度曲线或 Padé 主文件。
