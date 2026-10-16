# MgB₂ 原生 QE/EPW 真实测试

本目录用于 Issue 0015 的真实程序测试。手册和检查器纳入 Git，机器输入、运行脚本、原始输出和预算证据保存在忽略的 `local_run/`。本轮复用已有 SCF/PH，没有重新执行 SCF 或 PH。

## 2026-10-16 结果

使用 QE 7.4.1 配套的 **EPW 5.9**，在计算节点执行；以下三套均正常退出并通过原生产物冒烟检查。

| PH 来源 | 本地运行目录（相对本目录） | 原生直接求和 λ | 第一列谱积分 λ |
| --- | --- | ---: | ---: |
| nosplit | `local_run/0015-native/native-nosplit/` | 0.6509531 | 0.6509929 |
| coords | `local_run/0015-native/native-coords/` | 0.6509533 | 0.6509931 |
| index | `local_run/0015-native/native-index-02/` | 0.6508502 | 0.6508900 |

已完成：完整均匀 NSCF、Wannier 解纠缠和局域化、粗网格电子声子数据读取、细网格插值、α²F 和 λ。每份谱有 500 个能量点、十列声子展宽；第一列展宽为 0.5 meV。直接求和 λ 与展宽后的谱积分是不同量，保留两者。

**尚未完成严格数值验收。** 相对 nosplit，coords 的第一列同序号谱值 L1 诊断差约 `3.94e-6`，index 约 `9.71e-3`；能量轴最大差分别约 `1.3e-6`、`6.198e-4` meV。没有重采样或平滑，以上只是定位差异的诊断，不能冒充 Spec 要求的同一能量轴对照通过。index 谱形需要继续查明；λ 单项容差通过不替代全谱验收。

尚未验证独立 QE/PH 点上的插值质量、原生 Eliashberg 能隙/Tc、六组合完整对照或 epwflow 自动入口。原生 EPW 日志附带的经验 Tc 不作为 Eliashberg 结果。小网格仅用于调试工作流，不作为收敛的 MgB₂ 材料预测。

## 固定测试输入

| 参数 | 本轮值 |
| --- | --- |
| SCF/PH 来源 | 原结构、赝势和 40/320 Ry 截断；SCF 展宽 0.02 Ry，不修改来源 |
| 粗电子 / 声子网格 | 4×4×2 / 2×2×1；2 个不可约 q 点 |
| NSCF | 显式完整晶体坐标网格，32 点、14 带，`conv_thr=1e-10` |
| 对称性 | EPW 5.9 适配保留 NSCF 晶体对称性；实际输出检查点集合和归一化权重 |
| Wannier | 5 个函数，排除原始带 1:4；B:pz 与三个分数坐标 s 投影 |
| 能窗 | 外窗上界 25 eV，冻结窗 −10～9.3 eV；底层提示忽略 `dis_win_min`，实际下界由排除带控制 |
| Wannier 迭代 | 局域化/解纠缠上限各 500，阈值 `1e-8`，连续窗口 5 |
| 细电子 / 声子网格 | 8×8×4 / 4×4×2，完整均匀网格 |
| Fermi 能 / 窗口 | 9.1714 eV / 2 eV |
| 谱参数 | 电子展宽 0.1 eV，声子展宽 0.5～0.95 meV（十列），500 点，声学截断 2 cm⁻¹ |
| 实际资源 | 常规测试每作业 1 节点、8 MPI、8 GiB、15 分钟上限；单进程调试另记 |

精确输入、结构、程序身份及每次变更原因见各运行目录的 `provenance.json`、`epw.in`、`nscf.in` 或其明确的 NSCF 来源。所有失败尝试保留。历史名称含 `index` 的某个目录实际为 coords，不能用目录名代替输入/映射检查；本表 index 使用核实过 `start_q/last_q` 的另一份历史来源。

## 直接运行方式

