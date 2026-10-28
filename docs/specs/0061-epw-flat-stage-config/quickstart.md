# 验证步骤

八主阶段配置片段示例（完整运行需要填写全部八个主阶段及所需用途）：

```toml
[stages."1.qe-scf"]
calculation = "scf"
prefix = "MgB2"
ecutwfc = 40.0 # Ry
ecutrho = 320.0 # Ry
conv_thr = 1e-6 # Ry

[stages."3.qe-electronic".dos]
prefix = "MgB2"
degauss = 0.007349864435130998 # Ry
Emin = -100.0 # eV
Emax = 100.0 # eV
DeltaE = 0.01 # eV

[workflow."8.epw-superconductivity"]
task = "gap"
mode = "both"
spectrum_index = 0
```

运行 `CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p epwflow`。该命令不发布默认产品；不得在进程测试运行时重建同一产品。

以下展示同阶段原生同名字段的归属：

```toml
[stages."5.epw-wannier"]
num_iter = 500

[stages."5.epw-wannier".wannier90]
num_iter = 500
bands_plot = true

[stages."5.epw-wannier".postw90]
bands_plot = false
```

历史平层版本 2 仍可正常读取；`Config::to_stage_toml()` 显式导出八主阶段格式，普通序列化保留读入的表达。禁止在同一文件混入 `5.wannier90` 等历史主段。使用完整的 [SiP₂ TOML](../../../examples/SiP2/epwinput.toml) 查看八个主段及原生字段。

新建的隔离工作区可以使用本次构建的 `epw_phonon --check -c epwinput.toml --json` 检查。注意：配置驱动的 CLI 入口会保存配置版本，并自动转换旧可见状态；`--check` 在旧现场不保证文件系统只读。本轮实际 SiP₂ 仅通过 `Config::load` 和显式只读回归读取，不调用该入口、不转换状态、不提交作业，也不改写旧冻结文件。

只读实测需显式指定当前配置及改写前副本：

```bash
DFTFLOW_SIP2_GROUPED_CONFIG="/data/home/mayuan/code/dftflow/examples/SiP2/epwinput.toml" \
DFTFLOW_SIP2_PREVIOUS_CONFIG="/path/to/previous/epwinput.toml" \
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow --lib \
  real_sip2_grouped_configuration_is_read_only_and_scientifically_identical -- --ignored --nocapture
```

公开 CLI 与恢复、旧状态转换测试使用隔离临时目录及科学替身，实际 SiP₂ 的状态转换由用户按 [算例说明](../../../examples/SiP2/README.md) 显式执行。
