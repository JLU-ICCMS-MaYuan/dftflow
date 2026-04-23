# qgrid.py 与 QE 源码对应关系

本模块的实现目标不是“凭经验生成 q 点”，而是按 QE 现有源码逐步复刻。

## 对应关系
- `external/quantum-espresso/PW/src/kpoint_grid.f90`
  - `n = (k-1) + (j-1)*nk3 + (i-1)*nk2*nk3 + 1`
  - `xkg(1..3)` 的 Monkhorst-Pack 枚举
  - `xkr = S * xkg`
  - `xkr = xkr - nint(xkr)`
  - `eps = 1.0d-5`
  - `mod / nint` 回映射
  - weight 归一
- `external/quantum-espresso/PHonon/PH/q_points.f90`
  - `kpoint_grid(...)` 调用
  - Gamma-first 逻辑
  - `dyn0` 中 `3e24.15` 风格坐标输出

## 说明
- `qgrid.py` 同时保留：
  - `q_crystal`
  - `q_cart`
- `dyn0` 对齐和 `split_coordinate` 输入统一使用 `q_cart`
- `q_index_map.json` 中保留 `q_crystal` 便于人工检查和后续扩展
