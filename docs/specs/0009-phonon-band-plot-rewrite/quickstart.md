# 已有结果仅重绘

在包含 POSCAR、归档 phband/phdos 的独立副本中配置 [qe_phonon].work_dir 指向该副本：

```bash
qe_process_phonon -c input.toml -i POSCAR --plot
```

仅重新写出 CSV 和图，不运行 QE。产物为 phband/phonon_band.png、phdos/phonon_dos.png、phonon_combined.jpg。重绘前保存旧图；KPATH/labelinfo 必须属于同一计算晶胞与频率采样。安装和完整验收记录见 acceptance.md。
