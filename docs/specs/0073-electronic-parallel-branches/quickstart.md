# 验证与使用

已有合法 NSCF 后，在配置目录运行：

```bash
epw_phonon -c "input.toml" --stage electronic -s
```

首次运行应记录能带、DOS、收尾三个作业号。前两个没有相互依赖；第三个依赖前两者退出。12 CPU 单节点可排队依次运行。

失败原因修复后：

```bash
epw_phonon -c "input.toml" --stage electronic --resume -s
```

成功分支不重算；DOS 的投影失败时复用成功 dos。仅图片失败时显式使用 --plot 重绘。

回归：bash scripts/build-native.sh test -p epwflow --test three_stage_execution --locked。
真实验证使用独立 MgB₂ 工作目录，保留原计算；原始电荷与波函数不做核验。
