# scflow：QE 电子声子耦合超导后处理

[返回家族索引](README.md)

## 功能与入口

按展宽计算 direct/interp 两路 λ、频率矩、McMillan/Allen-Dynes/Eliashberg Tc 及派生量。

等价入口：`scflow run（也可用 qeflow scflow）`。

## 使用方法

```bash
scflow -c input.toml
# 等价显式子命令
scflow run -c input.toml
```

-c/--config 指定含 [scflow] 的配置，建议显式传入；无 -r/-s，调用即本地后处理。

## 输入、输出与衔接

读取根目录的 *.dynN.elph* 或 elph_dir/elph.inp_lambda.N（两种布局不能混用），利用 dyn 文件或声子日志获取 q 权重；默认输出 work_dir/results。按展宽写 CSV、result.json、PDF 及汇总 CSV/PDF；ph.out 结构可解析时导出结构/CIF。

## 使用边界

自动发现的声子日志名称为 *.ph.out、ph_no_split.out、split_ph.out.N；单独名为 ph.out 的文件不会被当前发现器选中，需按支持的布局准备。生产 CLI 不能仅用 a2F.dat；库内单曲线 solve 没有独立 CLI。缺失权重会回退均匀权重，须检查日志；Eliashberg 搜索边界状态不能直接当作有效物理 Tc。

## 实现依据与关联

[CLI](../../../scflow/src/bin/scflow.rs)；[核心](../../../crates/qf-scflow/src/lib.rs)；[配置](../../../crates/qf-scflow/src/config.rs)；[数值对照覆盖](../../../scflow/tests/scflow_parity.rs)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。

## 配置示例

```toml
[scflow]
work_dir = "qe_epc_results"
results_dir = "sc-results"
mu = 0.10
integration = "sum"
resolution = 1000
smoothing_thz = 3.0
gaussian_sigma = 5.0
structure_tolerance = 0.2
```

work_dir、results_dir 相对配置文件目录解析；integration 支持 sum/simps。不同 μ* 的分析应保留独立目录。普通声子谱没有 EPC 耦合量，不能直接推导 Tc。历史物理验证范围见 [0001](../../specs/0001-scflow-qe-superconductivity/spec.md)。
