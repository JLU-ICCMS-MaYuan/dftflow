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
