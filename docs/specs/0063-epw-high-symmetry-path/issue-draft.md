# 0063：第 5 步 Wannier 路径仅声明高对称端点

唯一类型：`type:feature`。创建日期：2026-10-27。状态：本地实现、自定义文件头及失败归档交接续修完成，正式 CLI 在真实 SiP₂ 目录准备验证通过；线上状态仍由用户管理。

沿用仓库线上 Gitee 由用户管理的约定。0063 是本地暂定编号，未创建线上 Issue，也不表示平台已分配该编号。

## 原始需求记录

### 2026-11-16 比较图与采样要求

> 1. 删除比较图上的紫色误差点、连线和相关图例。
> 2. QE／EPW 沿相同的物理高对称路径，各自使用自己的采样密度；不要求内部 k、q 点逐一重合。
> 3. 不为计算逐点差值而补算、重采样或阻止出图；没有共同点时，不强行计算差值。
>
> 修改相应的 issue 和 spec，然后开始修改代码。


> 5.epw_wannier那一步里面的高对称路径的写法你可以参考/data/home/mayuan/code/dftflow/test_auto/MgB2/epwflow/Wed.9.Mori.pdf里面的方法，把 kpath.in改造成只写高对称点的方法。不需要你把每个插值的点都写进来。
>
> 新建 issue 和 spec 来修改这个问题。

> 开始执行。

> 没关系，如果有计算相同坐标，就计算能量差，没有就不算。我人眼来判断拟合的时候好。dos 可以继续保留。也不要注明共同点覆盖率。

> $ epw_wannier --config "epwinput.toml" --stage wannier --resume -s
> reuse: scf
> reuse: nscf
> reuse: electronic
> reuse: electronic/projwfc
> data_invalid: data_invalid: /data/home/mayuan/code/dftflow/examples/SiP2/KPATH.in:1 应为 kx ky kz label
> 这个报错是什么意思？？？？？

> 你看看，我记得有一个 issue 专门用来解决 5.epw-wannier这一步里面为 wannier 写高对称路径的。你当时没测试好吗？为什么还是报错了？在那个 issue和 spec 里面继续完成这个debug

> (my_scripts) [mayuan@login02 SiP2]$ epw_wannier --config "epwinput.toml" --stage wannier --resume -s
> input_conflict: input_conflict: 无法定位 /data/home/mayuan/code/dftflow/examples/SiP2/5.epw-wannier/epw_wannier.in:No such file or directory (os error 2)
> (my_scripts) [mayuan@login02 SiP2]$ epw_wannier --config "epwinput.toml" --stage wannier  -s
> input_conflict: input_conflict: 无法定位 /data/home/mayuan/code/dftflow/examples/SiP2/5.epw-wannier/epw_wannier.in:No such file or directory (os error 2)这是为什么？你在/data/home/mayuan/code/dftflow/examples/SiP2测试过了吗？为什么不行，还是报错。你确定你重新编译得到新的 CLI 了吗

> bands_num_points = 1 默认以后都是 50。修改到对应的 issue 和 spec 里面，并且修改 input.toml文件

