# epwflow

Rust 的 QE/EPW 工作流模块。当前功能与限制以 [Overview](../docs/overview/epwflow/README.md) 为入口；完整自动提交、原生两种 Eliashberg 与 Tc 尚未通过验收。

已有的配置检查和输入准备可使用：

```bash
epwflow check -c input.toml --json
epwflow prepare -c input.toml
```

对已完成的原生 EPW 目录，可只读检查，不启动科学程序或提交作业：

```bash
epwflow interpolation check -w /path/to/native-epw --json
```

若目录包含 `quality-reference.json`，同一命令检查独立 QE/PH 参考、指纹和电子/声子误差；质量超差输出诊断并非零退出。若目录只有原生 EPW 输入输出、退出码、wout 和谱，只能报告 `native_epc_only`，不会宣称能隙/Tc 或完整流程成功。格式见 [原生数据契约](../docs/specs/0012-epwflow-interpolation/contracts/interface.md)。

MgB₂ 的 [真实测试手册](../test_auto/MgB2/epwflow/README.md) 区分旧 PBE 三模式冒烟和用户指定 Mori 讲义的新 LDA 输入。新输入已完成独立质量对照，但声子插值尚未满足严格阈值；没有越过该门生成超导结果。精确机器参数和原始输出留在忽略目录，未切换默认安装。
