# Architecture Decision Records

本目录用于记录 DFTFlow 的架构决策。ADR 按子系统分区，避免 `qeflow`、`vaspflow`、`substitutionflow` 等模块的决策混在同一层。

ADR 只记录仍然影响后续开发的决策，不替代 README、Mermaid 流程图或实现注释：

- README 说明用户如何使用。
- Mermaid 流程图说明 CLI 的执行链路。
- ADR 说明为什么这样划分边界，以及后续修改需要遵守什么约束。

## 编号规则

- `0000-template.md`：通用 ADR 模板。
- `<subsystem>/README.md`：子系统 ADR 索引，例如 `qeflow/README.md`。
- `<subsystem>/0001-*.md` 起：子系统内正式决策记录。
- 新 ADR 默认在所属子系统目录内递增编号，不改写历史 ADR 的结论；如果决策变化，新增 ADR 并在旧 ADR 标记 superseded。

## 状态

- `Proposed`：已提出，尚未落地。
- `Accepted`：已接受，当前代码应遵守。
- `Superseded`：已被后续 ADR 替代。
- `Deprecated`：决策仍有历史价值，但不再推荐用于新代码。

## 子系统索引

- [qeflow](qeflow/README.md)：Quantum ESPRESSO 工作流架构决策。

后续新增分区建议：

- `vaspflow/`：VASP 工作流架构决策。
- `substitutionflow/`：结构替换与批量调度架构决策。
