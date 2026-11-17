# 任务拆解：各阶段计算完成后自动绘图

本文档按用户故事组织任务，每个任务可独立执行和验证。

## US1：SCF/NSCF 阶段自动画能带范围图

### T001：SCF 阶段完成后自动绘图

- [ ] 在 `workspace_science.rs` 的 `Step::Scf` 分支，`run_scf` 完成后调用 `workspace_qe_plot::refresh(cfg, step)`
- [ ] 捕获绘图异常，失败时调用 `readable_state::update` 记录失败状态
- [ ] 验证：执行 `epw_phonon --stage scf -s`，检查 `scf-band-ranges.png` 存在

**验证命令**：
```bash
cargo build --release
# 在测试目录执行
epw_phonon -c input.toml --stage scf -s
ls -la scf/plots/scf-band-ranges.png
```

### T002：NSCF 阶段完成后自动绘图

- [ ] 在 `workspace_science.rs` 的 `Step::Nscf` 分支，`run_nscf` 完成后调用 `workspace_qe_plot::refresh(cfg, step)`
- [ ] 捕获绘图异常，失败时记录失败状态
- [ ] 验证：执行 `epw_phonon --stage nscf -s`，检查 `nscf-band-ranges.png` 存在

**验证命令**：
```bash
epw_phonon -c input.toml --stage nscf -s
ls -la nscf/plots/nscf-band-ranges.png
```

## US2：Electronic 阶段自动画完整电子图

### T003：Electronic 阶段完成后自动绘图

- [ ] 在 `workspace_science.rs` 的 `Step::Electronic` 分支，`run_electronic_leaf` 完成后调用 `workspace_qe_plot::refresh(cfg, step)`
- [ ] 捕获绘图异常，失败时记录失败状态
- [ ] 验证：执行 `epw_phonon --stage electronic -s`，检查以下图片存在：
  - `electronic-band-ranges.png`
  - `electronic_band.png`
  - `electronic_dos.png`
  - `electronic_pdos.png`

**验证命令**：
```bash
epw_phonon -c input.toml --stage electronic -s
ls -la 3.qe-electronic/*.png
```

### T004：补充 workspace_qe_plot 的 Electronic 绘图逻辑

- [ ] 检查 `workspace_qe_plot::refresh` 是否覆盖 Electronic 阶段
- [ ] 确认 `workspace_qe_plot::electronic` 函数调用了能带范围、能带、DOS、PDOS 绘图
- [ ] 验证：查看代码逻辑，确认四张图都会生成

**代码位置**：`epwflow/src/foundation/workspace_qe_plot.rs:136-150`

## US3：Phonon 阶段自动画声子图

### T005：Phonon 阶段完成后自动绘图

- [ ] 在 `workspace_science.rs` 的 `Step::Phonon` 分支，`run_phonon` 完成后调用 `workspace_qe_plot::refresh(cfg, step)`
- [ ] 捕获绘图异常，失败时记录失败状态
- [ ] 验证：执行 `epw_phonon --stage phonon -s`，检查以下图片存在：
  - `phonon_band.png`
  - `phonon_dos.png`

**验证命令**：
```bash
epw_phonon -c input.toml --stage phonon -s
ls -la phonon/plots/*.png
```

### T006：补充 workspace_qe_plot 的 Phonon 绘图逻辑

- [ ] 检查 `workspace_qe_plot::refresh` 是否覆盖 Phonon 阶段
- [ ] 确认 Phonon 绘图调用了 q2r/matdyn 后处理和绘图
- [ ] 验证：查看代码逻辑，确认声子能带图和 DOS 图都会生成

**代码位置**：`epwflow/src/foundation/workspace_qe_plot.rs`

## US4：Wannier/Coarse 阶段自动画对比图

### T007：Wannier 阶段完成后自动绘图

