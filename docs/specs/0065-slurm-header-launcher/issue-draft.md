# 全仓库统一使用 Slurm header 与 launcher

唯一类型：`type:feature`。0065 为本地暂定编号，沿用线上 Issue 用户管理约定；远程链接待验证。关联 [Spec 草案](spec.md)。独立跨模块 Feature，无已确认父 Issue。

## 原始需求记录

```text
我建议
[resources]\
partition = "local"\
nodes = 1\
mpi = 2
彻底放弃这个参数， mpi 只通过 launcher 启动。slurm 脚本的内容直接采用toml 里面读入相关的字符串，就能设置 slurm 的 ## 标志的文件头。

这个要求统一到所遇到 dftflow 里面的代码里面。

后续实施授权：

每次编写代码，都直接覆盖旧的编译的结果，或者删除旧的编译好的命令，然后重新编译，不要记录版本，直接覆盖，只保留一份编译结果。

开始修改。

后续答复：

统一采用QE／VASP 等模块已经支持 [slurm].header的方案。
```

## 目标与范围

删除 EPW `[resources]`，统一采用已有 `[slurm].header` 原生文件头字符串及 launcher 启动方式。范围包括配置、各新生成脚本、sbatch 调用、并行和恢复检查、作业登记、测试、活动示例及使用文档；现有 QE／VASP 等接口保持已有结构。具体验收为 Spec 的 SC-001～006。

## 范围外

不修改历史计算数据、冻结快照或科学参数；不自动操作远端、部署和提交真实科学计算。

## 依赖与关系

复用 qf-config 的 Slurm header 及共享脚本执行组件；与 0058 资源限制移除、0064 阶段恢复衔接，不改写历史验收结论。

## Documentation Impact

- [x] 更新受影响 docs/overview/ 模块说明与共享约束
- [x] 完成当前 Spec、Plan、Tasks 和验收证据
- [x] 更新根 README.md 和 Ubuntu MgB₂ 使用说明

## 关闭条件

SC-001～006 与文档门完成；线上状态与远程链接由用户管理。用户确认统一 slurm.header 并授权开始修改；实现与验证证据见后续验收，不将文档确认当成实现通过。

实现和本地验证见 [验收记录](acceptance.md)，线上状态与远程链接仍由用户管理。
