# VASP ELF 计算说明

## 1. 计算所需物理量
ELF（Electron Localization Function）计算需要以下实空间量：

- 电子密度 ρ（`CHDEN`）
- 动能密度相关项 T（由 `CKINE` 计算）
- 密度梯度平方 |∇ρ|²（`CGRDSQ`）
- 密度的拉普拉斯 ∇²ρ（`LAPLAC`）

## 2. 公式定义（源码注释）

- 动能密度
  - T = −(ħ² / 2m) Ψ ∇² Ψ
- 修正项
  - TCORR = −(ħ² / 4m) ∇²ρ
- “Bose 气体”动能项
  - TBOS = (ħ² / 8m) * |∇ρ|² / ρ
- 均匀电子气参考
  - DH = (3/5)(ħ² / 2m)(3π²)^(2/3) ρ^(5/3)
- 局域化度量
  - D = T + TCORR − TBOS
- ELF
  - ELF = 1 / (1 + (D / DH)²)

## 3. 源码实现细节（`ELFCAL`）

在 `external/vasp.6.3.2/src/elf.F` 的 `SUBROUTINE ELFCAL` 中，对每个格点 N：

- T = REAL(CKINE(N))
- TCORR = REAL(LAPLAC(N)) * HSQDTM / 2
- TBOS = HSQDTM / 4 * REAL(CGRDSQ(N) / CHDEN(N))
- 若 ρ < 0，则 DH = 0；否则
  - DH = 0.2 / NSPIN * HSQDTM / π² * (3π² * NSPIN * ρ)^(5/3)
- 最终
  - CKINE(N) = 1 / (1 + ((T + TCORR − TBOS) / max(DH, 1e-8))²)

## 4. 相关源码位置

- 主计算：`external/vasp.6.3.2/src/elf.F` → `SUBROUTINE ELFCAL`
- 相关量的计算（ρ、动能密度、|∇ρ|²、∇²ρ）在同文件中 ELF 前完成。
