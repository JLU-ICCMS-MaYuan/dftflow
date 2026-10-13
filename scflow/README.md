> **0004 迁移说明**：scflow 已全量迁移为 Rust（`scflow/src/bin/scflow.rs` + `crates/qf-scflow`），
> CLI 仍为 `scflow -c input.toml`，输出文件名与数值语义与本文描述一致；科学计算依赖（scipy/matplotlib）不再需要，
> 绘图由 plotters 完成。本文余下内容保留作为物理量与输出格式的参考说明。
>
# scflow

`scflow` 将 A2F 的 QE 后处理代码内置到 dftflow，用于从 `*.dyn*.elph*`、可选 `*.dyn*` 和 `*.ph.out` 计算电子声子耦合、A2F、McMillan/Allen-Dynes Tc 和 Eliashberg Tc。

安装完整依赖：

```bash
pip install -e ".[scflow]"
```

配置 `input.toml`：

```toml
[scflow]
work_dir = "./qe_epc"
results_dir = "./qe_epc/results"
smoothing_thz = 3.0
resolution = 1000
gaussian_sigma = 5.0
mu = 0.1
structure_tolerance = 0.2
integration = "sum"
```

执行：

```bash
scflow -c input.toml
```

程序只读取已有 QE 输出，不启动 QE。结果目录保留 direct/a2f CSV、`result.json`、PDF 图、summary、CIF 和 VASP 结构文件。

## 原生 QE 归档和精确零频

也可将 work_dir 直接指向包含以下文件的计算目录，无需复制或改名：

- elph_dir/elph.inp_lambda.N：每个 q 点的 EPC 数据。
- PREFIX.dynN：q 点星权重（不读取 dyn0、matdyn.out 或备份文件）。
- split_ph.out.N、ph_no_split.out 或 PREFIX.ph.out：可选结构/权重日志。

同一目录同时存在旧 *.dyn*.elph* 与原生 elph_dir 数据时会报错，避免重复计数。

精确 ω²=0 且 λ=0 的模按零贡献处理，原始频率和 γ 不变。每个 result.json 的 zero_frequency_handling 记录该规则、来源、q 点、模编号及原始 γ；若零频模的 λ 非零则拒绝计算。这是明确的零模处理约定，不是恢复缺失的低频谱。正频模仍使用原 A2F 算法；Tc 公式在 float64 上计算。

### ISIF4 真实归档验收

验收脚本仅在显式执行时读取用户归档，不作为普通 pytest 的隐含数据依赖：

```bash
python "test_auto/MgB2/validate_scflow_isif4.py" \
  --archive "/path/to/SiP2-19p2/vasp_opt_ISIF4/Tc-236" \
  --output "/path/to/new-acceptance-directory"
```

输出目录必须尚不存在。脚本检查全部 16 个 q 点、72 个模和 10 个展宽，在 μ*=0.10 / 0.16 下验证 CLI、CSV/JSON/PDF、结构、汇总、独立 Tc 公式与输入哈希。软件验收不等于 Tc 已对网格和低频处理收敛；其他材料的未通过项见 Feature 验收记录。

Eliashberg 结果附 `eliashberg_solver.status`：`resolution_limit` 表示触及 0.001 K 搜索下限，旧 `Tc, K` 数字保留但不可解释为真实 Tc；`resolved` 仅表示当前 25×25 矩阵与搜索步长下找到结果，不代表数值收敛。
