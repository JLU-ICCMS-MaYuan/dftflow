# qe_process_phonon：声子后处理、收集与绘图

[返回 qeflow 索引](README.md)

## 功能与入口

在已有动力学矩阵基础上准备/执行 q2r 与两项 matdyn，也可单独收集和绘图。

等价入口：`qeflow qe-process-phonon`。

## 使用方法

```bash
qe_process_phonon -c input.toml -r
# 已有 QE 后处理结果时，按次序使用以下两条
qe_process_phonon -c input.toml --collect
qe_process_phonon -c input.toml --plot
```

`-c`、`-i` 及 -p/--poscar 别名；普通模式无 -r/-s 只准备，-r 直接运行，-s 提交。--collect / --plot 是立即执行的专用分支。

## 输入、输出与衔接

已有 dyn → q2r → matdyn_phband/matdyn_phdos → phband/phdos 归档 → CSV、PNG 与 phonon_combined.jpg。完整文件清单见 [声子工作流](phonon-workflow.md)。默认后处理工作目录是配置文件目录，可用 qe_phonon.work_dir 明确指定。

## 声子谱与 DOS 绘图

声子谱横轴按倒格矢度量的累计路径距离布置，只显示高对称点标签；GAMMA 显示 Γ。每个 matdyn 采样点与频率逐点对应，连续段共享端点，跳段不连线，同位置端点合并显示为 A|L 等。纵轴为 Frequency (cm⁻¹)，保留负频并显示零频参考线。

独立谱 PNG 与联合 JPG 使用相同路径及绘制逻辑。联合图左谱右 DOS 约 3:1，右侧横轴是 DOS，两侧共享完整频率范围；原子投影按 POSCAR 聚合为元素。--plot 同时更新 CSV，避免旧表格与新图不一致。

路径标签必须覆盖全部频率点，索引严格递增；点数不匹配或非有限数据明确报错，不补零或静默截断。VASPKit 生成路径基于标准化原胞，使用前确认其基底与计算晶胞一致；不会自动替换结构。

## 使用边界

--plot 优先于 --collect；不要合写成 --collect --plot 期待顺序执行。仅重绘需要同一计算的 labelinfo、频率、DOS 与结构，不需要再次运行 VASPKit；生成新 matdyn 输入时才需要 KPATH.in。保证配置 prefix 与结果一致。该入口不重算 SCF/PH；分 q 自动链不需再手工提交它。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[归档逻辑](../../../crates/qf-phonon/src/archive.rs)；[绘图实现](../../../crates/qf-phonon/src/plot.rs)。绘图重写与实测依据见 [0009 验收](../../specs/0009-phonon-band-plot-rewrite/acceptance.md)。

关联 [0002 坐标声明](../../specs/0002-matdyn-crystal-coordinates/spec.md)、[0003 联合图](../../specs/0003-phonon-band-dos-combined/spec.md) 与 [0006](../../specs/0006-qe-phonon-submit-chain/spec.md)。
