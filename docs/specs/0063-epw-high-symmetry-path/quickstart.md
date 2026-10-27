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
