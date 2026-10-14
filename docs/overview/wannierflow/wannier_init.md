# wannier_init：初始化 Wannier90 流程

[返回家族索引](README.md)

## 功能与入口

根据结构与 [wannier_init] 配置生成 .win、接口输入和运行脚本。

等价入口：`wannier90 init`。

## 使用方法

```bash
wannier_init -c inputwannier.toml -i POSCAR
# 已有电子计算数据和计算资源后
wannier_init -c inputwannier.toml -i POSCAR -r
```

-c/--config 必填，支持 TOML/JSON；-i 默认 POSCAR；无执行标志只准备，-r 运行，-s 提交，选择一种。

## 输入、输出与衔接

在当前目录的 wannier90/ 下写输入；[wannier_init] 指定 system_name、num_wann、num_bands、projections 与 k_points。脚本顺序为 wannier90.x -pp → pw2wannier90.x → wannier90.x；[pw2wannier90] 指向已有 QE prefix/outdir。缺少 KPATH 时尝试 VASPKIT。

## 使用边界

不会替用户计算 QE SCF/NSCF；波函数、k 网格与 .win 必须匹配。DFTFlow 家族命令叫 wannier90，外部求解器叫 wannier90.x。

## 实现依据与关联

[CLI 定义](../../../wannier90flow/src/bin/wannier90.rs)；[配置模板](../../../wannier90flow/src/config.rs)；[初始化实现](../../../wannier90flow/src/init.rs)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
