# vasp_lobsterin：准备 LOBSTER 分析

[返回 vaspflow 索引](README.md)

## 功能与当前行为

在已有 VASP 波函数结果基础上创建成键分析目录、lobsterin 与运行脚本。等价入口：vaspflow vasp-lobsterin。

## 使用方法

```bash
vasp_lobsterin -c input.toml --species-custom Mg B -d 1.0 3.0 -e -10 5
# 在同一 VASP 结果目录可加 -r 运行，或 -s 提交
```

--species-custom 两个物种；-d/--d-limit 距离上下限；-e/--cohp-energy 能窗；-z/--zvalances、--custom-basis 设置基组；--orbital-wise、--k-space-cohp 控制输出；-m/--mode 默认 5。

## 输入、输出与衔接

以物种与距离组成子目录名；链接/复制 WAVECAR、CONTCAR、KPOINTS、OUTCAR、POTCAR、vasprun.xml、POSCAR，生成 lobsterin 与 run_lobster.sh。

## 使用边界

先完成适用的 VASP 计算并配置 LOBSTER 执行命令；本工具不生成缺失的 VASP 波函数。

## 实现依据与关联

[CLI 定义及别名映射](../../../vaspflow/src/bin/vaspflow.rs)；[核心实现](../../../crates/df-vasp/src/lobsterin.rs)；[VASP 库覆盖](../../../crates/df-vasp/tests/vasp.rs)、[后处理覆盖](../../../crates/df-vasp/tests/process.rs)。覆盖文件的存在不代表本次重新验收。

关联 [0004 Rust 迁移](../../specs/0004-dftflow-rust/spec.md)。
