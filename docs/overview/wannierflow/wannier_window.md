# wannier_window：分析 Wannier 能量窗口

[返回家族索引](README.md)

## 功能与入口

从能带能量文件报告带号／能量范围，或采用指定 AiiDA 工作流默认的能量冻结＋投影解纠缠方案计算窗口。

等价入口：`wannier90 window`。

## 使用方法

```bash
wannier_window EIGENVAL -n 1 8
wannier_window data-file-schema.xml -e -5 5
wannier90 window data-file-schema.xml --num-wann 5
wannier_window EIGENVAL --num-wann 5 --fermi 7.5
```

能量文件为必填位置参数；`-n`／`-e` 保留只读分析。自动选择使用 `--num-wann N`，QE NSCF XML 自动读取本征值与 Fermi 能，EIGENVAL 需显式绝对 `--fermi`；排除带可用 `--exclude-bands 1,2`。旧 `--auto nbnd1 num_wann nbnd3` 已弃用并明确报错，带号启发式实现已删除。

## 输入、输出与衔接

只读分析可读 EIGENVAL 或解析器支持的 QE／FLEUR XML。默认组合策略共用 `qf-wannier`，输出 JSON：固定上游版本、能谱指纹、能量平移、请求上界、态数上限、最终参数和逐点能量态数。默认投影阈值为 0.01／0.95，冻结上界从运行时平移值＋2 eV 出发再按上游规则限制；另外三个未设置边界保持原生默认。

## 使用边界

不会改写 `.win` 或执行 Wannier90。自动读取 XML 当前仅支持非磁性单通道 QE；空数据、错误排除带、态数不足或非法窗口会拒绝。能量冻结态数不包含原生投影冻结结果；选择 JSON 不是插值科学质量证明。初始化通过同一共享规则生成实际输入，不需将已经平移的 JSON 数值再按相对覆盖填写。

## 实现依据与关联

[CLI 定义](../../../wannier90flow/src/bin/wannier90.rs)；[配置模板](../../../wannier90flow/src/config.rs)；[入口实现](../../../wannier90flow/src/window.rs)；[共享策略](../../../crates/qf-wannier/src/lib.rs)。回归与指定上游实际函数差分证据见 [0016 验收](../../specs/0016-wannier-window-selection/acceptance.md)。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
