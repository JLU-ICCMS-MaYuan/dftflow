# 验证步骤

在 dftflow 根目录执行，旧声子测试含仓库相对路径检查：

```bash
cd "/home/mayuan/code/dftflow"
PYTHONPATH="/home/mayuan/code/dftflow" "/home/mayuan/miniconda3/bin/python" -m pytest \
  "/home/mayuan/code/dftflow/test_auto/MgB2/test_matdyn_crystal_coordinates.py" \
  "/home/mayuan/code/dftflow/test_auto/MgB2/test_qe_phonon.py" \
  "/home/mayuan/code/dftflow/test_auto/MgB2/test_phonon_combined_plot.py" -q
```

测试使用临时目录，不需要 QE/vaspkit，不触碰 SiP2 计算数据。预期非立方路径坐标、采样权重、标签距离与声明全部通过，原回归无新增失败。

后续真实后处理仍使用 `qe_process_phonon -c <配置绝对路径> -r`，但必须先准备独立工作区、匹配结构/矩阵/KPATH、配置 QE 可执行目录并加载其动态库。不在本修复中执行实算。历史文件不会自动修复。
