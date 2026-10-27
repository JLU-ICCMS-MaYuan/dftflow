# 0061：EPW 原生参数使用同层独立配置段

类型：`type:feature`。承接 [0059](../0059-epw-native-stage-config/issue-draft.md)，关联 [0060](../0060-epw-nscf-grid-contract/issue-draft.md)。本地暂定编号，线上 Gitee 继续由用户管理。

## 原始需求记录

> 我希望的是只区分到[stages."1.qe-scf"]就行，至于 1.qe-scf又有什么分类，我觉得就不别再细分，像[stages."1.qe-scf".pw.ELECTRONS]在1.qe-scf的基础上又细分了 2 层，太复杂了。同样的道理也应用到 2.qe-nscf, 3.qe-xxx, 4.xxx, 5.xxx, 6.xxx, 7.xxx
>
> [stages."1.qe-scf".workflow.derived]
> "pw.CONTROL.outdir" = "当前阶段工作目录和已绑定赝势源"
> "pw.CONTROL.pseudo_dir" = "当前阶段工作目录和已绑定赝势源"
> "pw.SYSTEM.ibrav" = "已绑定 POSCAR 生成 CELL_PARAMETERS 和 ATOMIC_POSITIONS"
> "pw.SYSTEM.nat" = "已绑定 POSCAR 生成 CELL_PARAMETERS 和 ATOMIC_POSITIONS"
> "pw.SYSTEM.ntyp" = "已绑定 POSCAR 生成 CELL_PARAMETERS 和 ATOMIC_POSITIONS"
> 像这种自动绑定的也不需要展示出来，就在程序内部确认即可。

> 采用同层独立配置段（推荐）

## 已确认结论

每次科学调用各用一个 `[stages."编号.用途"]`，直接填写原生参数；多个程序以及 iso/aniso 使用同层独立段，保留重复参数的独立设置。自动绑定由程序校验，用户不再填写推导登记表。此次确认替代 0059 的多层展示方式；其原生名称、单位和阶段独立性要求继续适用。用户已授权实施该调整。

## 范围与验收

需求与标准见 [Spec](spec.md)。只调整配置表达、内部解析和参数来源记录；不改变科学设置，不清理或重跑现有计算，不改默认安装。保留 0060 三整数 NSCF 网格契约；不兼容多层配置，不新增迁移工具。

## Documentation Impact

- 更新 Spec、Plan、Research、Tasks、契约、验收记录。
- 实现验证后同步 Overview、模块说明及当前 MgB₂ 算例。
- 线上状态与远程链接由用户管理，本地提交不代表线上关闭。
