# 验证步骤

```bash
cd "/home/mayuan/code/dftflow"
PYTHONPATH="/home/mayuan/code/dftflow" \
  "/home/mayuan/miniconda3/bin/python" -m pytest \
  "/home/mayuan/code/dftflow/test_auto/MgB2/test_scflow.py" -q
```

示例配置：

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

运行：

```bash
scflow -c input.toml
```
