# Gitee Issue 记录

- 标题：新增 scflow：将 A2F QE 超导后处理集成 dftflow
- 类型：`type-feature`
- 目标：复制 A2F 全部计算代码为独立 scflow 包，读取 input.toml，处理 QE EPC 输出并获得 Tc。
- Spec：本目录（编号 0001）；远端 Issue：[IKHZT0](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKHZT0)。
- 范围外：不运行 QE、不修改 external/a2f、不改动既有 qeflow 工作流。
