# 0063 验证入口

## 端点与短路径

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow --lib electronic_sampling::tests -- --skip real_workspace_wdata_readonly
```

要求路径行数随真实边数变化，不随插值密度或 NSCF 网格增长；非法端点有可定位错误。

## 原生调用与交付

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow --test three_stage_execution
```

此项使用隔离进程替身，不属于真实科学验收。比较口径已按用户答复固定；不能以端点单元测试通过代替该项。

本轮不重算用户工作区，不切换安装、不调用 Slurm 或推送。真实原生验证需要独立记录输入、软件和输出身份。`bands_num_points` 可设为所需的首段间隔数（例如 `100`）；后续段由 Wannier90 按实际段长比例取整，过低设置会在计算前被拒绝。

## 自定义文件头修复后的恢复

上文“不切换安装”是初次实施的边界。本次修复的验证及正式 CLI 发布另见 [验收记录](acceptance.md)。完整文件头的标题可以自定义，不要为绕过解析错误修改 `examples/SiP2/KPATH.in`，否则电子参考的来源哈希也会改变。

从 SiP₂ 目录加载正式入口后，使用当前 `s,p`、500 次上限配置显式恢复拟合：

```bash
cd "/data/home/mayuan/code/dftflow/examples/SiP2"
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
epw_wannier --config "epwinput.toml" --stage wannier --resume -s
```

失败尝试归档后按当前配置生成新输入。已有 SCF、NSCF、电子参考和声子必须通过原有复用校验；不得删除状态文件强制复用。作业结束后，用不带 `--resume` 的命令归集及推进：

```bash
epw_wannier --config "epwinput.toml" --stage wannier -s
```

每次有新原生任务提交时先等其结束。文件头兼容修复不意味着新模型一定收敛，也不替代 Wannier 完成回执。
