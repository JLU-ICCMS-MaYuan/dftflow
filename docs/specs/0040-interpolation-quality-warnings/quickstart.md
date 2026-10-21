# 操作与验证

进入当前算例并使用已经安装的公开程序：

```bash
source /data/home/mayuan/code/dftflow/scripts/env.sh
cd /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow
epw_superconductivity -c epwinput.toml --stage fine -s
```

程序重验既有 QE／PH、拟合及粗 EPC；原先只因数值质量门未发布的粗交付将按新策略继续处理。出现 quality_warning 时检查实际误差和理想值，但无需修改阈值或增加跳过参数。已有原质量报告的 passed=false 保留。等待本次作业终态并核对细网格交付后，再执行：

```bash
epw_superconductivity -c epwinput.toml --stage superconductivity -s
```

参数仍由本目录 epwinput.toml 管理。不要修改已提交的冻结脚本，不要同时启动另一个修改工作区的命令。数据缺失／损坏、非有限数、原生失败及未知锁仍报错停止；原生求解未收敛要查看原始日志和尝试记录，不能当作数值告警掩盖。

源码验证用规范 scripts/build-native.sh test --locked -p epwflow；默认真实数据依赖的跳过项不计通过。真实作业、误差、谱／求解与预算分别写入[验收记录](acceptance.md)，协议替身通过不代表科学结果已通过。
