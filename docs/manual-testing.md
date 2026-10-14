# 手动计算操作手册（test_auto/MgB2）

本文档完整收录测试用输入文件、需要输入的命令与操作步骤，供后续手动计算查对（Issue 0005）。
配合本地留档 `docs/local/paths.toml`（机器真实路径，不入 Git）与 `docs/local/qe-install-m5.md`（Mac QE 编译记录）使用。

## 一、前置条件：机器路径占位符机制

仓库内所有测试输入文件**不含机器真实路径**，统一使用 `@KEY@` 占位符；
真实路径只存在于本机的 `docs/local/paths.toml`（`[placeholders]` 表）。

| 占位符 | 含义 | 使用的文件 |
| --- | --- | --- |
| `@QE_EXEC@` | QE 启动命令 + bin 目录（如 `mpirun -np 6 /path/to/q-e/build/bin`） | `input_qe.toml` |
| `@VASP_EXEC@` | 集群 VASP 完整启动命令 | `input_vasp.toml` |
| `@LOBSTER_EXEC@` | 集群 LOBSTER 可执行文件 | `input_vasp.toml` |
| `@WANNIER90_EXEC@` / `@PW2WANNIER90_EXEC@` / `@POSTW90_EXEC@` | Wannier90 三件套 | `input_w90.toml` |
| `@LOCAL_VASP_EXEC@` | 本机 macOS VASP 启动命令 | `local_run/*` |
| `@LOCAL_LOBSTER_EXEC@` | 本机 macOS LOBSTER（Rosetta） | `local_run/*` |

- **pytest 运行**：`test_auto/MgB2/conftest.py` 拷贝输入文件时自动注入，无需手动操作；
- **手动计算**：先就地替换占位符（替换后的文件含真实路径，**严禁提交**）：

```bash
cd /path/to/dftflow   # 仓库根
python3 test_auto/MgB2/_local_paths.py test_auto/MgB2/local_run
```

## 二、测试资产清单（test_auto/MgB2/）

| 文件/目录 | 跟踪状态 | 说明 |
| --- | --- | --- |
| `MgB2.vasp` | 跟踪 | VASP 结构文件（POSCAR 格式） |
| `input_qe.toml` | 跟踪 | QE 全流程测试配置（scf/nscf/eledos/eband/phonon/phband/phdos），含占位符 |
| `input_vasp.toml` | 跟踪 | VASP 全流程测试配置（scf/eband/eledos/fermi/cohp/locpot/charge/elf/opt），含占位符 |
| `input_w90.toml` | 跟踪 | Wannier90 测试配置，含占位符 |
| `conftest.py` | 跟踪 | pytest fixtures：共享工作目录 + 占位符自动注入 |
| `_local_paths.py` | 跟踪 | 占位符注入模块（pytest 与手动模式共用） |
| `pp/` | 跟踪 | QE 赝势：`b_pbe_v1.4.uspp.F.UPF`、`mg_pbe_v1.4.uspp` |
| `potcar_lib/`（local_run 下） | 跟踪 | VASP POTCAR 库：`B`、`Mg` |
| `local_run/input_vasp_local.toml` | 跟踪 | 本机 macOS SCF 配置（smoke kmesh=0.3 与正式 0.04 两套段） |
| `local_run/input_vasp_cohp_local.toml` | 跟踪 | 本机 COHP uniform 配置（kmesh=0.25，正式 0.04 留给集群） |
| `local_run/run_vasp_scf.sh` | 跟踪 | 归档的真实运行命令记录：`vasp_scf` 工作目录内执行 VASP |
| `local_run/run_vasp_cohp_uniform.sh` | 跟踪 | 归档的真实运行命令记录：`vasp_cohp_uniform` 工作目录内执行 VASP |
| 各 flow 的 `test_*.py` | 跟踪 | pytest 黄金基线脚本 |
| `scflow/validate_scflow_isif4.py` | 跟踪 | scflow ISIF4 独立验证脚本 |
| `MgB2_calc/`、`local_run/` 运行输出 | **不入 Git** | 真实计算产物保留本地磁盘，重算可再生成 |

## 三、pytest 命令序列

```bash
cd /path/to/dftflow

# 收集检查（不需要外部程序）
python3 -m pytest test_auto/MgB2 -q --collect-only

# 运行全部测试（-r 标记的测试会真实运行 QE/VASP，耗时较长）
python3 -m pytest test_auto/MgB2 -q

# 只运行某一 flow
python3 -m pytest test_auto/MgB2/vaspflow -q
python3 -m pytest test_auto/MgB2/qeflow -q
```

> **当前可运行性（2026-10-13，Issue 0004 P5 后）**：Python 包
> `qeflow/scflow/vaspflow/wannier90flow/substitutionflow/public` 已删除并被 Rust 家族二进制替代，
> 上述 pytest 中依赖这些模块的用例为环境性失败（历史基线约 34 项）；
> 数值回归已迁至 Rust 侧 `cargo test --workspace`（75 项）。
> pytest 保留作黄金基线脚本档案，需在冻结参考工作树（标签 `python-reference-0004`，提交 `9396b3a`）上运行。

## 四、Rust 家族二进制构建与 CLI

```bash
cd /path/to/dftflow
cargo build --release          # 产出 target/release/{qeflow,vaspflow,scflow,wannier90,substitution}
cargo test --workspace         # 75 项 Rust 集成测试（黄金基线对照）

# 旧名兼容入口（48 个，argv[0] 分派）
scripts/install-compat.sh <bin_dir> [target_dir]
```