- [ ] 在 `workspace_science.rs` 的 `Step::Wannier` 分支，`run_wannier` 完成后调用 `refresh_post(cfg, step)`
- [ ] 验证 `refresh_post` 内部已处理绘图失败状态记录
- [ ] 验证：执行 `epw_wannier --stage wannier -s`，检查以下图片存在：
  - 电子能带对比图
  - 电子 DOS 对比图
  - 声子能带对比图
  - 声子 DOS 对比图

**验证命令**：
```bash
epw_wannier -c input.toml --stage wannier -s
ls -la 5.epw-wannier/*.png
```

### T008：Coarse 阶段完成后自动绘图

- [ ] 在 `workspace_science.rs` 的 `Step::Coarse` 分支，`run_coarse_with_retry` 完成后调用 `refresh_post(cfg, step)`
- [ ] 验证 `refresh_post` 内部已处理绘图失败状态记录
- [ ] 验证：执行 `epw_wannier --stage coarse -s`，检查以下图片存在：
  - 同 Wannier 的四类对比图
  - 实空间衰减图

**验证命令**：
```bash
epw_wannier -c input.toml --stage coarse -s
ls -la 6.epw-coarse/*.png
```

### T009：确认 refresh_post 覆盖 Wannier/Coarse 绘图

- [ ] 阅读 `workspace_science::refresh_post_inner` 函数
- [ ] 确认 Wannier/Coarse 阶段调用了 `delivery/comparison_plot.rs` 和 `delivery/decay.rs`
- [ ] 验证：查看代码逻辑，确认四类对比图和衰减图都会生成

**代码位置**：`epwflow/src/foundation/workspace_science.rs:146-201`

## 通用任务

### T010：绘图失败不阻断科学完成

- [ ] 在所有阶段的绘图调用中，使用 `if let Err(e)` 捕获异常
- [ ] 失败时使用 `eprintln!` 输出警告信息
- [ ] 失败时调用 `readable_state::update` 记录失败状态
- [ ] 验证：模拟绘图失败（例如删除依赖文件），确认科学数据仍标记完成

**验证方式**：
```bash
# 删除绘图依赖文件，模拟失败
rm -f nscf/out/*.xml
epw_phonon -c input.toml --stage nscf -s
# 检查 readable_state 中有失败记录，但 NSCF 数据仍标记完成
```

### T011：保留 --plot 选项用于显式重绘

- [ ] 确认 `--plot` 选项逻辑不变
- [ ] 确认 `--plot` 调用的绘图函数与自动绘图使用同一函数
- [ ] 验证：删除图片后，使用 `--plot` 重新生成

**验证命令**：
```bash
rm -f scf/plots/scf-band-ranges.png
epw_phonon -c input.toml -w <work_dir> --plot
ls -la scf/plots/scf-band-ranges.png
```

### T012：复用结果时补齐缺失图片

- [ ] 确认 `refresh_post_inner` 检查图片文件存在性
- [ ] 确认缺失时调用绘图函数补齐
- [ ] 确认不触发科学程序重新计算
- [ ] 验证：复用已有结果，删除图片，确认自动补齐

**验证方式**：
```bash
# 第一次完整执行
epw_phonon -c input.toml --stage scf -s
# 删除图片
rm -f scf/plots/scf-band-ranges.png
# 复用结果（不重新计算 SCF）
epw_phonon -c input.toml --stage nscf -s
# 检查 SCF 图片自动补齐
ls -la scf/plots/scf-band-ranges.png
```

## 文档更新任务

### T013：更新 0008 Spec 文档

- [ ] 在 `docs/specs/0008-epwflow-foundation/spec.md` 添加各阶段自动绘图需求
- [ ] 在 FR 中添加对应功能需求
- [ ] 在 SC 中添加对应验收标准

### T014：更新 three-programs.md

- [ ] 在 `docs/specs/0008-epwflow-foundation/three-programs.md` 删除"第一阶段禁止绘图"的限制
- [ ] 添加说明：各阶段完成后立即自动绘图

