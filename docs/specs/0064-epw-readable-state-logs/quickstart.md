# 0064 验证与使用

本项源码和受控回归已完成，见 [验收](acceptance.md)。2026-10-28 已按用户追加授权全工作区编译发布当前 CLI，并在指定 MgB₂ 工作区从头真实验证；执行结果与证据边界见验收末节。

用户原有 CLI 和 `--config` 输入路径继续使用。配置原文变化时在解析后的 `work_dir/epw-state/` 创建下一编号 TOML；`epw-state/` 用八份日志查看阶段现状，任务号位于各编号目录 `.jobs`。恢复仍使用所属公开命令与 `--resume -s`，有效子任务保留；明确重算直接覆盖本阶段。

例如，查看声子阶段当前状态：

```bash
cat "epw-state/4.qe-phonon-nosplit.log"
```

日志采用英文小节展示当前事实；实际文件还会列出 PH／后处理子任务、输入版本和产物路径：

```text
Step 4 · QE phonons
================================================================

[Stage]
  Status            : running
  Phonon mode       : nosplit

[Next step]
  Action            : After the job finishes, run: epw_phonon -c input.toml --stage phonon -s

================================================================
End of record
```

上面是阅读样例，不能手工写入日志来宣告完成。程序续算时仍检查实际数值文件与 Slurm 状态；调度结束不会自动等于科学完成。

需要对照作业号时，直接查看计算目录内的任务号表：

```bash
cat "4.qe-phonon-nosplit/4.qe-phonon-nosplit.jobs"
```

配置备份保存原始 TOML 文本，包含注释与排版。连续两次运行内容相同只保存一份；修改后再运行 CLI 才生成下一份。恢复旧配置内容后再次运行，也会保存为新的编号版本。

编号版本统一位于解析后的 `work_dir/epw-state/`。例如原文依次为 A、A、B、A，会得到 `0000-config.toml`、`0001-config.toml`、`0002-config.toml`，中间相同的 A 不重复保存。相对路径继续以原输入目录为基准；每版原始目录及原文哈希保存在 `1.qe-scf.log` 的 `Configuration snapshot` 小节，不手工编辑该记录或快照。

更新 CLI 后照常使用原 `--config` 文件；原输入旁的旧编号快照会自动核验、迁入 `epw-state/`、更新日志引用，最后移除旧位置文件，不需要手工搬动。只想迁移而不启动科学任务，可执行下文 `--migrate-state` 命令。活动／未知旧作业或目标内容冲突时程序保留原文件并明确报错，完成旧任务或查明冲突后重试相同命令。

验证使用 `CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p epwflow`，先运行新可读状态／配置／提交恢复用例，再运行已有相关公开入口回归。新二进制固定后才运行进程测试，不在执行中重新编译覆盖。

本次真实重算只覆盖用户指定的 MgB₂ 工作区，SiP₂ 现场保持不动。安装、受控回归与真实数值分别记录，不以源码回归代替科学结果。

## 已有 SiP₂ 的旧状态转换

已发布 CLI 可用以下命令退出旧状态，不提交科学计算：

```bash
cd "/data/home/mayuan/code/dftflow/examples/SiP2"
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
epw_wannier --config "epwinput.toml" --migrate-state
```

旧可见 epw-state 在公开入口先自动转换；命令末尾可能提示已采用八阶段日志。检查实际 `epw-state/` 应有八份日志、编号 TOML 及保留的 imports。旧 JSON、旧历史和旧回执直接删除，不搬移、不备份；历史内计算副本也随历史删除。当前计算目录、results 和外部声子资料保留。需要的历史须预先另存，未知或活动任务阻断时不能手工删状态。完整说明见 [SiP₂ README](../../../examples/SiP2/README.md#旧-epw-state-转换为八份阶段日志0064)。本轮只追加操作文档，未转换真实现场。

## 已迁移的旧能带赝势路径报错

旧 `eband.in`／QE XML 中记录的 `attempts/.../pp` 可以继续保留为历史计算信息，不手工改写它们，也不重建已删除目录。程序对当前配置生成的输入进行完整比较，仅允许 `CONTROL.pseudo_dir` 位置不同；还要逐元素核对保存 XML 的赝势文件映射、保存树中的 UPF 与当前配置赝势的 SHA-256。截断能、结构、带数、k 路径、赝势内容或上游批次不一致仍拒绝复用。

若八日志转换后第 3 步仍显示“旧结果需要检查”，使用修复后的正式 CLI 重新执行以下命令：

```bash
cd "/data/home/mayuan/code/dftflow/examples/SiP2"
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
epw_wannier --config "epwinput.toml" --migrate-state
```

这次只重新核验并登记已有电子结果，不准备或运行科学输入、不提交 Slurm 作业。成功时第 3 步日志显示“已完成”，旧报错清除；失败则保留具体原因和原始科学文件，不自动重算。重复执行不会增加科学任务。

## 英文输出检查

公开入口的 --help、配置错误、--status 和运行提示均应使用英文。复用子任务示例为 reuse: 3.qe-electronic /projwfc；主阶段示例为 reuse: 1.qe-scf，声子阶段显示实际模式。

旧中文日志和任务号表可继续读取；不要手工替换字段、重写哈希或删除状态来转换语言。程序正常更新记录时改用英文，用户路径和原始配置文字保持原样。只读状态查询不会为了转换语言重写记录。
