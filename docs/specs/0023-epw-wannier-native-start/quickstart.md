# 本地验证和真实恢复

在仓库根目录通过 `bash scripts/build-native.sh test --locked -p epwflow --test native_outputs` 验证原生输入契约；编号连续进程回归在 `three_stage_execution`。使用规范构建和分模块安装，不能直接复制不兼容构建。

实际操作：

```bash
source /data/home/mayuan/code/dftflow/scripts/env.sh
cd /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run
epw_wannier --stage wannier -s
```

只提交新的公开尝试，不手工 sbatch 已绑定旧作业的脚本。用 sacct、原生日志、阶段收据和对比图检查，提交返回不代表拟合成功；本轮由助手执行真实测试。

## 后处理续算的可见证据

原生拟合收据已存在而后处理失败时，下一次 `-s` 显示 `wannier: reuse` 仍会为后处理创建新任务。成功后应得到 `5.epw-wannier/electronic-bands.png`、`electronic-dos.png`、DOS CSV 和规范化说明；`.epwflow/post/wannier.json` 记录本次后处理结果。`.win/.eig` 由同一 NSCF 自动补齐，不需手写。

不能只看外部程序退出码：当前串行 W90 的错误可返回零。程序同时检查本次原生日志和 `.werr`，旧后处理产物先归档，成功拟合收据保持可复用。
