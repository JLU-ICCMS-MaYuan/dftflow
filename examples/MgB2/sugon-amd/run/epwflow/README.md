# MgB₂：在本目录运行 EPW 算例

当前工作目录是 `/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow`。日常输入只有本目录的 `epwinput.toml`；结构使用上一级 `run/POSCAR`，赝势使用上一级 `run/pp/`，路径使用本目录 `KPATH.in`。TOML 相对路径以 TOML 所在目录为基准，因此结构写 `../POSCAR`。

## 新版源码的常规对比（0043）

独立抽样核验已取消：新增运行不生成 quality-sources 或专用报告，不启动参考 NSCF／直接 PH／dynmat；常规路径与 DOS 出图仍保留。四项最大差值、位置、两侧值和配对数量存于 comparison/metrics.json，并在终端与图中展示，没有数值阈值或告警。新配置需删除 `[interpolation.quality]`。已有主科学数据通过 `--import-work` 复用，抽样参考不再导入；旧工作区不自动迁移。

本次只修改源码、测试与文档，没有覆盖默认安装或重跑本例。下方旧作业、旧报告及图像记录保持当时含义；不能将它们当成新版四项统计。设计与验收见 [0043](../../../../../docs/specs/0043-epw-comparison-max-differences/acceptance.md)。

## 历史运行记录

2026-10-21 已实际通过公开命令提交测试：第 1～4 步完成；第 5 步作业 1314551 完成，拟合与电子图已核验；原生粗 EPC 在 1314626 完成，作业 1314972 复用它完成了比较后处理。后者的最终退出原因是 **质量未通过**，不是此前的缺文件或路径解析错误。

- 电子最大／RMS 为 0.040298／0.022388 eV，既定门为 0.05／0.02，RMS 未通过。
- 声子最大／RMS 为 184.512／67.118 cm⁻¹，既定门为 50／20，均未通过，直接参考中还有负频率。
- 作业 1314972 当时因此停止。用户随后在 0040 要求超差只告警：新版本保留上述未达标数值，允许生成粗网格交付并继续第 7／8 步；不改这些理想阈值，也不把继续执行写成科学质量通过。实际续算结果见文末。

结果看 `6.epw-coarse/quality-report.json` 和 `6.epw-coarse/results/figures/`（四张比较图加总览，各有 PNG／SVG／PDF）。图中显示 `quality: failed`、`PARTIAL RESULTS`；宽外窗的电子 DOS 有 239 个参考态没有对应的 Wannier 子空间态，只能作为子空间诊断。严格投影仍暂缓，图像完整不等于科学完整。前后保护的 QE／PH、独立参考与拟合文件哈希保持。

只想重绘现有图，不需提交计算：

```bash
epw_wannier --plot --results "$PWD/6.epw-coarse/results"
epw_wannier --results-check --results "$PWD/6.epw-coarse/results" --json
```

结果检查的 `operation_status="ok"`、`figures_verified=true` 表示操作及图像身份有效；`quality_status="failed"` 仍表示数值未达目标。0040 后该状态只告警，允许继续细网格／超导；workflow_complete 另行记录原生链是否完整。路径图将 G/GAMMA 显示为 Γ，邻近断点的标签合并为 `A | L` 等，竖线与科学坐标不改动，见 [0039](../../../../../docs/specs/0039-comparison-path-labels/acceptance.md)。

## 进入目录与检查

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
cd "/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow"
epw_phonon -c epwinput.toml --check --json
epw_phonon -c epwinput.toml --status --json
```

`source` 将仓库 `bin/epwflow` 等产品目录加入当前终端 PATH。`-c epwinput.toml` 明确使用你指定的文件；当前程序的默认文件名为 `input.toml`，这里不依赖默认查找。`--check` 核对配置、文件路径和并行设置，`--status` 核对科学参数、上游和产物身份，均不提交作业。

## 按阶段执行

每条提交后先等待对应作业结束，查看日志和结果，再执行下一条；程序会复用相容且完整的上游，不需要手工指定 CALC、CFG、W 或 LEDGER。

```bash
# 第 1～4 步：QE SCF、完整均匀 NSCF、电子参考与 PH／声子后处理。
epw_phonon -c epwinput.toml -s

# 第 5 步：原生 Wannier 拟合、电子能带及总 DOS 对比。
epw_wannier -c epwinput.toml --stage wannier -s

# 第 6 步：复用 Wannier 变换，粗 EPC、常规比较图与最大差值。
epw_wannier -c epwinput.toml --stage coarse -s

