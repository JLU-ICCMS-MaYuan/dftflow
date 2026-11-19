# chem 集群上的 MgB₂ 测试

## 配置与范围

本目录使用 `input.toml` 的 `schema_version = 2` 八阶段 EPW 配置。结构复用 `test_auto/MgB2/MgB2.vasp`，高对称路径复用 `examples/MgB2/KPATH.in`；运行现场在 `run/`；已移除 `[budget]` 和 campaign／ledger 用户配置。相对路径以配置文件所在目录为基准。

- SCF／NSCF：PBE，k 网格 4×4×2，截断能 40／320 Ry，MV 展宽 0.02 Ry，NSCF 16 条带。
- 声子：index 模式，粗 q 网格 2×2×1。
- Wannier：5 个函数，B:pz 和三个键中心 s 投影；排除 Mg 半芯态的原始能带 1–4。
- 细网格：k 为 8×8×4、q 为 4×4×2。
- 超导：iso／aniso 在 20／30／40 K 求能隙，μ*=0.1；不做 Tc 搜索。
- 初始能窗沿用 Ubuntu 的 PBE 样例，须结合本次 NSCF 和 Wannier 日志核对。这些小网格只用于功能测试，不代表收敛的 MgB₂ 物性。

本配置替代本目录旧版 `prepare.sh chem index` 的 QE-only 使用说明；不需要再运行旧准备脚本。

## 程序和资源

DFTFlow 固定入口在 `/public/home/mayuan/code/dftflow/bin/<模块>/`，QE 7.6／EPW 6.1 在 `external/q-e/bin/`，独立 Wannier90 4.0.3 在 `external/wannier90/bin/`。科学程序路径在配置中显式指定。

Slurm 使用 `cpu` 分区、1 节点、56 任务，每任务 1 CPU、30 分钟时限；实际启动 32 路 MPI，EPW pool 为 32，OMP／MKL 线程为 1。粗 k 网格共 32 点；当前保留 32 路 MPI／32 pool 的已校验组合。作业加载 `/public/home/mayuan/intel/oneapi/setvars.sh`，使用 `LC_ALL=C`、`LANG=C`。这里是 chem 集群，不采用 Ubuntu 的 local 分区和 12 路约定。

DFTFlow 的私有运行库使用仓库固定前缀 `bin/.native/linux-x86_64`；不要把其中的 glibc 目录加入全局 `LD_LIBRARY_PATH`。配置环境脚本只负责科学程序，DFTFlow 的 ELF 加载器直接使用私有前缀。

```bash
source "/public/home/mayuan/code/dftflow/scripts/env.sh"
cd "/public/home/mayuan/code/dftflow/examples/MgB2/chem"
epw_phonon -c "input.toml" --check --json
epw_wannier -c "input.toml" --check --json
epw_superconductivity -c "input.toml" --check --json
```

`--check` 检查配置、路径和并行组合，不提交计算，也不表示整个工作流已完成。

## 赝势

`pp/` 已准备 GBRV 官方 PBE UPF 包中的 Mg、B v1.4 文件。Mg 为超软势、10 价电子、821 个径向点、3 个原子波函数、7 个 projector；保留 Mg 2s／2p 半芯态，与模板的排除带设置对应。B 文件与仓库原有 PBE UPF 完全一致。

来源：<https://www.physics.rutgers.edu/gbrv/all_pbe_UPF_v1.5.tar.gz>。文件 SHA-256：

| 文件 | SHA-256 |
| --- | --- |
| `mg_pbe_v1.4.uspp.F.UPF` | `84a2ce998da82ed0d4e82253bbad1ee281cae4c3001fceba688e54f450c096bf` |
| `b_pbe_v1.4.uspp.F.UPF` | `01a3696c14b1b245b9741165ffea38120b879286cddbeb956142f8fa922941ee` |

赝势与运行产物按仓库规则留在本地，不纳入 Git。若在新工作区恢复此配置，可下载上述官方包，仅提取这两个文件到 `pp/`，核对上述摘要；不要使用原始二进制 `mg_pbe_v1.4.uspp` 直接代替 UPF。

## 先运行 SCF

```bash
# 只生成当前就绪输入和脚本
epw_phonon -c "input.toml" --stage scf
# 确认脚本后提交；作业结束后重复此命令核验结果
epw_phonon -c "input.toml" --stage scf -s
# 查看状态
epw_phonon -c "input.toml" --status --json
```

