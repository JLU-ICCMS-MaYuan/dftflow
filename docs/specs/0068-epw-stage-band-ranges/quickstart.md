# 快速验证

新计算按原命令执行，阶段分配内科学数据核验完成后即绘图：

```bash
epw_phonon -c "input.toml" --stage scf -s
epw_phonon -c "input.toml" --stage nscf -s
epw_phonon -c "input.toml" --stage electronic -s
```

已有有效结果重复执行会复用并补图。不得为补图增加 --resume 强制重算 SCF。也可通过 epw_wannier 的正常前置复用补齐三图，非法 Wannier 窗口仍会报错，已完成诊断保留。

检查各阶段 plots 下的 *-band-ranges.png／svg／csv／json，以及 results/<阶段>/band-ranges 的发布。electronic 的图使用路径样本及 NSCF E_F，不得与 NSCF 网格范围混为一谈。

回归：bash scripts/build-native.sh test -p qf-wannier -p epwflow --locked；部署：bash scripts/build-native.sh build --release -p epwflow --locked -j 6。本机绘图是串行派生处理，不宣称 12 MPI；若新增科学任务才遵循本机 launcher 12 路约定，本次不新增。
