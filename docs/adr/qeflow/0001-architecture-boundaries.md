# qeflow 架构边界

- 状态：Accepted
- 日期：2026-05-28
- 相关模块：`qeflow/`

## 背景

`qeflow` 已从早期以 CLI 文件为中心的实现，演进为多个有明确职责的模块：

- `setup` 负责 QE calculation setup 生命周期。
- `crystal` 负责结构解析、化学式、kmesh 和晶格参数推导。
- `input_deck` 负责 QE 原生输入 deck 写出。
- `io_reader` 负责读取 QE 原生输入、输出和后处理文件。
- `phonon` 负责声子工作流编排、状态机、split q 点、merge、后处理和绘图。
- `check` 负责低层输出文件检查。

如果这些职责重新混在 CLI 或兼容 facade 中，SCF/NSCF/EBAND/ELEDOS/phonon 会再次出现重复的目录生命周期、输入写出和状态判断逻辑。因此需要用 ADR 固化当前边界，作为后续新增 QE 能力和重构时的架构约束。

## 决策

qeflow 的依赖方向固定为：

```text
CLI Adapter
  -> setup / phonon workflow
  -> input_deck / io_reader / crystal / check
  -> common / io_utils / public helpers
```

模块职责如下：

| 模块 | 职责 | 不应承担的职责 |
| --- | --- | --- |
| `qeflow/scf`、`qeflow/nscf`、`qeflow/eband`、`qeflow/eledos` | 保留 CLI Adapter 和后处理入口 | 不直接维护通用 setup 生命周期 |
| `qeflow/setup` | 编排 QE Calculation Setup：配置、结构副本、依赖准备、输入写出、脚本、运行/提交 | 不直接拼接 QE 输入 deck 细节 |
| `qeflow/crystal` | POSCAR 解析、结构语义、化学式、kmesh、`cell2ibrav` fallback 所需晶格推导 | 不写 QE `.in` 文件 |
| `qeflow/input_deck` | 按 QE 程序写出 `pw/ph/q2r/matdyn/bands/dos` 输入和脚本片段 | 不读取 QE 输出，不判断 workflow 状态 |
| `qeflow/io_reader` | 读取 QE 原生输出和后处理产物 | 不编排运行，不写输入 deck |
| `qeflow/phonon` | 声子 nosplit/split 工作流、状态机、split materialization、merge、postprocess、plot | 不复制 `input.toml` 作为配置真源，不重新实现通用 QE reader |
| `qeflow/check` | 基于文件内容判断单个 QE 步骤三态 | 不维护声子整体续算策略 |
| `qeflow/common.py` | 跨 qeflow 的小型公共能力，如赝势解析、配置路径归一化 | 不承载大型工作流对象 |

## 架构约束

1. 用户命令名保持兼容。内部模块可以重命名，`pyproject.toml` 中 console script 名称不因内部重构轻易改变。
2. CLI 文件应保持薄层：解析参数、选择 mode、调用对象式 workflow 或 setup。
3. 新增 QE 输入写出逻辑时，优先进入 `qeflow/input_deck/`，并按 QE 程序或 deck 类型拆分。
4. 新增 QE 输出读取逻辑时，优先进入 `qeflow/io_reader/`，避免散落在 workflow、plot 或 check 模块中。
5. 新增结构语义时，优先进入 `qeflow/crystal/`；QE 语法序列化仍留在 `input_deck`。
6. 声子状态只由 `qeflow.phonon.status_machine` 汇总和写回 `workflow_status.json`；`check` 只提供低层判断。
7. `qeflow/phonon` 工作目录只保留 `POSCAR` 副本，不复制 `input.toml`；配置真源通过 `-c/--config` 显式传入。
8. 不新增旧式 facade 或只为兼容内部导入而存在的转发层；如需兼容，只保留用户可见 CLI 行为。
9. 文档同步要求：修改模块边界时，同步更新本 ADR 或新增 ADR，并检查 `qeflow/README.md` 与对应 `.mmd` 流程图。

## 影响

收益：

- `qe_scf/qe_nscf/qe_eband/qe_eledos` 共享同一套 setup 生命周期，减少重复和行为分叉。
- `input_deck` 与 `io_reader` 分离，使 QE 文件写出和读取可以独立测试。
- `phonon` 的状态机、split、merge、postprocess 边界清晰，便于维护 Slurm 提交与本地运行两套路径。
- `crystal` 与 QE 语法解耦，便于未来复用结构语义到其它工作流。

代价：

- 新增功能需要先判断模块归属，不能直接写进最近的 CLI 文件。
- 跨模块修改时必须同步 README、Mermaid 流程图和 ADR，否则文档会落后于代码。

## 验证

涉及 qeflow 架构边界的变更至少检查：

- `rg "qeflow\\.qe_|qeflow\\.phonon\\._shared|qeflow\\.qe_input" qeflow test_auto`
- 聚焦 qeflow 测试，例如 `pytest test_auto/MgB2/test_qeflow.py test_auto/MgB2/test_qe_phonon.py`
- `qeflow/README.md` 与相关 `qeflow/**/*.mmd` 是否仍描述真实 CLI 链路

## 后续

- 若新增非声子 QE 后处理 workflow，优先评估是否需要独立 workflow 模块，而不是放入 CLI。
- 若 `check` 需要覆盖 matdyn band/dos/eph，保持低层检查职责，不把声子续算策略移入 `check`。
