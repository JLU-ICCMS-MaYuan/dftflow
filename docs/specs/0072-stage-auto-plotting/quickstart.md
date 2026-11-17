# 快速验证：各阶段计算完成后自动绘图

## 前提条件

- Rust/Cargo 环境就绪
- epwflow 已构建（`cargo build --release`）
- 有效的测试目录（例如 MgB₂）
- 输入配置文件 `input.toml` 准备好

## 验证步骤

### 1. SCF 阶段自动绘图

```bash
cd /home/mayuan/code/dftflow/test_auto/MgB2/local_run
epw_phonon -c input.toml --stage scf -s

# 验证图片存在
ls -la scf/plots/scf-band-ranges.png
```

**预期结果**：
- SCF 计算成功完成
- `scf/plots/scf-band-ranges.png` 文件存在
- 图片尺寸合理（至少 300 DPI）

### 2. NSCF 阶段自动绘图

```bash
epw_phonon -c input.toml --stage nscf -s

# 验证图片存在
ls -la nscf/plots/nscf-band-ranges.png
```

**预期结果**：
- NSCF 计算成功完成
- `nscf/plots/nscf-band-ranges.png` 文件存在

### 3. Electronic 阶段自动绘图

```bash
epw_phonon -c input.toml --stage electronic -s

# 验证所有图片
ls -la 3.qe-electronic/*.png
```

**预期结果**：
- Electronic 计算成功完成
- 以下图片存在：
  - `electronic-band-ranges.png`
  - `electronic_band.png`
  - `electronic_dos.png`
  - `electronic_pdos.png`

### 4. Phonon 阶段自动绘图

```bash
epw_phonon -c input.toml --stage phonon -s

# 验证图片
ls -la phonon/plots/*.png
```

**预期结果**：
- Phonon 计算成功完成
- 以下图片存在：
  - `phonon_band.png`
  - `phonon_dos.png`

### 5. Wannier 阶段自动绘图

```bash
epw_wannier -c input.toml --stage wannier -s

# 验证对比图
ls -la 5.epw-wannier/*.png
```

**预期结果**：
- Wannier 计算成功完成
- 四类对比图存在：
  - 电子能带对比图
  - 电子 DOS 对比图
  - 声子能带对比图
  - 声子 DOS 对比图

### 6. Coarse 阶段自动绘图

```bash
epw_wannier -c input.toml --stage coarse -s

# 验证对比图和衰减图
ls -la 6.epw-coarse/*.png
```

**预期结果**：
- Coarse 计算成功完成
- 四类对比图 + 实空间衰减图存在

## 绘图失败测试

### 模拟绘图失败

```bash
# 删除绘图依赖文件
rm -f nscf/out/*.xml

# 执行计算
epw_phonon -c input.toml --stage nscf -s

# 检查状态
cat readable-state.json | jq '.nscf."Plot · NSCF"'
```

**预期结果**：
- NSCF 计算成功完成（科学数据标记为完成）
- 绘图失败，readable_state 中记录失败原因
- 不阻断后续阶段

### 使用 --plot 重新绘图

```bash
# 恢复依赖文件
# ...

# 显式重绘
epw_phonon -c input.toml -w <work_dir> --plot

# 验证图片
ls -la nscf/plots/nscf-band-ranges.png
```

**预期结果**：
- 绘图成功
- 图片文件存在
- 不触发 NSCF 重新计算

## 复用结果补图测试

```bash
# 第一次执行
epw_phonon -c input.toml --stage scf -s

# 删除图片
rm -f scf/plots/scf-band-ranges.png

# 执行下一阶段（复用 SCF）
epw_phonon -c input.toml --stage nscf -s

# 检查 SCF 图片
ls -la scf/plots/scf-band-ranges.png
```

**预期结果**：
- NSCF 复用 SCF 结果（不重新计算 SCF）
- SCF 图片自动补齐
- SCF 科学数据保持不变

## 完整流程验证

```bash
# 清空工作目录
rm -rf /tmp/mgb2_test
mkdir -p /tmp/mgb2_test
cd /tmp/mgb2_test
cp /path/to/input.toml .
cp /path/to/POSCAR .

# 执行完整流程
epw_phonon -c input.toml --stage scf -s
epw_phonon -c input.toml --stage nscf -s
epw_phonon -c input.toml --stage electronic -s
epw_phonon -c input.toml --stage phonon -s
epw_wannier -c input.toml --stage wannier -s
epw_wannier -c input.toml --stage coarse -s

# 统计图片数量
find . -name "*.png" -type f | wc -l
```

**预期结果**：
- 所有阶段成功完成
- 所有图片自动生成
- 图片总数符合预期（至少 15 张）

## 性能测试

### 测量绘图耗时

```bash
time epw_phonon -c input.toml --stage scf -s
```

**预期结果**：
- 绘图增加的时间 < 10% 总时间
- 用户体验改善（无需手动绘图）

## 回归测试

### 验证 --plot 选项保持兼容

```bash
# 不使用自动绘图（假设临时禁用）
# ...

# 手动调用 --plot
epw_phonon -c input.toml -w <work_dir> --plot
```

**预期结果**：
- `--plot` 选项功能不变
- 图片正常生成

## 失败场景

如果验证失败，检查：
1. readable_state.json 中的失败原因
2. 绘图依赖文件是否存在
3. 绘图函数是否正确调用
4. 异常是否被正确捕获

## 验收输出

按 tasks.md 覆盖全部 FR/SC，实际证据存 `docs/specs/0072-stage-auto-plotting/acceptance.md`。先记录未运行，不能通过脚本存在或帮助输出完成数值验收。
