# vasp_opt：结构优化

[返回 vaspflow 索引](README.md)

## 功能与当前行为

生成 rv4、rv1 或 rvf 优化输入，支持一个或多个压力。等价入口：vaspflow vasp-opt。

## 使用方法

```bash
vasp_opt -c input.toml -i POSCAR -m rv4
vasp_opt -c input.toml -i POSCAR -m rv4 -p 0 10 -s
```

-i/--poscar 指定结构；-m/--mode 默认 rv4；-p/--pressure 以 GPa 输入，可多个；-w/--work-dir 指定目录；-r 运行、-s 提交、无标记只准备。

## 输入、输出与衔接

rv4 写 INCAR_1..4 和 fouropt.sh，每步归档 CONTCAR_i、OUTCAR_i、vasp.log_i，再把 CONTCAR 用作下一步 POSCAR；rv1 写 oneopt.sh，rvf 写 fopt.sh。-p 覆盖压力并乘 10 转为 PSTRESS 的 kbar；不传 -p 使用配置压力。

## 使用边界

rvf 按 OSZICAR 判断并有循环上限，不等于保证最终物理收敛。有 -p 时输出为 `<压力>/vasp_opt`，当前会忽略 -w；多个压力写 submit_all_vasp_opt.sh。

## 实现依据与关联

[CLI 定义及别名映射](../../../vaspflow/src/bin/vaspflow.rs)；[核心实现](../../../crates/df-vasp/src/opt.rs)；[VASP 库覆盖](../../../crates/df-vasp/tests/vasp.rs)、[后处理覆盖](../../../crates/df-vasp/tests/process.rs)。覆盖文件的存在不代表本次重新验收。

关联 [0004 Rust 迁移](../../specs/0004-dftflow-rust/spec.md)。
