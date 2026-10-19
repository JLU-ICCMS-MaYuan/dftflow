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

在当前目录的 wannier90/ 下写输入；[wannier_init] 指定 system_name、num_wann、projections、k_points，以及指向真实非磁性 QE NSCF XML 的 `bands_file`。相对文件路径按配置文件所在目录解析；带数从源数据及排除带推导，显式 num_bands 必须一致。

选窗与 epwflow 共用指定上游默认组合策略，写入冻结上界、`dis_proj_min/max=0.01/0.95` 和原生投影启用开关 `dis_froz_proj=true`；同时生成同名前缀的 `.window-selection.json`。未显式覆盖的其他边界不补造；显式四界按上游相对运行时平移值解释并继续受态数上限约束。

脚本顺序为 wannier90.x -pp → pw2wannier90.x → wannier90.x，失败即停；[pw2wannier90] 指向已有 QE prefix/outdir。模板空值不再写成 `'None'`。缺少 KPATH 时尝试 VASPKIT。完整配置及迁移说明见 [0016 使用说明](../../specs/0016-wannier-window-selection/quickstart.md)。

## 使用边界

不会替用户计算 QE SCF/NSCF；波函数、k 网格与 .win 必须匹配。缺本征值不沿用旧固定窗；运行／提交前检查实际 W90 的必要参数关键字，无法核验时拒绝，不退回纯能量算法。关键字存在仍不等于原生运行已通过，投影矩阵和实际收敛继续由科学链验证。DFTFlow 家族命令叫 wannier90，外部求解器叫 wannier90.x。

## 实现依据与关联

[CLI 定义](../../../wannier90flow/src/bin/wannier90.rs)；[配置模板](../../../wannier90flow/src/config.rs)；[初始化实现](../../../wannier90flow/src/init.rs)。输入生成、CLI 和脚本失败传播验证见 [0016 验收](../../specs/0016-wannier-window-selection/acceptance.md)。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
