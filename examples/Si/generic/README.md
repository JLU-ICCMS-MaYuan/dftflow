# Si 通用展示资产

## 来源

`POSCAR` 和 `KPATH.in` 从原 `qeflow/examples/` 原样迁入。该目录只保留结构和旧路径展示，不是已经验证的完整 QE 计算配置。

## 使用范围

`POSCAR` 是两原子的立方 Si 示例，未进行物理合理性或收敛验收。旧 `KPATH.in` 是五行连续点列表，与当前声子生成器要求的成对路径端点格式不同；运行时需按实际晶胞重新生成路径。不要直接把本目录当作生产计算输入。

可运行的 QE 声子流程参见 [MgB₂ / sugon-amd](../../MgB2/sugon-amd/README.md)。原目录内的合成 α²F 与 Si 没有物理关联，现单独放在 [synthetic-a2F](../../synthetic-a2F/generic/README.md)。
