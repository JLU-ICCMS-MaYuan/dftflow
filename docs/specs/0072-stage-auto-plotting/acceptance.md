# 验收记录：各阶段计算完成后自动绘图

## 验收状态

**当前状态**：未开始实施

**最后更新**：2026-11-17

## 功能需求验收

### FR-001：SCF 阶段自动绘图

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：执行 `epw_phonon --stage scf -s`，检查 `scf-band-ranges.png` 存在
- [ ] **备注**：

### FR-002：NSCF 阶段自动绘图

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：执行 `epw_phonon --stage nscf -s`，检查 `nscf-band-ranges.png` 存在
- [ ] **备注**：

### FR-003：Electronic 阶段自动绘图

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：执行 `epw_phonon --stage electronic -s`，检查四张图存在
- [ ] **备注**：

### FR-004：Phonon 阶段自动绘图

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：执行 `epw_phonon --stage phonon -s`，检查声子图存在
- [ ] **备注**：

### FR-005：Wannier 阶段自动绘图

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：执行 `epw_wannier --stage wannier -s`，检查四类对比图存在
- [ ] **备注**：

### FR-006：Coarse 阶段自动绘图

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：执行 `epw_wannier --stage coarse -s`，检查四类对比图和衰减图存在
- [ ] **备注**：

### FR-007：绘图失败不阻断科学完成

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：模拟绘图失败，检查科学数据仍标记完成，readable_state 记录失败
- [ ] **备注**：

### FR-008：保留 --plot 选项

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：删除图片后，使用 `--plot` 重新生成
- [ ] **备注**：

### FR-009：复用结果补齐图片

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：复用已有结果，删除图片，检查自动补齐
- [ ] **备注**：

## 成功标准验收

### SC-001：SCF/NSCF 能带范围图

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：执行 SCF/NSCF 阶段，检查图片存在且 readable_state 记录正确
- [ ] **备注**：

### SC-002：Electronic 四张图

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：执行 Electronic 阶段，检查四张图存在；PDOS 失败时其他三张图仍生成
- [ ] **备注**：

### SC-003：Phonon 声子图

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：执行 Phonon 阶段，检查声子能带图和 DOS 图存在
- [ ] **备注**：

### SC-004：Wannier/Coarse 对比图和衰减图

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：执行 Wannier/Coarse 阶段，检查对比图和衰减图存在
- [ ] **备注**：

### SC-005：绘图失败不影响科学数据

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：模拟绘图失败，检查科学数据交付状态，用户可用 `--plot` 重绘
- [ ] **备注**：

### SC-006：复用结果自动补图

- [ ] **状态**：未验收
- [ ] **证据**：待实施后提供
- [ ] **验证方式**：复用结果，删除图片，检查不触发 SCF/NSCF/PH/EPW 重新计算
- [ ] **备注**：

## 真实数据验收

### MgB₂ 完整流程测试

- [ ] **状态**：未验收
- [ ] **测试目录**：`/home/mayuan/code/dftflow/test_auto/MgB2/local_run`
- [ ] **执行命令**：
  ```bash
  epw_phonon -c input.toml --stage scf -s
  epw_phonon -c input.toml --stage nscf -s
  epw_phonon -c input.toml --stage electronic -s
  epw_phonon -c input.toml --stage phonon -s
  epw_wannier -c input.toml --stage wannier -s
  epw_wannier -c input.toml --stage coarse -s
  ```
- [ ] **图片清单**：待验证后列出
- [ ] **图片质量**：待验证后确认（DPI、内容完整性）
- [ ] **备注**：

## 文档验收

### Spec 文档更新

- [x] **状态**：已创建
- [x] **文件**：`docs/specs/0072-stage-auto-plotting/spec.md`
- [x] **内容**：完整的需求、用户故事、功能需求和成功标准
- [x] **备注**：待实施后更新状态

### Plan 文档创建

- [x] **状态**：已创建
- [x] **文件**：`docs/specs/0072-stage-auto-plotting/plan.md`
- [x] **内容**：技术栈、架构分层、执行顺序、设计决策
- [x] **备注**：待实施后更新

### Tasks 文档创建

- [x] **状态**：已创建
- [x] **文件**：`docs/specs/0072-stage-auto-plotting/tasks.md`
- [x] **内容**：按用户故事组织的任务拆解
- [x] **备注**：待实施后逐项勾选

### Quickstart 文档创建

- [x] **状态**：已创建
- [x] **文件**：`docs/specs/0072-stage-auto-plotting/quickstart.md`
- [x] **内容**：快速验证步骤
- [x] **备注**：待实施后更新

### 0008 Spec 更新

- [ ] **状态**：待更新
- [ ] **文件**：`docs/specs/0008-epwflow-foundation/spec.md`
- [ ] **内容**：添加各阶段自动绘图需求
- [ ] **备注**：

### three-programs.md 更新

- [ ] **状态**：待更新
- [ ] **文件**：`docs/specs/0008-epwflow-foundation/three-programs.md`
- [ ] **内容**：删除"第一阶段禁止绘图"的限制
- [ ] **备注**：

### Overview 更新

- [ ] **状态**：待更新
- [ ] **文件**：`docs/overview/epwflow/README.md`
- [ ] **内容**：记录自动绘图行为
- [ ] **备注**：

## Issue 关联

### GitHub Issue #72

- [x] **状态**：已创建
- [x] **链接**：https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/72
- [x] **类型**：type:feature
- [x] **父级**：#8 (epwflow 基础)
- [ ] **双向链接**：待 Spec 推送后验证远程链接
- [ ] **备注**：

## 实施记录

### 代码修改

- [ ] **文件**：`epwflow/src/foundation/workspace_science.rs`
- [ ] **修改内容**：待实施后记录
- [ ] **行数**：待统计
- [ ] **备注**：

### 构建与安装

- [ ] **状态**：未构建
- [ ] **命令**：`cargo build --release`
- [ ] **安装**：待执行
- [ ] **备注**：

### Git 提交

- [ ] **状态**：未提交
- [ ] **提交信息**：待按 AGENTS.md 格式编写
- [ ] **变更文件数**：待统计
- [ ] **变更行数**：待统计
- [ ] **备注**：

## 遗留问题

无。

## 下一步行动

1. 等待用户确认 Spec 文档
2. 开始实施 tasks.md 中的任务
3. 逐项验收并记录证据
4. 更新相关文档
5. 提交 Git 并关闭 Issue
