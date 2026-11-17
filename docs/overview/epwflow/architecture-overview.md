# EPW 工作流架构总览

## 一、整体流程图

```
                        ┌─────────────────────────────────────┐
                        │      用户配置 (input.toml)          │
                        │  - 结构文件 (POSCAR)                │
                        │  - 程序路径 & 参数                  │
                        │  - 物理参数 (网格、能窗等)          │
                        └──────────────┬──────────────────────┘
                                       │
                                       ▼
                        ┌─────────────────────────────────────┐
                        │   epw_phonon / epw_wannier /        │
                        │   epw_superconductivity             │
                        │   (三个独立 CLI 入口)               │
                        └──────────────┬──────────────────────┘
                                       │
                ┌──────────────────────┼──────────────────────┐
                │                      │                      │
                ▼                      ▼                      ▼
        ┌───────────┐          ┌───────────┐          ┌───────────┐
        │  阶段 1-4  │          │  阶段 5-6  │          │  阶段 7-8  │
        │  (声子链)  │          │ (Wannier) │          │  (超导)    │
        └─────┬─────┘          └─────┬─────┘          └─────┬─────┘
              │                      │                      │
              │                      │                      │
    ┌─────────▼─────────┐  ┌─────────▼─────────┐  ┌─────────▼─────────┐
    │  1. SCF           │  │  5. EPW Wannier   │  │  7. 细网格 EPC    │
    │  ↓                │  │     - 拟合        │  │     - λ, α²F      │
    │  2. NSCF          │  │     - 电子能带    │  │  ↓                │
    │  ↓                │  │     - DOS         │  │  8. 超导求解      │
    │  3. QE Electronic │  │  ↓                │  │     - Tc/Gap      │
    │     - bands.x     │  │  6. EPW Coarse    │  │     - iso/aniso   │
    │     - dos.x       │  │     - 粗网格 EPC  │  └───────────────────┘
    │  ↓                │  │     - 实空间衰减  │
    │  4. QE Phonon     │  │     - 声子比较    │
    │     - ph.x        │  └───────────────────┘
    │     - q2r         │
    │     - matdyn      │
    └───────────────────┘

每个阶段：
  输入：上游结果 + 配置参数
    ↓
  复用检查：compare_fingerprints(上游哈希, 配置参数)
    ↓
  决策：Reuse | Update | Conflict
    ↓
  执行：生成输入 → 提交作业 → 核验输出
    ↓
  发布：results/<阶段>/
```

## 二、数据流向与依赖关系

### 2.1 显式依赖链

```
1.scf.log ────────┐
                  ├──→ 2.nscf.log ────────┐
                  │                       ├──→ 3.qe-electronic.log
                  │                       │
                  └──→ 4.qe-phonon.log ───┤
                                          │
                                          ├──→ 5.epw-wannier.log
                                          │
                  ┌───────────────────────┘
                  │
                  ├──→ 6.epw-coarse.log
                  │
                  └──→ 7.epw-fine.log ──→ 8.epw-superconductivity.log
```

### 2.2 文件复用矩阵

| 阶段             | 复用自         | 关键文件                                    |
|------------------|----------------|---------------------------------------------|
| 1.scf            | -              | prefix.save/, prefix.xml                   |
| 2.nscf           | 1.scf          | prefix.save/, nscf.xml (完整 k 网格)       |
| 3.qe-electronic  | 2.nscf         | bands.dat.gnu, dos.dat                     |
| 4.qe-phonon      | 1.scf (快照)   | prefix.dyn*, prefix.fc, matdyn.freq        |
| 5.epw-wannier    | 2,4            | prefix.wout, prefix.chk, prefix.eig        |
| 6.epw-coarse     | 5              | prefix.ukk, decay.*, epmatwp               |
| 7.epw-fine       | 6              | prefix.fmt (细网格矩阵元)                   |
| 8.超导           | 7              | a2f.*, selfen_*.*, .ahc_* (各温度点)       |

**复用策略**：
- **只读复制**：SCF → PH 快照（独立目录，避免 PH 修改原 SCF）
- **符号链接**（已废弃）：早期使用，现改为哈希校验 + 物理复制
- **指纹绑定**：每个阶段日志保存 `input_fingerprint` 字段

## 三、状态管理机制

### 3.1 阶段状态机

```
每个阶段的生命周期：

  [Not Started]
       │
       ├─ 配置准备
       ↓
  [Prepared] ──────────→ [Skipped]（已有相容结果）
       │
       ├─ 提交作业 (-s)
       ↓
  [Submitted] ─┬─→ [Running]
               │        │
               │        ├─ 科学计算失败
               │        ↓
               │   [Failed] ──→ --resume -s
               │
               └─ 作业完成
                    ↓
               [Completed] ──→ 核验输出
                    ↓
               [Verified] ──→ 后处理（绘图等）
                    ↓
               [Published]
```

### 3.2 状态持久化

每个阶段日志 (`epw-state/<N>.<name>.log`) 包含：

