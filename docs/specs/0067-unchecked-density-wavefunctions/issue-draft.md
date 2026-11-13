# 取消电荷密度和波函数核对并统一历史记录行为

唯一类型：type:feature。0067 为本地暂定编号；线上状态由用户管理，未操作当前不匹配的 origin。

## 原始需求记录

当前要求：

> SCF 快照，包括电荷密度和波函数	13 个文件，共约 3.2 MB 这些东西不需要核对，我强烈要求电荷密度和波函数不需要核对。 你来帮我用 issue 和 spec 来改正代码。把之前的那些要求也都写到 issue 和 spec 里面。

本次范围澄清：

> 本次只取消核对，目录保持现状

之前相关原话（保留原意所对应的完整消息；不把提问改写成实施授权）：

> 我想测试 epwflow 计算 MgB2的效果。我想请自测试。你要/home/mayuan/code/dftflow/examples/MgB2里面新增一个目录叫 ubuntu, 我要在这个目录里面开展  input.toml也写在 ubuntu 这个目录里面。只git 保留这个输入文件和相应的 readme.md的文档，其他计算数据都不跟踪

> 激活编译环境的命令都放到 \~/bin/ 里面, 修改 toml 和 md 文件

> 我建议
> [resources]\
> partition = "local"\
> nodes = 1\
> mpi = 2
> 彻底放弃这个参数， mpi 只通过 launcher 启动。slurm 脚本的内容直接采用toml 里面读入相关的字符串，就能设置 slurm 的 ## 标志的文件头。
>
> 这个要求统一到所遇到 dftflow 里面的代码里面。

> 统一采用QE／VASP 等模块已经支持 [slurm].header的方案。

> 每次编写代码，都直接覆盖旧的编译的结果，或者删除旧的编译好的命令，然后重新编译，不要记录版本，直接覆盖，只保留一份编译结果。
>
> 开始修改。

> 当前机器是 12 个核，是吗。我建议所有的计算，用 12 个核并行计算。这个要求记忆写到文档里。

> 为什么  epw_phonon -c "input.toml" --stage scf -s 执行后，多套了一层 run 的目录，为什么不是直接展开 各个步骤的目录。

> 帮我解决一下，我觉得老赝势应该也是可以读取的

> campaign/ 什么作用

> 我可以删除 repair 吗？

> $ epw_phonon -c "input.toml" --stage nscf -s\
> 复用：1.qe-scf
>
> 我想知道这里的  复用：1.qe-scf  都在做什么操作。 [$big-project-investigator](/home/mayuan/.codex/skills/big-project-investigator/SKILL.md)

> **计算条件**：当前与上次 SCF 的结构、赝势、\`pw\.x\` 内容，以及截断能、网格等参数是否一致。其中部分比较需要重新读取文件、计算哈希。 这里这个计算 hash，是不是又要计算很多大文件，然后拖慢速度？

## 目标、范围与关联

按 [Spec](spec.md) 的 FR-001～008／SC-001～004 交付。关联 0041、0064、0065、0066；只替代电荷密度和波函数相关核验，不撤销其他科学检查。本机 12 路并行、header／launcher、固定覆盖构建、Ubuntu 仅纳管输入与 README 继续有效。用户明确目录保持现状。

## Documentation Impact

- [x] 更新 EPW／QE 当前说明和 Ubuntu 手册
- [x] 完成当前 Spec、Plan、Tasks 与验收
- [ ] 线上 Issue／远程链接仍由用户管理，不自动关闭或推送

实现与验证见 [验收](acceptance.md)：420 回归通过、15 既有 ignored；真实复用的原始大数据打开次数 3934→0。线上关闭与链接未操作。
