# vaspflow vasp-charge：电荷密度计算输入

[返回 vaspflow 索引](README.md)

## 功能与当前行为

采用 [vasp_charge] 模板生成电荷计算任务；这是家族子命令，独立 vasp_charge 名称指向后处理。 等价家族入口：`vaspflow vasp-charge`。

## 使用方法

```bash
vaspflow vasp-charge -c input.toml -p POSCAR
# 检查输入后，在计算资源中运行；也可使用 -s 提交
vaspflow vasp-charge -c input.toml -p POSCAR -r
```

-c/--config 指定配置，建议显式传入；-p/--poscar 默认 POSCAR；-w/--work-dir 改变输出目录；不加 -r/-s 只准备，-r 直接运行，-s 提交。注意这里结构选项是 -p，不是 -i。

## 输入、输出与衔接

默认 `vasp_charge/` 下生成 POSCAR、INCAR、KPOINTS，以及配置了 `[vasp].potcar_dir` 时的 POTCAR。当前启动目录生成 `run_vasp_charge.sh`，实际 VASP 日志为工作目录中的 vasp.log。`[vasp].executable_path` 是完整执行命令；`[slurm].header` 提供作业环境。

## 使用边界

不要用 vasp_charge 替代本命令生成计算输入；该独立别名会读取既有 CHGCAR。 公共 setup 优先用 VASPKIT 生成网格，失败时回退 Rust 公式。未配置 POTCAR 库不会自动补齐 POTCAR；续算文件缺失可能仅警告，需提交前检查。

## 实现依据与关联

[CLI 定义及别名映射](../../../vaspflow/src/bin/vaspflow.rs)；[核心实现](../../../crates/df-vasp/src/setup.rs)；[VASP 库覆盖](../../../crates/df-vasp/tests/vasp.rs)、[后处理覆盖](../../../crates/df-vasp/tests/process.rs)。覆盖文件的存在不代表本次重新验收。

关联 [0004 Rust 迁移](../../specs/0004-dftflow-rust/spec.md)。
