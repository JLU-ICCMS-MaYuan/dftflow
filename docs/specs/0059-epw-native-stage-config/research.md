# 设计依据

## 原生名称与单位

本机配套 QE PW/Doc/INPUT_PW.def 和 EPW/src/readin.f90、input.f90 是当前版本依据，直接核对配套源码。ecutwfc/ecutrho/degauss/conv_thr 使用 QE 原生 Ry；EPW degaussw/fsthick/wscut/gap_edge 为 eV，degaussq 为 meV，eps_acoustic 为 cm⁻¹，temps 为 K。原生 Wannier90 的 num_iter、conv_tol、dis_num_iter、dis_conv_tol 不另加别名。

## 内嵌 Wannier90

保留 EPW 调用内嵌库的实际路径，按 Wannier90 原生键表达，再受控编译为 EPW 参数及 wdata；不得让同一值被两处定义。

## 数据所有权

各次科学调用读取自己的阶段配置；数据交接是显式依赖校验，不是参数继承。自动选窗、结构卡片、PH 拆分、搜索温度及工作区路径属于明确登记的推导规则。

## 兼容与并行任务

用户明确拒绝旧配置兼容与迁移。当前工作树另项 0058 取消资源限制，保留其改动，不恢复旧额度；本项只保留累计消费事实。

## 补充实现核对

- NSCF 改为原生显式 `K_POINTS crystal` 点表，避免配置自动网格与实际显式输入不一致；通过点集推导内部网格元数据。
- 共享 qeflow 的 nosplit 执行库增加输入适配钩子，在所有输入生成后、任何科学程序启动前应用 EPW 阶段配置；原 qeflow CLI 默认不传钩子。
- 冻结赝势副本时保留原卡片的相对路径和文件名，避免旧的元素名重命名破坏手填卡片。
- 额外展宽使用完整的原生细网格子任务；每项单独声明参数和推导目标，禁止按主任务补齐缺失科学设置。

本轮再次核对配套 `readin.f90` 的 `degaussw / ryd2ev`、`degaussq / ryd2mev` 转换，配置按原生输入单位传入，不在 TOML 名称或取值上另做单位转换。

## 后续契约变更

0060 明确替代本研究的 NSCF 完整点表决策，改为三个正整数，输出时展开；历史决策保留供追溯，现行规则见 [0060 Plan](../0060-epw-nscf-grid-contract/plan.md)。