### T015：更新 epwflow Overview

- [ ] 在 `docs/overview/epwflow/README.md` 记录自动绘图行为
- [ ] 说明绘图失败不影响科学数据交付
- [ ] 说明 `--plot` 选项用于显式重绘

## 验收任务

### T016：MgB₂ 完整流程验证

- [ ] 在 MgB₂ 测试目录执行完整流程
- [ ] 验证 6 个阶段的所有图片都自动生成
- [ ] 验证图片质量（DPI、内容完整性）
- [ ] 验证绘图失败时的状态记录

**验证命令**：
```bash
cd /home/mayuan/code/dftflow/test_auto/MgB2/local_run
epw_phonon -c input.toml --stage scf -s
epw_phonon -c input.toml --stage nscf -s
epw_phonon -c input.toml --stage electronic -s
epw_phonon -c input.toml --stage phonon -s
epw_wannier -c input.toml --stage wannier -s
epw_wannier -c input.toml --stage coarse -s
# 检查所有图片
find . -name "*.png" -type f
```

### T017：编译并覆盖正式入口

- [ ] `cargo build --release`
- [ ] 复制二进制文件到正式入口（按 AGENTS.md 要求）
- [ ] 验证帮助信息和版本信息

**验证命令**：
```bash
cargo build --release
cp target/release/epw_phonon ~/bin/
cp target/release/epw_wannier ~/bin/
cp target/release/epw_superconductivity ~/bin/
epw_phonon --help
epw_wannier --help
```

### T018：Git 提交

- [ ] 检查工作树，只暂存本次修改的文件
- [ ] 提交消息使用 AGENTS.md 规定格式
- [ ] 记录修改文件、影响范围和验证方式

**提交格式**：
```
[2026-11-17 HH:MM:SS] 新增：各阶段计算完成后自动绘图

[修改内容]
- epwflow/src/foundation/workspace_science.rs: 在 6 个阶段完成后调用绘图函数
- docs/specs/0072-stage-auto-plotting/: 创建完整 Spec 文档
- docs/specs/0008-epwflow-foundation/spec.md: 添加自动绘图需求
- docs/specs/0008-epwflow-foundation/three-programs.md: 删除第一阶段禁止绘图限制
- docs/overview/epwflow/README.md: 记录自动绘图行为

[影响范围]
- epwflow 6 个计算阶段的执行流程
- 绘图触发时机从手动变为自动

[验证方式]
- MgB₂ 完整流程测试，所有阶段图片自动生成
- 绘图失败测试，科学数据仍正常交付
- 复用结果测试，缺失图片自动补齐

已更改 X 个文件, XX 行插入(+), XX 行删除(-)
```

## 任务依赖关系

```
T001 (SCF 绘图) → 独立
T002 (NSCF 绘图) → 独立
T003 (Electronic 绘图) → T004
T004 (Electronic 绘图逻辑检查) → 独立
T005 (Phonon 绘图) → T006
T006 (Phonon 绘图逻辑检查) → 独立
T007 (Wannier 绘图) → T009
T008 (Coarse 绘图) → T009
T009 (refresh_post 检查) → 独立
T010 (绘图失败处理) → 所有绘图任务
T011 (--plot 选项) → 独立
T012 (复用补图) → 独立
T013-T015 (文档更新) → T001-T009 完成
T016 (MgB₂ 验证) → T001-T015 完成
T017 (编译发布) → T016 完成
T018 (Git 提交) → T017 完成
```

## 风险与注意事项

1. **绘图依赖数据缺失**：捕获异常，记录失败原因，不阻断科学完成
2. **并发问题**：各阶段独立目录，无并发冲突
3. **回归风险**：保留 `--plot` 选项，用户可手动重绘
4. **性能影响**：绘图在同一作业内执行，增加少量时间，但用户体验更好
