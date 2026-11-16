# 统一能带与 DOS 组合图

类型：type:feature。沿用本地 0055，线上由用户管理。

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

## Documentation Impact

- [x] 更新 docs/overview/epwflow/workflow.md 与配置说明。
- [x] 更新 0055 Spec、Plan、Tasks、契约及验收；协调 0047／0050。
- [x] 更新根 README.md 的绘图配置说明。

## 原始需求记录

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

### 历史原始记录

```text
你的电子能带结构画的太差了。我建议 默认 画费米能级附近上下 5 eV 内的能带 和最低价带到费米能级往上 5eV的范围内的能带。

为什么你画的电子态密度有这么多的小黑点，而且没有花元素投影态密度呢？

是不是你设置的 dpi 太低了，我要求最起码 300 dpi 的清晰度。而且态密度的能量范围和能带一直。你总结一下我要你画几个图。使用 issue 和 spec 新建 issue 来完成。
```

```text
QE 元素 PDOS；第 5 步只比较总 DOS（推荐）
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

## 现场重绘补充

第 3、5 步分别提供 epw_phonon --stage electronic --plot 与 epw_wannier --plot，使用当前 [plot] 并定向迁移旧现场的已保存绘图数值；只绘图，不补科学计算。详细要求与完整补充原话见 Spec 的 FR-011／SC-006。