已有 Slurm 分配时可使用 `--stage scf -r` 同步执行；不要在登录节点直接运行科学计算。普通阶段命令会复用相容的已完成结果，不应为查询状态加 `--resume`。

## 后续阶段

上一阶段完成并核验后再运行下一条，不要一次连续提交全部阶段：

```bash
epw_phonon -c "input.toml" --stage nscf -s
epw_phonon -c "input.toml" --stage electronic -s
epw_phonon -c "input.toml" --stage phonon -s
epw_wannier -c "input.toml" --stage wannier -s
epw_wannier -c "input.toml" --stage coarse -s
epw_superconductivity -c "input.toml" --stage fine -s
epw_superconductivity -c "input.toml" --stage superconductivity -s
```

第 3～6 步包含阶段内后处理与绘图；原生作业退出后按同一公开阶段命令核验并推进交付。检查 `run/epw-state/` 中文日志、各计算目录的原生 `.out` 和 `run/results/`。排队或运行中的任务不要重复提交；工作区、赝势、初始能窗或计算参数改变时，应按程序提示处理相容性，不手改完成标记。

本配置能准备全部八阶段，但本次安装验收只执行一次小网格 SCF；没有完成完整声子、EPC、能隙或 Tc 科学验收。EPW 6.1 的运行头识别不等于所有原生数据格式已通过验收。

## 2026-11-18 安装验收

- 已重新编译并发布全部 9 个主产品及 `qf-gen-dyn0`；计算节点 `cpu32` 上 56 个主命令／兼容入口的 `--help` 均成功。
- 三个 EPW 入口均通过本配置的 `--check --json`。EPW 6.1 运行头兼容回归及阶段执行回归共 119 项通过、1 项既有环境测试跳过。
- Slurm 作业 `2751176` 在 4 路 MPI 下完成实际 SCF：16 电子，9 次迭代收敛，总能量 −138.11356494 Ry，费米能约 9.1840 eV，原生日志包含 `JOB DONE.`。
- DFTFlow 已核验第 1 阶段，状态为“已完成、可复用”，并生成 `run/1.qe-scf/plots/scf-band-ranges.png`。后续七个阶段尚未计算，完整工作流仍为未完成。
- 原生日志：`run/1.qe-scf/scf.out`；状态日志：`run/epw-state/1.qe-scf.log`；安装、回归与节点验证证据：仓库 `docs/local/2026-11-18-chem-install/`。

继续测试可从 `epw_phonon -c "input.toml" --stage nscf -s` 开始。

## 2026-11-19 更新

已重新获取远程 master（最新仍为 62d7537），重新编译发布全部主产品和 dyn0 工具；三个 EPW 入口的配置检查均通过。该次基于旧 Gitee 源，仍保留了 `[budget]`；此判断已被下述 GitHub 同步更新替代。本次未提交新的科学作业，前述 4 MPI SCF 为历史验收记录。


## 2026-11-19：同步 GitHub 当前配置

已将 GitHub `origin/master` 更新到 `f1e9d1f`（2026-11-19 13:11），重新编译安装，并按现行模板迁移本目录 `input.toml`：

- 删除废弃 `[budget]`，不再配置 `campaign_id` 或 `ledger`。
- 程序路径改为 `[programs]` 中的 `pw_x`／`ph_x`／`epw_x` 等平铺字段，各自保留独立参数数组；后处理程序全部显式指定本机位置。
- 公共参数使用 `[baseparams]`；第 3 步及第 5／8 步用途参数按新版结构嵌套，k／q 网格使用数组。
- 保留 chem 的 PBE 赝势、40／320 Ry、4×4×2 粗 k、2×2×1 粗 q、8×8×4／4×4×2 细网格、16 带／5 Wannier、初始四窗及 20／30／40 K 设置；不以 Ubuntu 新模板的较大网格替换本机测试参数。

旧阶段日志来自旧安装版本，不能用来证明新版本已完成后续阶段；先运行三个 `--check --json`，再按当前程序提示推进。历史计算数据保留，本次不提交新的科学计算。

本次验证：全部主产品及 dyn0 的 release 编译和固定入口发布成功；三个 EPW 命令的新版配置检查通过；native_stage_config、program_paths、superconductivity_plot 共 40 项通过。
