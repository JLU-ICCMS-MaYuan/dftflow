# 0075 验收记录

## 范围

2026-11-18：补齐三／六元素 SCF 数组的解析、校验、配置写回、QE 卡片生成，以及 EPW→qeflow 的偏移传递。NSCF 继续使用三元素零偏移网格，完整 EPW 工作流保留同网格、零偏移约束。

旧 SCF automatic 字符串可读取并写回数组，旧 crystal 卡片保留类型与坐标。独立 QE 声子工作流对旧 k_shift 字段明确提示迁移。

## 复现与回归

修复前新增用例确认两处失败：SCF 数组被原生卡片校验拒绝；QE 输入适配仍按字符串读取数组。修复后相关回归通过。

- `qf-config`、`qf-input`、`qeflow`：59 项通过。
- EPW 的 `common_stage_config`、`flat_stage_config`、`native_stage_config`、`nscf_grid_contract`、`qe_data`：最终共 63 项通过，3 项按原有环境条件跳过。
- 共 122 项通过，3 项跳过；跳过项不计为科学验收通过。

验证命令：

```bash
bash "scripts/build-native.sh" test --release --locked -p qf-config -p qf-input -p qeflow -j 6
bash "scripts/build-native.sh" test --release --locked -p epwflow --test native_stage_config --test common_stage_config --test nscf_grid_contract --test qe_data --test flat_stage_config -j 6
```

本机 `/tmp` 存在配额限制，本轮设置 `TMPDIR` 指向仓库构建缓存中的临时目录。最后新增的旧 crystal 卡片兼容用例随 `native_stage_config` 单独复测通过，该目标最终为 21 项通过。

## 构建与发布

工作区存在其他任务改动。本轮导出 HEAD 后只覆盖本任务文件，以隔离源码快照构建工作区 release 产品；不创建或切换 Git 分支。构建缓存继续使用固定 `bin/.build/linux-x86_64`，成功后覆盖唯一公开模块入口。

构建命令为 `bash "scripts/build-native.sh" build --release --workspace --locked --manifest-path "<隔离源码>/Cargo.toml" -j 6`。

工作区 release 构建成功，自动覆盖 9 个主程序：qeflow、vaspflow、scflow、wannier90、substitution、epw_phonon、epw_wannier、epw_superconductivity、epw_clear。每个公开程序均通过 `--help` 启动检查，且 SHA-256 与本次构建缓存中的对应制品一致；EPW 四入口与 QE 主入口、兼容入口的命令发现路径均指向固定模块目录。

构建后逐文件核对，实际编译快照中的本任务代码与待提交工作区文件一致。只读独立复核未发现阻断缺陷；构建仅有既有依赖及其他模块警告，无编译错误。

## 边界

未提交真实 QE／EPW 或 Slurm 科学作业；未修改科学数据。绘图线宽、0061 文档、MgB₂ 示例的网格与其他参数改动，以及三份既有 PDF 删除不纳入本次提交或编译源码。
