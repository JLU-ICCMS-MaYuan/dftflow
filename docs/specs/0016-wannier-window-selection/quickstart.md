# 默认组合选窗的使用与验证

> 2026-10-19 最新取舍：用户已放弃能量＋投影组合方案并要求撤回新原生构建。下文保留上一阶段事实；不再作为后续实施／使用建议。新的兼容纯能量建议见 [重新设计调研](energy-window-redesign.md)，尚未替换现有 Rust 实现。

## 构建与回归

```bash
bash "scripts/build-native.sh" test -p qf-wannier -p wannier90flow --locked --offline
bash "scripts/build-native.sh" test -p epwflow --test interpolation --test native_outputs --test window_selection --test qe_data --locked --offline
```

使用本次构建的二进制核查，不把系统已安装命令当成本次制品：

```bash
DFTFLOW_BUILD_BIN="$PWD/target/linux-x86_64/x86_64-unknown-linux-gnu/debug"
"$DFTFLOW_BUILD_BIN/wannier90" window "/path/to/data-file-schema.xml" --num-wann 5
```

默认从实际 QE XML 读取绝对 eV 本征值和 E_F；输出选择 JSON，不启动科学程序。排除带用 `--exclude-bands 1,2`；非 QE XML 的 EIGENVAL 输入需显式提供绝对 `--fermi`。`-n`／`-e` 仅作只读分析。旧 `--auto nbnd1 num_wann nbnd3` 已弃用并报错，没有替代默认算法的兼容回退。

## 初始化输入

在已有完整匹配的 QE NSCF 及结构基础上，配置示意如下；实际投影和网格必须与物理基底及源数据匹配，不能把示意参数当作已收敛方案。

```toml
[wannier_init]
system_name = "mgb2"
num_wann = 5
bands_file = "/path/to/nscf/mgb2.save/data-file-schema.xml"

[wannier_init.projections]
list = ["B:pz", "f=0.5,1.0,0.5:s", "f=0.0,0.5,0.5:s", "f=0.5,0.5,0.5:s"]

[wannier_init.k_points]
mp_grid = [6, 6, 6]

[pw2wannier90]
prefix = "mgb2"
outdir = "/path/to/nscf/tmp"
seedname = "mgb2"
```

`bands_file` 相对路径按配置文件目录解析。初始化生成 `.win` 及同名前缀的 `.window-selection.json`；默认加入 `dis_proj_min/max=0.01/0.95`、投影启用开关及实际冻结上界，不补造另外三个边界。明确填写的四界作为上游相对平移值覆盖，仍受态数上限约束；不能直接沿用过去按绝对值填写的配置而不核对参考。

```bash
"$DFTFLOW_BUILD_BIN/wannier90" init -c "/path/to/inputwannier.toml" -i "/path/to/POSCAR"
```

不带运行／提交标志只生成输入。执行前要求实际 W90 可核验投影参数支持；生成脚本按 `-pp → pw2wannier90 → wannier90` 顺序执行，前一步失败后停止。

## epwflow 接入

完整配置继续使用既有结构、资源和数据契约。若希望使用上游默认边界，移除原先显式的 `outer_window_ev`、`frozen_window_ev`、`frozen_max_ev`；它们一旦明确提供就属于覆盖值，不是默认值。生产入口从有效 F2 清单绑定本征值和共同 E_F；独立 `.win` 和 EPW `wdata` 使用相同选择。

实际程序无法核验支持 `dis_proj_min/max/dis_froz_proj` 时返回 `unsupported_version`，不会退回只用能量窗。当前本机程序的只读能力证据见 [验收](acceptance.md)。本次未升级科学程序，也没有通过新窗口实际科学运行；已有科学质量门仍适用。

## 与固定上游差分

测试脚本仅需要 Python／NumPy，不安装或运行 AiiDA；它从子仓库解析并执行实际 `prepare_inputs` 函数，以轻量对象替代框架数据容器。

```bash
python3 "test_auto/MgB2/epwflow/verify_window_policy.py" \
  --binary "$DFTFLOW_BUILD_BIN/wannier90" \
  --qe-xml "/path/to/data-file-schema.xml" --num-wann 5
```

检查包含固定数值用例、固定种子的形状／排除带组合和可选真实能谱；不执行候选科学扫描。子模块提交不匹配时直接拒绝，避免悄悄改变比较基准。