# 第 7 步：重验粗网格来源，执行细网格 EPC、α²F 和 λ。
epw_superconductivity -c epwinput.toml --stage fine -s

# 第 8 步：按配置执行 iso／aniso 原生求解及 Tc 搜索。
epw_superconductivity -c epwinput.toml --stage superconductivity -s
```

去掉 `-s` 只生成当前请求所需输入和提交脚本。`--resume -s` 同样读取当前配置；失败后执行同一公开命令即可创建新尝试，不手改冻结记录，也不重复 sbatch 已绑定旧作业的脚本。活动或终态未知的任务禁止重复提交。作业号由命令输出，可用 `sacct -j 作业号` 查看终态；提交成功并不等于计算完成。

## 文件放在哪里

```text
epwflow/
├── epwinput.toml、KPATH.in、submit_all.sh
├── 1.qe-scf/
├── 2.qe-nscf/
├── 3.qe-electronic/qe_eband/、qe_eledos/
├── 4.qe-phonon-nosplit/save/
├── 5.epw-wannier/
├── 6.epw-coarse/
├── 7.epw-fine/
├── 8.epw-superconductivity/iso/、aniso/
├── epw-state/                  # 可见的提交快照、锁、历史与跨阶段记录
└── historical-inputs/          # 本轮归档的旧准备文件，内容不变
```

`work_dir = "."` 将编号阶段直接放在本目录，不再额外嵌套 epwflow。各阶段的 `out/` 保存原生重读数据；`scratch` 是配置的临时数据基址，实际 outdir 以生成的原生输入为准。无需另建旧 `work/`。输入、提交该任务的脚本和日志位于同一个实际计算目录，PH 分 q 和各温度子任务同样如此。

阶段完成记录是本阶段的 `completion.json`，电子子任务记录在第 3 步的 `receipts/`，后处理记录跟随本阶段。删除阶段目录或必要产物后，当前源码将该阶段判为需要更新，并保留相容上游；内容篡改、损坏记录、活动或未知作业仍单独报错。可见状态维护的源码与安装边界见 [0025 验收](../../../../../docs/specs/0025-epw-visible-run-state/acceptance.md)。

第 5 步成功后可看 `electronic-bands.png`、`electronic-dos.png`，并保留 DOS CSV 和规范化说明；新版第 6 步只显示常规对比的最大差值，不据此判断是否继续细网格。图像重叠不能代替数值校验，严格元素／轨道投影仍暂缓。

非磁标量 DOS 两侧原生程序都计入两重自旋，Wannier 曲线直接使用原生数值，倍率为 1；`electronic-dos-normalization.json` 保存每态电子数及原始 DOS／日志哈希。断开的能带路径分别绘制，同位置端点合并为 `A | L`、`M | H`。修复与实际重绘验收见 [0031](../../../../../docs/specs/0031-postw90-dos-spin-normalization/acceptance.md)、[0032](../../../../../docs/specs/0032-disconnected-electronic-band-paths/acceptance.md)。

第 6 步需要第 5 步生成并绑定的 `.ukk`、`.bvec`、`.mmn` 和收敛日志。旧收据没有绑定速度矩阵配套文件时，第 5 步会重建一次；已有 QE／PH 保留，原拟合归档。后续独立绘图关闭 `write_bvec`，保护已绑定文件。细节见 [0033](../../../../../docs/specs/0033-wannier-velocity-handoff/acceptance.md)。历史抽样参考与编排器设计保存在 [0034](../../../../../docs/specs/0034-reference-orchestrator-upgrade/acceptance.md)。

## 本例的局域化与声子恢复

本例显式设置 `[wannier] guiding_centres = true`，用于原生 Wannier 局域化的中心相位引导；局域化上限为 5000、容差仍为 `1e-8 Å²`、收敛窗口仍为 5 步。仅增加迭代上限未能解决此前振荡，中心引导已在独立副本上通过原生对照，完整公开链的最新结果以 [0030 验收](../../../../../docs/specs/0030-wannier-guiding-centres/acceptance.md) 为准。程序会检查原生日志确实采用该开关，不自动放宽收敛标准。

q2r／matdyn 失败后，已校验的 PH 响应可以复用，重新执行公开命令继续后处理；`phonon: update` 不必然表示重算 PH。归集副本是恢复依据，不能只凭 `ph.out` 的 `JOB DONE` 删除或伪造记录。实际自复制截断缺陷及恢复见 [0028](../../../../../docs/specs/0028-phonon-postprocess-checkpoint/acceptance.md)；Slurm 日志归档入口修复见 [0029](../../../../../docs/specs/0029-active-slurm-log-archive/acceptance.md)。

## 并行与状态字段

当前 NSCF 为 4×4×2，共 32 个 k 点；采用 32 MPI／32 pool。当前 EPW 单 image 路径每 pool 一个 MPI，pool 不能超过 k 点数，不能只把进程和 pool 改成 64。生成、提交前和脚本启动均预检；程序不会自动改变物理网格绕过错误。

`operation_status="ok"` 仅表示本次操作成功；`workflow_status="configured"` 表示输入／脚本准备完成；`reuse` 表示当前参数和文件身份允许复用，`update` 表示需要执行。上述状态都不能单独证明科学计算收敛或质量通过。

`epw-state/control.lock` 防止并发生成／提交，`execution.lock` 防止科学执行覆盖，正常结束自动释放。看到锁提示应核对 owner、正在执行的命令及 Slurm 状态，不直接删除未知锁。本轮真实 debug、程序版本、作业及限制见 [0027 验收](../../../../../docs/specs/0027-mgb2-epwflow-example-debug/acceptance.md)。

Slurm 日志是执行输出，正常追加不能当成科学输入损坏；原生重试归档后当前日志入口仍持续写入，见 [0035](../../../../../docs/specs/0035-native-retry-slurm-logs/acceptance.md)。第 6 步 `out/MgB2.wfcN` 是可写的原生工作副本，EPW 会改写它们；失败重试会归档并从已校验来源重新复制。原始第 2 步 NSCF、portable SAVE 波函数和 PH 响应仍受严格哈希保护，不需要手工复制或改收据，见 [0036](../../../../../docs/specs/0036-epw-mutable-wavefunction-scratch/acceptance.md)。


QE 显式路径的原生频率文件可能在未使用的第四列打印 `**********`。专用路径解析忽略该占位列，仍校验前三坐标和全部频率；保留原始文件，不需手工改它。程序升级后的比较恢复仍读取本次配置，复用原始科学数据及已完成的比较子任务。详见 [0037](../../../../../docs/specs/0037-matdyn-path-unused-weight/acceptance.md)、[0038](../../../../../docs/specs/0038-comparison-orchestrator-upgrade/acceptance.md)。

## 历史数值告警策略实测（0040，已由 0043 取代）

当时不需要改理想阈值。保留本目录 epwinput.toml，按上面的第 6→7→8 步顺序提交；已有粗 EPC 和上游重验后复用。终端／作业 stderr 的 quality_warning 是数值告警，不是任务失败。原 quality-report.json 中 passed=false 以及误差保留；阶段清单 quality.status=warning。实际原生程序失败、数据损坏、并行错误或预算不足仍非零退出。

完整交付时 workflow_complete=true 与 quality_warning=true 可以同时出现，前者表示执行链完成，后者提示数值尚不理想；不能据此认定 Tc 已科学收敛。需求与实测记录见 [0040](../../../../../docs/specs/0040-interpolation-quality-warnings/acceptance.md)。

2026-10-21 告警策略实测：第 7 步作业 **1316421 已完成（0:0）**，前五阶段复用，原生粗 EPC 不重算，粗交付按 warning 补发布后实际完成细网格和 500 点 α²F。当前 λ_integral=1.0811305、λ_sum=1.0810853，原电子／声子误差不变，属于当前小网格诊断结果。后续超导测试单独记录，不因第 7 步成功宣布 Tc 已完成。

第 8 步作业 **1316539** 已进入原生 iso／aniso。iso 线性区间 75.89～76.16 K，但 10 K 非线性出现 NaN 且未收敛；aniso 原生 80 K 收敛后遇到能隙省略 E 指数的解析 Bug，见 [0042](../../../../../docs/specs/0042-epw-fortran-exponent/acceptance.md)。因此当前没有完整双模式 Tc 交付，失败不是插值数值告警导致。保留当前输入和失败现场，不重复提交同一配置来掩盖未收敛。

0042 安装后已实际恢复为作业 **1316704**：前七阶段及十次有效 iso 线性求解复用，aniso 第一初值成功解析并保存 80 K／589 次收敛结果。当前停在 aniso 第二初值以及 iso 10 K 非线性未收敛，Slurm FAILED/1:0；这是原生求解结果，不是此前的格式错误，也不是插值误差超限。旧失败现场和新尝试均保留，尚无完整双模式 Tc。下一轮需要针对非线性求解参数单独诊断；直接重复同一配置预计仍会重试相同失败计算。
