# dftflow

DFT 计算工作流自动化（Issue 0004：全量 Rust 重写）。

当前分支 `qeflow-rust` 已完成 `qeflow / vaspflow / scflow / wannier90flow / substitutionflow / public` 六个 Python 包的全量 Rust 迁移（P0–P5）：交付五个家族静态二进制（`qeflow` / `vaspflow` / `scflow` / `wannier90` / `substitution`），48 个旧命令名经 `scripts/install-compat.sh` 创建的硬链接入口直接可用；`mattersimflow` 保留 Python（`mattersim_opt`）。

## 构建

```bash
cargo build --release
# Linux musl 静态交付（目标机）
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
strip target/x86_64-unknown-linux-musl/release/{qeflow,vaspflow,scflow,wannier90,substitution}
```

## 安装旧命令兼容入口

```bash
./scripts/install-compat.sh target/release ~/.local/bin
```

为 48 个旧命令名（17 个 `qe_*`、19 个 `vasp_*`、4 个 `wannier_*`、7 个 `substitution*`、1 个 `scflow`，含 `qf-gen-dyn0` 别名）创建指向五个家族二进制的硬链接，二进制按 `argv[0]` 分派，现有 Slurm/脚本调用方式不变。

## 命令速查

- **QE**：`qe_scf` `qe_nscf` `qe_eband` `qe_eledos` `qe_process_eband` `qe_process_eledos` `qe_ph_nosplit` `qe_merge` `qe_q2r` `qe_matdyn_phband` `qe_matdyn_phdos` `qe_process_phonon` `qe_split_submit` `qe_check` `qe_phonon_nosplit` `qe_phonon_split_index` `qe_phonon_split_coords`
- **VASP**：`vasp_scf` `vasp_scf_converge` `vasp_eband` `vasp_process_eband` `vasp_eledos` `vasp_process_eledos` `vasp_idos_computed` `vasp_fermi` `vasp_cohp_uniform` `vasp_cohp_fatband` `vasp_lobsterin` `vasp_plot_cohp` `vasp_plot_kcohp` `vasp_locpot` `vasp_process_locpot` `vasp_charge` `vasp_charge_diff` `vasp_opt` `vasp_elf`
- **Wannier**：`wannier_init` `wannier_post` `wannier_window` `wannier_plot`
- **Substitution**：`substitution` `substitution_structure` `substitution_continue` `substitution_prepare_potcar` `substitution_prepare_qe_pseudo` `substitution_plot_eledos` `substitution_check_phonon_stability`
- **scflow**：`scflow -c input.toml`（λ/ω_log/Tc、a2F、Eliashberg；详见 [scflow README](scflow/README.md) 与 [功能总览](docs/overview/scflow.md)）
- **MatterSim（Python 例外）**：`pip install .` 后使用 `mattersim_opt`

配置契约（`input.toml` 递归覆盖、状态机、容差）见 [0004 契约](docs/specs/0004-dftflow-rust/contracts/config-and-output.md)。

## High Throughput Substitution

- `mattersim_opt` 通过外部 MatterSim Python 环境运行两阶段预优化：先只优化原子坐标，再在目标压力下联合优化晶格和原子坐标，输出 `mattersim_opt/CONTCAR` 供后续 `vasp_opt` 使用。
- `substitution_structure` 是高通量替换工作流的 prepare 步骤：从 `[substitutionflow]` 读取 `XA/XB` 原子序号范围，生成 `<Z>.XA_<元素>/period_n/<Z>.<formula>` 目录树，并同时写出替换结构、case `input.toml` 与初始状态文件。
- 每个 `period_n/` 目录保存一份静态 `workflow_manifest.json`，只记录输入结构、`XA` 元素、压力列表和该组 case 清单。
- 每个 case 下生成的 `input.toml` 是后续 `vasp_*`、`qe_phonon_*` 和 `substitution_continue` 的统一配置真源；它默认完整继承根配置中的共享块和子工作流参数块，只对 `chgcar_path`、`wavecar_path` 做 case 级相对路径重写。
- 每个 case 的 `case_status.json` 记录各压力点在 `mattersim_opt / vasp_opt / vasp_scf / vasp_eledos / vasp_process_eledos / qe_phonon` 的 `finished / error / unstarted` 状态（OUTCAR 启发式判定，无 daemon）；根目录 `workflow_status.json` 汇总供续算。
- `[substitutionflow].submit_groups` 控制 `-s/--submit` 的提交分组；一个 `case + pressure` 是不可拆分的完整提交单元，逐组 `sbatch submit_group_NN.sh`。
- `[substitutionflow].phonon_mode` 控制末尾 QE 声子总工作流，允许 `nosplit / split_index / split_coords`，默认 `split_index`；split 模式下 `qe_split_submit` 会进入 `NN/` 目录逐 q 点 `sbatch run_split.sh`。
- `substitution_continue -o <dir> -c <input.toml>`（`-rfs` 刷新状态 / `-rws` 重写 error、`-r/-s` 运行/提交）按状态文件从第一个非 `finished` 步骤续算。
- `substitution_prepare_potcar --pot <lib>` 与 `substitution_prepare_qe_pseudo --pp <dir>` 准备本地 `potcar_lib/`、`qe_pseudo_lib/`（交互式选号，选择记录即时写盘）。
- `substitution_plot_eledos` / `substitution_check_phonon_stability` 输出周期表式批量总览（`eledos_summary_<P>.*`、`phonon_stability_summary_<P>.png`；最小频率 `< -1.0 THz` 判 `unstable`）。

示例 `input_substitution.toml` 片段：

```toml
pp_dir = "qe_pseudo_lib"

[qe]
executable_path = "/opt/qe/bin"
lattice_mode = "ibrav_angstrom"

# lattice_mode 允许：
# - "ibrav_angstrom"：默认，自动判断 ibrav 并写 A/B/C/cos*
# - "ibrav_celldm"：自动判断 ibrav 并写 celldm(n)
# - "cell_parameters"：旧格式，写 ibrav = 0 + CELL_PARAMETERS {angstrom}
# 不要手动写 ibrav、A/B/C、cosAB/cosAC/cosBC 或 celldm(1..6)。

[qe_phonon]
prefix = "ht_ph"

[qe_phonon.ph]
qpoints = [2, 2, 1]
recover = true

[substitutionflow]
XA = "3,4"
XB = "3-5"
pressures = [20, 50]
structure_filename = "XAXBH7-P63mmc.vasp"
phonon_mode = "split_index"
```

## 文档

- 实现总览：[docs/overview/dftflow-rust.md](docs/overview/dftflow-rust.md)
- Spec / 契约 / 验收：[docs/specs/0004-dftflow-rust/](docs/specs/0004-dftflow-rust/spec.md)
- 各 flow 的 README：`qeflow/` `vaspflow/` `scflow/` `wannier90flow/` `substitutionflow/` `mattersimflow/`
- 架构决策：[docs/adr/](docs/adr/README.md)