| 家族二进制 | 覆盖旧命令 |
| --- | --- |
| `qeflow` | 17 个 `qe_*`（独立 dyn0 工具见 `tools/qf-gen-dyn0`） |
| `vaspflow` | 19 个 `vasp_*` |
| `scflow` | `scflow` |
| `wannier90` | 4 个 `wannier_*` |
| `substitution` | 7 个 `substitution*` |

复用现有 `input.toml` 契约（`[qe]`、`[qe_phonon.*]`、`[vasp]`、`[wannier90]`、`[scflow]` 等配置块），
典型调用形如 `scflow -c input.toml`；各命令详细参数用 `--help` 查询，语义见
`docs/specs/0004-dftflow-rust/contracts/config-and-output.md`。

## 五、本机 macOS 手动计算步骤（local_run）

以下步骤重现 2026-09 归档的 COHP uniform E2E 全流程（产物保留在 `local_run/`，不入 Git）。

```bash
cd /path/to/dftflow/test_auto/MgB2/local_run

# 1. 注入本机真实路径（占位符 -> 真实 VASP/LOBSTER 命令）
python3 ../_local_paths.py .

# 2. VASP SCF：生成 CHGCAR/WAVECAR 供 COHP 使用
#    配置来自 input_vasp_local.toml [vasp_scf]：
#    ENCUT=520 ISMEAR=0 SIGMA=0.05 EDIFF=1e-6 LREAL=Auto NCORE=4 kmesh=0.04
#    工作目录 vasp_scf/ 内需要 INCAR/KPOINTS/POSCAR/POTCAR（B、Mg 顺序拼接）
#    归档命令（run_vasp_scf.sh，注入后展开）：
bash run_vasp_scf.sh
#    即: cd vasp_scf && mpirun -np 4 .../vasp_std > vasp.log（产物 vasp_scf/vasp.log、CHGCAR、WAVECAR）

# 3. COHP uniform：input_vasp_cohp_local.toml
#    [vasp_cohp_uniform] kmesh=0.25 ENCUT=520 ISMEAR=-5 ISYM=-1 NCORE=4 NBANDS=32
#    工作目录 vasp_cohp_uniform/，产物 COHPCAR.lobster 等（lobster 由 @LOCAL_LOBSTER_EXEC@ 调用）
bash run_vasp_cohp_uniform.sh

# 4. 分压强扫描（历史归档 vasp_cohp_uniform/B_B_1.0_2.5/ 即 d_limit 1.0–2.5 一组）：
#    通过 vaspflow 的 vasp_cohp_uniform 命令以 -d "1.0 2.5" 参数批量生成并运行，
#    产物目录命名 <元素>_<元素>_<dmin>_<dmax>/，含 COHPCAR/ICOHPLIST 等 lobster 输出。
```

> smoke 快速验证：改用 `input_vasp_local.toml` 中 kmesh=0.3 的 `[vasp_cohp_uniform]` 段（历史 `input_vasp_smoke.toml`）。

## 六、集群手动计算步骤

集群侧使用 `test_auto/MgB2/input_vasp.toml`（VASP+LOBSTER）与 `input_qe.toml`（QE 全流程）、
`input_w90.toml`（Wannier90），真实路径来自 `docs/local/paths.toml` 的集群键。

```bash
# 1. 在集群上注入占位符（拷贝 docs/local/paths.toml 到集群仓库同位置后执行）
python3 test_auto/MgB2/_local_paths.py test_auto/MgB2

# 2. 构建 Rust 家族二进制或使用发布包（48 个旧名入口）
cargo build --release
scripts/install-compat.sh <bin_dir>

# 3. 以旧名 CLI 复现 pytest 所覆盖的流程（在任意空工作目录，放置 MgB2.vasp 与输入 toml）：
vasp_scf    -c input_vasp.toml      # SCF：生成 CHGCAR/WAVECAR
vasp_eband  -c input_vasp.toml      # 能带（WAVECAR 复用，ISTART=1）
vasp_eledos -c input_vasp.toml      # 分波态密度（vaspkit 生成 KPOINTS 并按 kmesh 校准）
vasp_cohp_uniform -c input_vasp.toml -d "1.0 2.5"   # COHP/LOBSTER
qe_scf      -c input_qe.toml        # QE SCF -> nscf -> eledos/eband
qe_phonon   -c input_qe.toml        # 声子 split-merge 全流程（ph/q2r/matdyn）
```

> 旧名命令与 Rust 家族二进制等价（`argv[0]` 分派）；具体参数以各命令 `--help` 与
> `docs/specs/0004-dftflow-rust/contracts/` 为准。

## 七、scflow 独立验证脚本

```bash
# ISIF4 完整验收脚本（读取 Tc-236 等 ISIF4 工作区，输出对照表）
python3 test_auto/MgB2/scflow/validate_scflow_isif4.py --help
```

## 八、注意事项

1. 注入占位符后生成的文件（含真实路径）**不要提交**；提交前用 `git status` 检查，
   `git checkout -- <file>` 可恢复占位符版本。
2. `docs/local/` 整个目录不入 Git；机器变化只改 `docs/local/paths.toml`。
3. 运行产物目录（`MgB2_calc/`、`local_run/` 运行输出、各 flow 运行时生成目录）已被
   `.gitignore` 忽略，可随时删除重算。
4. 新增测试输入或占位符时，同步更新：`docs/local/paths.toml`、本手册第一节表格、
   `docs/qe-environment.md` 第四节。
