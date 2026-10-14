# mattersim_opt：机器学习势结构预优化

[返回家族索引](README.md)

## 功能与入口

按 [mattersim_opt] 阶段设置生成和运行 MatterSim/ASE 优化。

等价入口：`Python 入口 mattersimflow.opt.mattersim_opt:main`。

## 使用方法

```bash
mattersim_opt -c input.toml -i POSCAR
mattersim_opt -c input.toml -i POSCAR -p 0 10 -s
```

-c 默认 input.toml；-i 为结构；-p 多个压力，单位 GPa；无 -r/-s 只准备，-r 运行，-s 提交。

## 输入、输出与衔接

写 POSCAR、run_mattersim_opt.py、run_mattersim.sh；运行后输出 CONTCAR、stageN_relaxed.vasp、mattersim_opt.log/json。无 -p 时使用 0 GPa 和 mattersim_opt 目录；有 -p 时写各压力目录，多压力另生成批脚本。

## 使用边界

保留 Python，主 Rust 安装器不安装它；需 MatterSim、ASE 等依赖及模型。支持 FIRE/BFGS 和 ExpCellFilter/FrechetCellFilter；每阶段需 ionic_steps、fmax。预优化不替代 DFT 验证。

## 实现依据与关联

[CLI/runner](../../../mattersimflow/opt/mattersim_opt.py)；[包入口](../../../pyproject.toml)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
