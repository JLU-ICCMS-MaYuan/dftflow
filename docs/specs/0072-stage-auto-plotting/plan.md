# 实施计划：各阶段计算完成后自动绘图

## 技术栈与工具链

- Rust 2021 edition，rustc 1.99.0
- 复用现有 Cargo workspace 的 epwflow crate
- 绘图复用 `plotters/svg2pdf`（已有）
- 无新增第三方依赖

## 架构分层与职责

### 1. 阶段执行层（workspace_science.rs）

**职责**：各阶段科学计算完成后，调用绘图函数

**修改点**：
- `workspace_science::run(cfg, step)` 函数
- 在每个阶段（SCF/NSCF/Electronic/Phonon/Wannier/Coarse）完成后，调用对应的绘图函数
- 捕获绘图异常，失败时记录到 readable_state，不影响科学完成状态

### 2. QE 绘图层（workspace_qe_plot.rs）

**职责**：QE 阶段（SCF/NSCF/Electronic/Phonon）的绘图功能

**现状**：
- 已有 `workspace_qe_plot::refresh(cfg, step)` 函数
- 已有 `workspace_qe_plot::electronic(cfg, leaf)` 处理 Electronic 绘图
- 已有能带范围图生成（`band_ranges::generate`）

**修改点**：
- 确保 `refresh` 函数覆盖所有 QE 阶段
- 补充 Phonon 阶段的绘图调用（声子能带图 + 声子 DOS 图）

### 3. Wannier/Coarse 绘图层（workspace_science.rs + delivery/）

**职责**：Wannier 和 Coarse 阶段的对比图和衰减图

**现状**：
- `workspace_science::refresh_post_inner` 已有绘图逻辑
- `delivery/comparison_plot.rs` 处理对比图
- `delivery/decay.rs` 处理实空间衰减图

**修改点**：
- 确保 Wannier/Coarse 阶段完成后立即调用 `refresh_post_inner`
- 验证四类对比图和衰减图的自动生成

## 执行顺序

### 阶段 1：SCF 自动绘图

```rust
// workspace_science.rs:377-379
Step::Scf => {
    qe_data::scf::run_scf(&qe, root, &cfg.programs.launcher, Backend::Local)?;
    // 新增：立即画 SCF 能带范围图
    if let Err(e) = super::workspace_qe_plot::refresh(cfg, step) {
        eprintln!("Warning: SCF band ranges plotting failed: {:#}", e);
        super::readable_state::update(root, step, |d| {
            d.set("Plot · SCF", "Status", "failed");
            d.set("Plot · SCF", "Failure", format!("{:#}", e));
            Ok(())
        })?;
    }
}
```

### 阶段 2：NSCF 自动绘图

```rust
// workspace_science.rs:380-382
Step::Nscf => {
    qe_data::nscf::run_nscf(&qe, root, &cfg.programs.launcher, Backend::Local)?;
    // 新增：立即画 NSCF 能带范围图
    if let Err(e) = super::workspace_qe_plot::refresh(cfg, step) {
        eprintln!("Warning: NSCF band ranges plotting failed: {:#}", e);
        super::readable_state::update(root, step, |d| {
            d.set("Plot · NSCF", "Status", "failed");
            d.set("Plot · NSCF", "Failure", format!("{:#}", e));
            Ok(())
        })?;
    }
}
```

### 阶段 3：Electronic 自动绘图

```rust
// workspace_science.rs:383-386
Step::Electronic => {
    run_electronic_leaf(cfg, "qe_eband")?;
    run_electronic_leaf(cfg, "qe_eledos")?;
    // 新增：立即画 Electronic 全部图（能带范围、能带、DOS、PDOS）
    if let Err(e) = super::workspace_qe_plot::refresh(cfg, step) {
        eprintln!("Warning: Electronic plotting failed: {:#}", e);
        super::readable_state::update(root, step, |d| {
            d.set("Plot · Electronic", "Status", "failed");
            d.set("Plot · Electronic", "Failure", format!("{:#}", e));
            Ok(())
        })?;
    }
}
```

### 阶段 4：Phonon 自动绘图

```rust
// workspace_science.rs:387
Step::Phonon => {
    run_phonon(cfg, &qe)?;
    // 新增：立即画声子能带图 + 声子 DOS 图
    if let Err(e) = super::workspace_qe_plot::refresh(cfg, step) {
        eprintln!("Warning: Phonon plotting failed: {:#}", e);
        super::readable_state::update(root, step, |d| {
            d.set("Plot · Phonon", "Status", "failed");
            d.set("Plot · Phonon", "Failure", format!("{:#}", e));
            Ok(())
        })?;
    }
}
```