随附图片的读取状态及 PDF 核查见 [Spec 附件登记](spec.md#附件与推断来源)。

## 问题与目标

当前 `KPATH.in` 已使用高对称端点；冗长的路径来自第 5 步 `electronic_sampling::wdata`：它将展开的 QE 路径点和 NSCF 网格点组合成校准段、人工段，并用 `P{i}/Q{i}` 代替真实标签。仅压缩用户输入文件不能解决生成输入的问题。

目标是让用户只维护高对称点与连线，第 5 步实际传给 EPW 内嵌 Wannier90 的 `begin kpoint_path` 也只包含真实高对称段。中间采样交给程序和原生插值规则，不逐点列入该块。

## 范围

- 以 Mori 讲义第 4 页的“标签＋分数坐标＋标签＋分数坐标”作为生成格式依据，保留当前材料的路径、标签、顺序及断段。
- 沿用现有 `kpath` 配置和仅含端点的 `KPATH.in` 输入，不要求用户逐点填写，不强制重命名文件或更换现有端点文件格式。
- 移除生成路径中的人工校准、逐点拼接和网格覆盖段；路径行数由真实连线数决定。
- 同步处理 QE／Wannier 能带配对与 DOS 采样，保持真实坐标检查、来源证据及第 5 步结果发布能力。
- 保持 EPW 内嵌 Wannier90 生成能带、仅电子 DOS 允许 postw90 的既有边界。

## 验收标准

1. MgB₂ 现有路径生成 9 条真实高对称连线，含 L–M、H–K 断段，无 `P0/Q0` 等人工标签或 NSCF 网格段；改变插值密度不改变路径行数。
2. 生成的 `kpoint_path` 与源文件的标签、坐标、顺序及断段逐段一致；不会将讲义 Pb 的 G–X–W–L–K 路径套用到 MgB₂。
3. 不等长路径段的真实生产入口受控验证通过；QE／Wannier 比较核对实际坐标，没有共同坐标时不计算该点能量差，不显示共同点覆盖率；不缩放横轴伪造配对，不以线性插值后的能量冒充原生求值。
4. 电子 DOS 保持独立均匀网格及相同物理口径，不能因删除人工段而缺点、停留在仅能生成输入的状态或静默失去比较结果。
5. 非法端点、空路径、非有限坐标、零长度段及原生输入容量／行长不足明确拒绝；已有成功拟合和历史数据保持可核验，不自动重做 SCF／NSCF／PH 或拟合。

## 依赖与关系

关联 [0050 电子比较前置](../0050-epw-stage-results/issue-draft.md)、[0061 平层配置](../0061-epw-flat-stage-config/issue-draft.md)和 [0060 NSCF 网格](../0060-epw-nscf-grid-contract/issue-draft.md)。本项局部替代 0050 的人工拆段／校准路径表达；不改变 NSCF 完整网格要求，不新增父子关系。

## 范围外事项

自动寻找高对称路径、替换材料路径、修改电子能窗／科学阈值、重构声子路径接口、增加独立 Wannier90 能带执行路线、自动重新计算或推送均不在本项范围。初次实施未安装；本次继续调试沿用会话已授权的正式 CLI 更新要求，发布证据另记，不提交真实计算。

## Documentation Impact

- [x] 更新 Spec、Plan、Tasks、接口契约、一致性分析和验收证据。
- [x] 已更新 `docs/overview/epwflow/workflow.md` 中拆段／校准及路径采样说明，按需更新配置手册。
- [x] 已更新 MgB₂ 示例与测试说明，给出仅声明高对称端点的输入／生成结果。
- [x] 核对根 README：公开入口与导航不受影响，依据见验收记录。
- [ ] 线上类型、关系和 Issue／Spec 双向远程链接由用户管理，当前未核验。

## 关闭条件

上述验收和 [Spec](spec.md) 的 SC 全部取得可复查证据，路径简化与比较／DOS 同时有效，相关当前功能文档与最终实现一致。生成短输入、文档提交或受控替身通过均不单独代表科学验收。

## 最终草案确认

用户后续明确答复“开始执行。”，本轮据此落地端点路径目标。共同采样点差值统计按后续用户原话执行，线上写入及关闭仍由用户管理。

## 自定义文件头回归

继续在本项下修复 SiP₂ 已有四行文件头的自定义标题兼容遗漏，不新建 Issue。初次受控测试只覆盖固定 VASPKIT 标题和裸端点，漏测真实 SiP₂ 标题；严格解析器因此在第 1 行拒绝。目标为完整识别文件头、保留正文严格校验及原始行号，保持用户 KPATH 和已有电子来源身份不变。对应 FR-011／SC-007、T008～T010；验证及发布结果追加到 [验收记录](acceptance.md)。线上 Gitee 仍由用户管理，本地修复不代表线上已关闭。

## 归档后悬空活动记录续修

已发布 CLI 的哈希与发布记录一致；用户此次报错源于更早的失败任务已归档，而旧活动记录仍引用原输入路径。上一轮受控测试从成功模型改参数恢复，漏测“旧模型失败后再次输入生成失败”的状态。继续在本项下补 FR-012／SC-008 和 T011～T013，修复活动记录归档交接并严格接续遗留状态。必须在真实 SiP₂ 目录验证恢复与准备，不以只读路径生成或替身成功代替现场验收；实际作业仍由用户提交。

本次续修已完成：68 个不同测试通过，真实失败记录经生产 API 核验接续，正式 CLI 在原目录退出 0 并保持同一 `prepared` 任务；新输入为 `s,p`／96 函数／500 次，旧失败记录原样归档、前置数据复用。4 个正式 CLI 已重编译安装；详见 [验收记录](acceptance.md)。未提交真实作业，不把准备成功当作拟合收敛或线上关闭。

## 2026-11-16 默认采样设置补充

按上述用户要求，`bands_num_points` 的用户示例默认值由 `1` 改为 `50`。同步 MgB₂ Ubuntu `input.toml` 与 SiP₂ `epwinput.toml`，保留用户显式调整能力及原生按段长比例取点规则；不改变 QE 的 `points_per_edge`，不重写历史输入或触发重算。

验收对应 Spec 的 FR-013／SC-009、T014：两份示例值均为 `50`，中文注释准确，TOML 解析与文档一致性检查通过。Documentation Impact：更新本 Issue、Spec、Plan、契约、快速说明、任务与验收；当前功能架构、公开命令和根 README 不变，无需更新 Overview。本地维护不表示线上 Issue 已更新或关闭。

## 2026-11-16 与 0043 的范围衔接

用户已授权实施；[0043 最新要求](../0043-epw-comparison-max-differences/spec.md) 将本需求已有的电子独立采样边界扩展到常规声子比较，并取消全部比较图的紫色误差标记。FR-005／SC-003 的共同点统计继续有效；不得将旧版“声子仍按原有口径”解释为继续强制内部 q 点一致。物理路径、端点、分段和每份原生输出自身有效性仍核对，不为差值补算或重采样。首段 bands_num_points 默认 50 的既有决定不变。本轮实施与验收统一见 0043，不操作线上 Gitee。
