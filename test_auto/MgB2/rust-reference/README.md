# MgB2 独立 SCF/NSCF 参考输入

- 来源：冻结 Python 提交 `9396b3a`（`python-reference-0004`）的 `create_qe_calculation_setup`。
- 输入：上级目录 `MgB2.vasp`、`input_qe.toml`、`pp/`；仅将 `@QE_EXEC@` 换为测试命令目录。
- `@WORK@` 是临时目录根，不参与路径逐字节对照；实际赝势文件名仍精确对照。
- Rust 回归：`cargo test -p qeflow --test basic_cli`；比较全部 namelist 字段、卡片、原子坐标和 k 点，数值容差为 `1e-8 + 1e-6 × |ref|`。
- 进程测试使用明确的替身程序检查 stdin、cwd、退出码及提交参数，不能代替真实 QE 验收；真实 MgB2 对照见 0004 `acceptance-20261014.md`。

- `grid.xml`：本轮真实 MgB2 SCF XML 中仅保留倒格矢与旋转矩阵，用于 dyn0 别名行为测试；`KPATH.in` 是同一 MgB2 经 VASPKIT 3→303 生成的原始格式，包含四列标题回归。