```json
{
  "stage": "5.epw-wannier",
  "status": "published",
  "slurm_job_id": 1234567,
  "input_fingerprint": "sha256:abc123...",
  "upstream_fingerprints": {
    "2.nscf": "sha256:def456...",
    "4.qe-phonon": "sha256:ghi789..."
  },
  "config_version": {
    "saved_at": "2026-11-17T10:30:00Z",
    "content_hash": "sha256:jkl012..."
  },
  "native_output": {
    "exit_code": 0,
    "runtime_seconds": 3600,
    "convergence": "success"
  },
  "results": {
    "published_at": "2026-11-17T11:00:00Z",
    "plots": ["electronic.png", "electronic-from-lowest-to-highest.png"]
  }
}
```

## 四、并发控制与锁机制

### 4.1 三层保护

```
1. Slurm 作业保护
   - 检查 .jobs 文件中的作业号
   - 用 scontrol/sacct 查询作业状态
   - 活动作业 → 拒绝重复提交

2. 文件系统锁（已移除 workflow_lock）
   - 进程内使用 flock 保护日志写入
   - 不跨进程持久化锁文件

3. 结果发布锁
   - 发布到 results/ 时使用临时锁
   - 避免并发写入冲突
```

### 4.2 恢复语义

```bash
# 场景 1：作业失败，科学参数不变
epw_wannier --stage wannier --resume -s
  → 重验上游指纹
  → 复用成功部分
  → 重新提交失败任务

# 场景 2：修改配置参数
epw_wannier --stage wannier -s
  → 检测配置变化
  → 保存旧配置版本到 config-versions/
  → 标记下游为 needs_update
  → 提交新任务

# 场景 3：删除阶段目录
epw_wannier --stage wannier -s
  → 复用检查返回 Update
  → 自动重建该阶段
  → 撤销下游 Published 状态
```

## 五、关键设计原则

### 5.1 单一数据源原则（Single Source of Truth）

- **配置**：`input.toml` 是唯一输入源（冻结快照除外）
- **状态**：`epw-state/*.log` 是唯一状态源
- **结果**：`results/` 是唯一发布源
- **不允许**：从中间文件反向推断状态

### 5.2 显式优于隐式

```toml
# ✅ 好的设计：显式程序路径
[programs]
pw_x = "/opt/qe/bin/pw.x"
ph_x = "/opt/qe/bin/ph.x"
epw_x = "/opt/epw/bin/epw.x"

# ❌ 坏的设计：隐式路径推导
# qe_root = "/opt/qe"  # 工具猜测 bin/pw.x
```

### 5.3 失败快速原则（Fail Fast）

```rust
// ✅ 提交前全面检查
pub fn preflight_check() -> Result<()> {
    check_upstream_ready()?;      // 上游完成？
    check_config_valid()?;        // 配置合法？
    check_no_active_jobs()?;      // 无活动作业？
    check_program_available()?;   // 程序可执行？
    check_resources_sufficient()?; // 资源足够？
    Ok(())
}

// ❌ 提交后才发现问题
// 提交 → 排队 → 启动 → 发现上游文件缺失 → 失败
```

### 5.4 幂等性保证

```bash
# 多次执行相同命令，结果相同
epw_wannier --stage wannier -s
epw_wannier --stage wannier -s  # 第二次：检测已完成，不重提

# 除非显式 --resume
epw_wannier --stage wannier --resume -s  # 强制恢复
```

## 六、与 VASP/QE 工作流的对比

| 维度             | VASP 工作流           | QE/EPW 工作流             | 设计启示                     |
|------------------|-----------------------|---------------------------|------------------------------|
| 目录结构         | 按功能分（`vasp_scf/`）| 按阶段编号（`1.scf/`）     | 编号更清晰，推荐              |
| 配置方式         | `[vasp_scf].kmesh`    | `stages."1.scf".kmesh`    | 嵌套更灵活                   |
| 任务提交         | 单阶段脚本            | 阶段链自动推进            | EPW 模式更适合长链           |
| 复用机制         | CHGCAR/WAVECAR 路径   | 内容哈希 + 指纹           | 哈希更可靠                   |
| 状态管理         | 无显式状态文件        | 八个阶段日志              | 显式状态是 EPW 优势          |

## 七、待改进方向

### 7.1 可观测性增强

```bash
# 建议新增命令
epw_workflow --status --all     # 全局状态总览
epw_workflow --timeline         # 时间线视图
epw_workflow --dependency-tree  # 依赖树可视化
```

### 7.2 错误恢复增强

```bash
# 当前：手动清理 + --resume
rm -rf 5.epw-wannier/
epw_wannier --stage wannier --resume -s

# 建议：自动清理模式
epw_wannier --stage wannier --clean --resume -s
```

### 7.3 配置校验

```rust
// 建议：提交前模拟完整链
pub fn validate_full_workflow(config: &Config) -> Result<ValidationReport> {
    // 检查每个阶段的参数兼容性
    // 预估资源需求
    // 生成 dry-run 报告
}
```

## 八、参考资料

- [完整流程图与续算说明](workflow.md)
- [开算前检查与旧结果复用](preflight-and-reuse.md)
- [原生阶段配置](native-stage-config.md)
- [原生超导适配](native-superconductivity.md)
