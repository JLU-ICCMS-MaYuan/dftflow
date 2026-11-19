# MgB₂ 超导性质计算

本示例基于 **EPW School 2024 Tutorial Wed.9 (Mori)** 的 MgB₂ 超导计算流程，使用各向异性 Migdal-Eliashberg 方程计算超导性质。

## 环境准备

### 1. 编译 dftflow（首次使用）

dftflow 是 Rust 项目，需要编译后使用：

```bash
cd "/public/home/mayuan/code/dftflow"

# 编译全部主产品（EPW 命令会自动发布到 bin/epwflow/）
bash "scripts/build-native.sh"
```

### 2. 配置 PATH

在当前终端加载环境脚本，将 dftflow 命令添加到 PATH：

```bash
source "/public/home/mayuan/code/dftflow/scripts/env.sh"
```

如需每次登录自动生效，将上面命令加入 `~/.bashrc`：

```bash
echo 'source "/public/home/mayuan/code/dftflow/scripts/env.sh"' >> "$HOME/.bashrc"
```

### 3. 验证安装

```bash
# 检查命令是否可用
command -v epw_phonon epw_wannier epw_superconductivity
epw_phonon --help
```

应该显示 `/public/home/mayuan/code/dftflow/bin/epwflow/epw_phonon`。

## 参数说明

配置文件：`input.toml`

**关键参数（基于 Tutorial）：**
- SCF k-mesh: 8×8×8
- NSCF k-mesh: 6×6×6
- Phonon q-mesh: 3×3×3
- Fine grids: 40×40×40 (k), 20×20×20 (q)
- 截断能: 40 Ry
- 展宽: 0.05 Ry (mp)
- Wannier 函数数: 5 个
- 投影: B:pz + 3 个 s 轨道
- dis_froz_max: 8.8 eV

**注意：** 本配置使用小网格用于功能测试，实际收敛计算需要更密的网格。SCF 和 NSCF 使用不同的 k 点网格（如 PDF Tutorial 所示）是允许的。

程序配置检查已通过，尚不代表完整八阶段计算或科学结果验收完成。计算参数变化后，应由程序检查已有结果的相容性。

## 计算流程

完整的 EPW 超导计算分为以下 8 个步骤：

### 1. SCF 自洽计算
```bash
epw_phonon -c "input.toml" --stage scf -s
```
在 8×8×8 k 网格上进行自洽场计算，获得电荷密度。SCF 和 NSCF 可以使用不同的 k 点网格。

### 2. NSCF 计算
```bash
epw_phonon -c "input.toml" --stage nscf -s
```
在 6×6×6 均匀 k 网格上进行非自洽计算。

### 3. 电子能带与态密度
```bash
epw_phonon -c "input.toml" --stage electronic -s
```
生成电子能带结构和态密度（DOS/PDOS）。

### 4. 声子计算
```bash
epw_phonon -c "input.toml" --stage phonon -s
```
在 3×3×3 q 网格上计算声子色散和态密度。

### 5. Wannier 插值
```bash
epw_wannier -c "input.toml" --stage wannier -s
```
构建 5 个 Wannier 函数，用于电声耦合插值。

### 6. 粗网格电声耦合
```bash
epw_wannier -c "input.toml" --stage coarse -s
```
在粗网格（6×6×6 k, 3×3×3 q）上计算并写入电声耦合矩阵。

### 7. 细网格电声耦合
```bash
epw_superconductivity -c "input.toml" --stage fine -s
```
在细网格（40×40×40 k, 20×20×20 q）上插值电声耦合，准备超导计算。

### 8. 超导性质计算
```bash
epw_superconductivity -c "input.toml" --stage superconductivity -s
```
当前配置使用 `task = "gap"`、`mode = "both"`，在 20、30、40 K 求解各向同性和各向异性 Migdal-Eliashberg 方程，计算固定温度能隙；不执行 Tc 搜索。超导态密度等图表取决于实际生成的原生数据。

## 结果文件

计算结果保存在 `run/` 目录下：
- `run/1.qe-scf/`: SCF 自洽计算结果
- `run/2.qe-nscf/`: NSCF 非自洽计算结果
- `run/3.qe-electronic/`: 电子能带和态密度
- `run/4.qe-phonon-index/`: 声子色散和态密度
- `run/5.epw-wannier/`: Wannier 函数和能带插值
- `run/6.epw-coarse/`: 粗网格电声耦合
- `run/7.epw-fine/`: 细网格电声耦合和 Eliashberg 函数
- `run/8.epw-superconductivity/results/`: 超导分析图（SVG/PNG/PDF 格式）

## 参考文献

- H. Mori, *Calculations of superconducting Tc*, EPW School 2024, Tutorial Wed.9
- Phys. Rev. B **87**, 024505 (2013) - Migdal-Eliashberg 理论

## 快速开始

**前提条件：**
1. 已安装 dftflow 并激活环境（见"环境准备"）
2. 已配置好 Quantum ESPRESSO 和 Wannier90
3. 已准备好赝势文件（配置在 `input.toml` 的 `[pseudos]` 中）
4. 已配置好 Slurm 集群参数（如使用 `-s` 提交）

**运行流程：**

按顺序执行上述 8 个命令。每步提交后等待作业完成并核验结果，再推进下一步；不要一次连续提交所有阶段。

如已处于 Slurm 分配内，可将 `-s` 替换为 `-r` 同步运行；不要在集群登录节点直接执行科学计算：
```bash
epw_phonon -c "input.toml" --stage scf -r
```
