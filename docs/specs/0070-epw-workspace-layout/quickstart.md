# 快速核验

新建或已采用新布局的工作区按现有三个入口推进。不要为本次目录改造直接删除既有正式计算目录。

```bash
epw_phonon -c "input.toml" --stage phonon -s
epw_wannier -c "input.toml" --stage wannier -s
epw_wannier -c "input.toml" --stage coarse -s
epw_superconductivity -c "input.toml" --stage fine -s
epw_superconductivity -c "input.toml" --stage superconductivity -s
```

只在需要恢复科学失败时显式添加 `--resume`。绘图失败应保留并显示原因，普通运行／恢复不自动重试失败图片。用户明确请求重绘时使用已有绘图数据：

```bash
epw_wannier --plot --results "run/results/6.epw-coarse"
```

检查文件归属、方法目录和清理边界时只使用本目录的规格与验收记录。结果单独移走后可查看图片；依赖原始数据的重绘需保持整个工作区的相对关系。历史自包含结果包仍由其原格式读取器处理。