以下是原生接口测试命令，**不是当前 epwflow 自动链已经可用的声明**。当前 `epwflow submit/run` 的实现限制见 [Overview](../../../docs/overview/epwflow/README.md)。

在登录节点先登记共享预算、核验来源、准备独立目录及 Slurm 脚本，再以原生 `sbatch --parsable submit.sh` 提交。不要在登录节点直接执行 MPI 科学计算。`QE_BIN`、`EPW_EXE`、`ENV_SCRIPT`、`TEST_ROOT` 是本地机器路径参数；本轮精确脚本已保存在各运行目录。

计算节点脚本中的关键调用为：

```bash
source "$ENV_SCRIPT" --force
export OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
ulimit -s unlimited

# 仅在没有经核验的相容完整 NSCF 保存树时执行：
mpirun -np 8 "$QE_BIN/pw.x" -nk 8 -in nscf.in > nscf.out 2>&1

# tmp 是独立副本，save 含与物理 q 点及扰动基底配对的 PH 数据。
set +e
mpirun -np 8 "$EPW_EXE" -nk 8 -in epw.in > epw.out 2>&1
epw_rc=$?
set -e
printf '%s\n' "$epw_rc" > epw.exitcode
test "$epw_rc" -eq 0
python3 "$TEST_ROOT/verify_native_epw.py" . > native-smoke.json
```

本轮三个成功目录可直接重新检查，不会提交作业或启动 QE/EPW：

```bash
python3 test_auto/MgB2/epwflow/verify_native_epw.py test_auto/MgB2/epwflow/local_run/0015-native/native-nosplit
python3 test_auto/MgB2/epwflow/verify_native_epw.py test_auto/MgB2/epwflow/local_run/0015-native/native-coords
python3 test_auto/MgB2/epwflow/verify_native_epw.py test_auto/MgB2/epwflow/local_run/0015-native/native-index-02
```

检查器专用于本套固定 MgB₂ 输入。它核对进程退出码、运行版本、实际 XML 完整网格/权重/本征值、两项 Wannier 收敛、原生谱形状/有限性/非零性及各列 λ 积分；不会声明独立质量参考或自动工作流已通过。

## 文件与故障边界

- `epw.in` / `epw.out`：本轮原生 EPW 输入与输出；`epw.exitcode` 单独记录 MPI 命令退出码。
- `MgB2.wout`：必须分别确认解纠缠和局域化收敛，不能只查 `Final State`。
- `MgB2.a2f.01.300.000`：原生谱；后缀温度不代表已运行该温度的超导能隙求解。
- `tmp/MgB2.ephmat/`、`epwdata.fmt` 等：原生中间数据，不仅凭存在判断后续可复用。
- `save/MgB2.dyn_q<i>`、`MgB2.dvscf_q<i>`、`MgB2.phsave/patterns.<i>.xml`：目标版本读取布局；coords 需核对内部 `QPOINT_NUMBER` 与全局编号，位移向量和响应势必须配对。
- `native-smoke.json`：冒烟检查结果，显式区分未完成的质量/超导/自动链验收。
- `local_run/0015-native/audit.json`：三套结果、原始数据哈希核对、负向检查、失败尝试和预算证据。

已复现并定位的接口问题包括：`phonselfen` 不接受 `mp_mesh_k=true`；强制无对称 NSCF 保存树在该 EPW 5.9 中触发崩溃；coords 内部编号未转换；旧 index 合并根目录存在零字节 Γ 点响应势；EPW 5.9 正常结束不打印 PW/PH 的 `JOB DONE`。生产输入层已修复对称性设置，其余归集和结束检查仍须接入生产编排并回归。

本轮新增计算累计 67 节点秒；保守计入既有仓库计算后共 667 节点秒，低于 8 节点小时授权。账本为本地独立测试记账，不冒充 epwflow 生产预算机制已通过。保留原数据，未切换安装、未操作线上 Issue。
