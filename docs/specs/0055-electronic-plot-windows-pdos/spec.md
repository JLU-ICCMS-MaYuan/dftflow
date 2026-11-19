# 0055：统一能带与 DOS 组合图

> **协作入口（2026-11-17）**：[GitHub Issue #55](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/55)。Issue 已在 GitHub 登记，协作状态以 GitHub 为准；本文件保留设计与历史验收，旧的“本地草案／线上自管／未发布”说明是迁移前语境。Spec 远程链接待发布及验证。

**Issue**：[GitHub Issue #55](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/55)，沿用线上用户管理约定。

## 当前有效需求

本次规则替代旧 ±5 eV、最低占据态至 E_F＋5 eV、第 5 步默认实际外窗、分离图片及重复总览要求。历史原话与验收保留用于追溯，不再作为新生成端的验收规则。逐带范围与实空间衰减属于独立诊断，继续保留。

- **FR-001**：第 3、5 步各生成两张电子组合图：electronic.png 与 electronic-from-lowest-to-highest.png；各自放在阶段 plot/，并即时发布到 results 对应阶段的 plot/。
- **FR-002**：左侧能带、右侧 DOS，纵轴均为 E−E_F（eV），绘图区上下边界严格对齐；DOS 横轴为态密度。
- **FR-003**：input.toml 的 [plot] 提供 electronic_energy_min_ev、electronic_energy_max_ev，共享于第 3、5 步，分别默认 −3、＋3；有限且严格递增，可独立设置。
- **FR-004**：全范围取 QE 实际路径全部能带的最低、最高能量，第 5 步同样以 QE 为准；不使用最低占据态、NSCF 极值、DOS 网格或实际 Wannier 外窗替代。
- **FR-005**：第 4 步全部声子模式与第 6 步生成 phonon.png，放在阶段 plot/；左侧声子谱、右侧 DOS，共用频率纵轴；范围为 0 到图中最高路径频率，第 6 步取 QE／EPW 最大值。
- **FR-006**：负频原始数据、虚频判断、科学核验门保留。模型或 DOS 未覆盖区域留空，不补零或外推数据。
- **FR-007**：QE 实线先画，EPW 蓝色虚线后画；保留元素投影语义、高对称点符号、费米线、四窗口标注、独立诊断和至少 300 DPI。沿用最新约定，不恢复紫色误差标记。
- **FR-008**：组合图等待对应能带与 DOS 数据就绪；DOS 失败保留科学数据与独立逐带范围诊断，不发布缺失 DOS 的组合图。
- **FR-009**：绘图范围只参与派生绘图身份；改变范围、删除图片、结果包移动后的补图不调用科学程序。
- **FR-010**：新生成端不再要求分离图、-valence 图或重复比较总览；仅清理由既有生成器或交付清单拥有的旧产物，不删除科学数据或未知文件。

## 用户场景

### US1：电子双范围组合图

完成第 3 步即可在同一能量轴上检查 QE 能带和 DOS；第 5 步使用相同范围比较实际 QE／EPW 数据。选择非对称范围不改变科学输入。

### US2：声子组合图

第 4 步观察 QE 色散及元素 DOS，第 6 步对比 QE／EPW 色散与总 DOS。负频仍参与原科学判断。

### US3：复用、补图与交付

图片缺失或绘图参数改变时，只重绘派生图片；结果包仅依赖包内自包含图参重绘，保留来源与数据核验。

## 成功标准

- **SC-001**：第 3、5 步分别仅有两张电子科学组合图；第 4、6 步分别一张声子组合图，目录与命名满足 FR-001／005。
- **SC-002**：默认 ±3 eV、独立非对称上下限、非法范围均有验证；电子全范围可从 QE 路径独立复算。
- **SC-003**：左右面板绘图区共享纵轴像素坐标，DOS 方向、线型、端点与窗口标注正确；真实 PNG 可读且至少 300 DPI。
- **SC-004**：失败恢复、配置改变、缺图修复、移动结果包重绘不新增科学调用；相关进程回归通过。
- **SC-005**：旧管理产物定向清理、逐带范围及衰减诊断保留；文档更新、编译覆盖固定入口并按仓库规则精确提交。

## 边界与确认

不改变科学数据、网格、选窗、投影算法、费米参考或质量门。不操作线上 Issue、不推送、不创建或切换分支。沿用本地 0055 追踪，0047／0050 协调同一交付契约。

用户澄清原文：

> 两种范围各一张（推荐）

> 仅取 QE 能带极值

用户已通过 PLEASE IMPLEMENT THIS PLAN 明确授权实施组合图方案。

## 第 3、5 步现场纯重绘补充

- **FR-011**：epw_phonon --stage electronic --plot 和 epw_wannier --plot 分别读取当前 input.toml 的 [plot]，仅重绘相应阶段。已有组合图参直接使用；旧工作区从保存的分离图参定向迁移，不调用 QE、EPW、postw90 或 Slurm。
- **SC-006**：旧分离图参现场可经上述两个入口生成各两张组合图；非对称范围生效，第 3 步命令不提前改第 5 步，科学程序、后处理与提交次数均不增加。缺少已有数值时明确报告，不自动补算。

## 原始需求记录

### 现场重绘补充

> 帮我调整`input.toml` ，我现在就要试一下 重新画图 3,5 步的图

### 本次需求

```text
electronic-bands-valence.png最低能级到最高能级，名字也改为 from-lowest-to-highest

electronic-bands.png 费米能级上下 3 eV, 这个参数可以从 input.toml里调整，上限下限都可以调

electronic-dos-valence.png 最低能级到最高能级，名字也改为 from-lowest-to-highest

electronic-dos.png 费米能级上下 3 eV, 这个参数可以从 input.toml里调整，上限下限都可以调

我希望只画 2 个图，即最低能级到最高能级和费米能级上下 3 eV。把电子能带和态密度合在一起，画在一个图里，共用一个能量纵轴。



3.qe-electronic 里面的图只画一个就行，plot 目录放在 3.qe-electronic里面



4.qe-phonon-index 同理，声子谱和声子态密度画到一个图里，共用一个频率纵轴。



5.epw-wannier 同理，即使绘制对比图，也要共用同一个纵轴。而且只绘制费米能级上下 3 eV和最低能级到最高能级两种能量范围，而且绘制费米能级上下 3 eV这里的 3eV和前面一样是可调节的。



6. 6.epw-coarse这里绘制声子谱的对比图也是一个道理，但是声子谱不需要绘制上下3eV，只需要从 0 到最高频率即可。





修改之前的关于绘图的 issue 和 spec 的设置。清理掉和我当前设置矛盾的要点，按照我的要求，落地代码。
```

### 历史原话及附件记录

下列记录保留原文，后续冲突以当前有效需求为准。附件 IMG-001 仍仅有历史聊天预览，原文件未归档。

```text
你的电子能带结构画的太差了。我建议 默认 画费米能级附近上下 5 eV 内的能带 和最低价带到费米能级往上 5eV的范围内的能带。

为什么你画的电子态密度有这么多的小黑点，而且没有花元素投影态密度呢？

是不是你设置的 dpi 太低了，我要求最起码 300 dpi 的清晰度。而且态密度的能量范围和能带一直。你总结一下我要你画几个图。使用 issue 和 spec 新建 issue 来完成。
```

```text
QE 元素 PDOS；第 5 步只比较总 DOS（推荐）
```

```yaml
images:
  - id: IMG-001
    upload_order: 1
    original_name: null
    locator: null
    status: preview_only
    user_context: "为什么你画的电子态密度有这么多的小黑点，而且没有花元素投影态密度呢？"
    related: [US1, US3, FR-007, SC-003]
    sha256: null
    media_type: null
    dimensions_px: null
    verification: "2026-10-26：可见红框标出的短线状黑点；原字节不可读取，不能用当前工作区 PNG 冒充上传图。"
```

```text
开始落地。
```

```text
我希望你一定要对所有的电子能带和dos 图绘制费米能级。费米能级使用红色绘制，epw 或者 wannier 的插值使用蓝色。其他的线或者 label标色不能使用这两种颜色了。
```

```text
我希望即使他们的线重叠在一起来，也能看到两个颜色，你可以一个线用长一点的打断的虚线，一个用短一点的打断的虚线。而且被压在下面的虚线（一般来说是先绘制的）可以稍微粗 30%。这样他在底部也不会被挡住。
```

```text
你为什么把 epw 和 qe 对比的那个能带图也按照这个要求改了。这是不对的，在这个图里面qe 的计算结果就是要实线的。
```

```text
/data/home/mayuan/code/dftflow/test_auto/MgB2/local_run/014-electronic-plots/band-solid-2r9ljckd/results/5.epw-wannier/electronic-dos-valence.png 这个里面的 qe 为什么是虚线？我要求：图里面qe 的计算结果就是要实线的。

/data/home/mayuan/code/dftflow/test_auto/MgB2/local_run/014-electronic-plots/band-solid-2r9ljckd/results/5.epw-wannier/electronic-dos-valence.png
/data/home/mayuan/code/dftflow/test_auto/MgB2/local_run/014-electronic-plots/band-solid-2r9ljckd/results/5.epw-wannier/electronic-dos.png
都是一样的问题。
```

```text
/data/home/mayuan/code/dftflow/test_auto/MgB2/local_run/014-electronic-plots/band-solid-2r9ljckd/results/5.epw-wannier/electronic-bands.png 我还希望这个能带和 /data/home/mayuan/code/dftflow/test_auto/MgB2/local_run/014-electronic-plots/band-solid-2r9ljckd/results/5.epw-wannier/electronic-dos.png 绘制的范围是从 dis_win_min到 dis_win_max。这才对！

下面目录的 qe 为什么是虚线？我要求：图里面qe 的计算结果就是要实线的。
/data/home/mayuan/code/dftflow/test_auto/MgB2/local_run/014-electronic-plots/band-solid-2r9ljckd/results/5.epw-wannier/electronic-dos-valence.png
/data/home/mayuan/code/dftflow/test_auto/MgB2/local_run/014-electronic-plots/band-solid-2r9ljckd/results/5.epw-wannier/electronic-dos.png
都是一样的问题。
```

## 2026-11-18：电子 DOS 自适应横轴

本补充采用用户确认的能窗方案，不采用中位数或 98% 分位数方案。沿用 Issue #55；本轮不修改线上正文或状态，不推送。

- **FR-012**：第 3、5 步两张电子组合图的 DOS 横轴共同参考当前 `[plot]` 近费米能窗内所有待绘 DOS 曲线的最大值，加 10% 留白并向上整理刻度；深能级峰不参与近费米窗口的范围统计。
- **FR-013**：全能窗纵轴与原始 DOS 不变；超出横轴的曲线在边界处裁剪并加箭头及实际峰值说明，不把峰压平到边界或修改数值。相邻重叠峰的标记可合并并显示其中最大值。
- **FR-014**：图参保存 DOS 参考能窗，支持移动后的独立重绘；旧图参现场重绘使用当前配置，旧独立包优先取同目录近费米图的能窗，缺失时采用默认 ±3 eV。不读取额外科学来源。
- **FR-015**：参考能窗与可见能窗无共同正 DOS 时明确注明并用当前可见数据设置范围；全零数据提供有效正刻度。声子 DOS 范围规则保持原样。
- **SC-007**：深能级峰从 32.6 增大到更高数值不改变近费米 DOS 横轴；所有待绘曲线及非对称配置均参与正确判断。当前 MgB₂ 近费米峰约 4.24 时横轴上限为 5。
- **SC-008**：SVG 的 DOS 曲线不越过右侧边界，截断峰有箭头与真实值；原始能量、曲线数组和来源信息不变，旧包及移动包可重绘。
- **SC-009**：编译覆盖固定入口，在用户原结果目录重绘两张指定 PNG；科学输入／输出和 XML 不变，科学程序与 Slurm 提交次数均为零。保留用户线宽修改但不混入本次提交。

### 原始需求记录：DOS 横轴

> /home/mayuan/code/dftflow/examples/MgB2/ubuntu/run/results/3.qe-electronic/electronic-from-lowest-to-highest.png 图里面 dos 的横轴设置的 dos 范围太宽了，不利于看清，我希望能自适应的调节横轴的刻度范围，，你说这个刻度范围以 dos 值的中位数为最大值，是不是合理一些，你还有什么更好的建议吗？

> 我就是觉得被深能级的 dos 影响导致dos 范围必须很大，我就是希望能在 5 附近，怎么才能自适应拿到排除了深能级 dos 值影响的 dos 范围

> 认可。开始修改

确认依据：用户认可上一轮展示的完整规则，即沿用近费米配置能窗，取该窗内待绘 DOS 最大值乘 1.10 后向上整理刻度，完整能量纵轴保留，超出峰用箭头标真实值。分位数说明仅用于解释旧建议，不属于本次实现方案。

附件 IMG-002：用户本轮上传了全能窗 DOS 局部截图（上传顺序 1，问题为深能级尖峰撑宽横轴）；仅聊天预览可读，未取得原始上传文件，不宣称已归档。对应可读取的原生成图片为 examples/MgB2/ubuntu/run/results/3.qe-electronic/electronic-from-lowest-to-highest.png，本轮直接核对其自包含图参。