### 阶段 5：Wannier 自动绘图

```rust
// workspace_science.rs:388
Step::Wannier => {
    run_wannier(cfg, &qe)?;
    // 新增：立即画四类对比图
    if let Err(e) = refresh_post(cfg, step) {
        eprintln!("Warning: Wannier plotting failed: {:#}", e);
        // refresh_post 内部已处理失败状态记录
    }
}
```

### 阶段 6：Coarse 自动绘图

```rust
// workspace_science.rs:389-397
Step::Coarse => {
    interpolation::scheduler::run_coarse_with_retry(...)?;
    // 新增：立即画四类对比图 + 实空间衰减图
    if let Err(e) = refresh_post(cfg, step) {
        eprintln!("Warning: Coarse plotting failed: {:#}", e);
        // refresh_post 内部已处理失败状态记录
    }
}
```

## 关键设计决策

### 决策 1：绘图失败不阻断科学完成

**理由**：绘图是派生产物，科学数据才是核心。绘图失败时，用户仍可通过 `--plot` 重新生成。

**实现**：
- 使用 `if let Err(e)` 捕获绘图异常
- 记录失败状态到 readable_state
- 不传播错误到科学完成状态

### 决策 2：复用现有绘图函数

**理由**：不修改图像样式、配色、DPI 等参数，保持一致性。

**实现**：
- 调用 `workspace_qe_plot::refresh(cfg, step)`
- 调用 `refresh_post(cfg, step)`
- 不新增绘图函数，只在执行位置调用

### 决策 3：分阶段渐进实施

**理由**：6 个阶段独立，可逐个验证，降低风险。

**顺序**：
1. SCF/NSCF（最简单，只有能带范围图）
2. Electronic（4 张图，已有 PDOS 功能）
3. Phonon（声子能带 + DOS）
4. Wannier（四类对比图）
5. Coarse（四类对比图 + 衰减图）

## 数据流与依赖

```
科学计算完成
    ↓
数据校验通过
    ↓
调用绘图函数
    ↓
绘图成功？
    ├─ 是 → 图片文件存在，继续
    └─ 否 → 记录失败状态，不阻断
```

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 绘图依赖数据缺失 | 绘图失败 | 检查依赖文件，缺失时跳过绘图并记录原因 |
| 绘图函数 Bug | 阶段失败 | 捕获异常，不传播到科学完成状态 |
| 复用结果时图片缺失 | 用户看不到图 | `refresh_post_inner` 检查图片，缺失则补齐 |
| 并发写入图片 | 文件损坏 | 各阶段独立目录，无并发冲突 |

## 测试策略

### 单元测试
- 测试绘图失败时的异常捕获
- 测试 readable_state 失败状态记录

### 集成测试
- 测试 SCF/NSCF/Electronic/Phonon 阶段完成后图片存在
- 测试 Wannier/Coarse 阶段完成后对比图和衰减图存在
- 测试绘图失败时科学数据仍正常交付

### 真实验证
- 在 MgB₂ 测试目录执行完整流程
- 验证 6 个阶段的图片都自动生成
- 验证 `--plot` 重新绘图功能

## 回滚计划

如果自动绘图引入问题：
1. 注释掉 `workspace_science.rs` 中的绘图调用
2. 恢复用户手动调用 `--plot` 的方式
3. 修复绘图 Bug 后再重新启用

## 文档更新计划

1. 更新 `docs/specs/0008-epwflow-foundation/spec.md`
   - 在 FR 中添加各阶段自动绘图需求
   - 在 SC 中添加对应验收标准

2. 更新 `docs/specs/0008-epwflow-foundation/three-programs.md`
   - 删除"第一阶段禁止绘图"的限制
   - 说明各阶段完成后立即绘图

3. 更新 `docs/overview/epwflow/README.md`
   - 记录自动绘图行为
   - 说明绘图失败不影响科学数据

## 验收标准映射

| 成功标准 | 验证方式 |
|----------|----------|
| SC-001 | 执行 SCF/NSCF 阶段，检查能带范围图文件存在 |
| SC-002 | 执行 Electronic 阶段，检查四张图存在 |
| SC-003 | 执行 Phonon 阶段，检查声子图存在 |
| SC-004 | 执行 Wannier/Coarse 阶段，检查对比图和衰减图存在 |
| SC-005 | 模拟绘图失败，验证科学数据仍标记完成 |
| SC-006 | 复用已有结果，删除图片，验证自动补齐 |
