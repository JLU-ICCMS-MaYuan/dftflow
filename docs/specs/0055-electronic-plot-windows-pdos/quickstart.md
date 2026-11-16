# 绘图配置与验证

    [plot]
    electronic_energy_min_ev = -3.0
    electronic_energy_max_ev = 3.0

上下限可独立修改，例如 −1.5 与 4.0；相对 E_F，省略的单项使用其默认值。必须有限且下限小于上限。

第 3、5 步 plot/ 各有 electronic.png 和 electronic-from-lowest-to-highest.png；第 4、6 步 plot/ 为 phonon.png。results 中按相同阶段与 plot/ 发布。逐带范围与实空间衰减为独立诊断。

使用已有阶段恢复入口补图；现场第 5 步可运行 epw_wannier -c input.toml --plot；移动后的结果包运行 epw_wannier --plot --results PATH。重绘不运行科学程序。

## 立即重绘第 3、5 步

在 examples/MgB2/ubuntu 目录中运行：

    epw_phonon -c input.toml --stage electronic --plot
    epw_wannier -c input.toml --plot

分别输出 run/3.qe-electronic/plot/ 和 run/5.epw-wannier/plot/ 的两张组合图。旧工作区保存的分离图参会被定向迁移，不调用科学程序、不提交作业；现场重绘不改写独立 results 包。缺少已有绘图数据时报告缺项。
