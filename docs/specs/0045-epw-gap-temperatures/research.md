# 0045 依据与取舍

## 已核对的主路径

- 当前 EPW 源码 `EPW/src/supercond_iso.f90` 第 77 行和 `supercond_aniso.f90` 第 134 行均循环 itemp=1..nstemp，支持在配置温度求解。相应代码调用 mix_wrap 并传递 broyden_beta。
- DFTFlow `superconductivity/attempt.rs::render` 已生成 nstemp=1、temps(1)、模式开关、非线性初值、混合等原生参数。当前不是缺少 EPW 方法，而是公开配置和调度强制 Tc。
- `foundation/config.rs` 将 Tc 表、双初值和温度范围绑定；`engine.rs::run` 先搜索，`verify.rs` 与 `delivery/native.rs` 仍把完成定义为 Tc 定位及双初值。需要共同适配，不能只向 TOML 增加温度列表。
- `foundation/stages.rs` 第 7 步身份含首个温度，变温度不能先承诺完全复用旧细网格。
- [0015 真实证据](../0015-epwflow-mgb2-real-validation/temperature-points-validation.md) 已有六点正常结束，但为独立原生调用，不能作为本功能公开入口验收。

尝试访问 EPW 官方在线输入说明时，浏览工具出现流解码错误；本轮依据本机实际运行版本的源代码及真实输入，不声称已核验在线文档内容。

## 设计取舍

采用显式 task=gap 与既有 mode、gap_temperatures_k，用户能区分“按列表算能隙”和“搜索 Tc”；不隐式改变旧配置语义。单初值单独配置，避免隐藏选取旧双初值数组某一项。20／30／40 K 只是例子，不作为程序常量。

保留一次原生调用一个温度的执行方式，适配已存在的逐任务提交和续算；不依赖 EPW 多温度调用的跨温度初值继承。允许原生正负混合系数，不隐藏改参或为每模式新增无依据的参数扫描。

执行结束与有效能隙分开：遵守用户要求的正常结束验收，同时避免把退出 0 但 NaN／未收敛的输出发布为有效物理数据。Tc 仍使用原来的严格验证。指定温度任务无需构造虚假的 Tc 区间。

实现和真实计算均尚未开始；本机无 Gitee CLI，线上用户管理，不创建伪线上编号。
