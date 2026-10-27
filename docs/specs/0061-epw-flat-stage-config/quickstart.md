# 验证步骤

配置片段示例（完整运行需要填写全部调用）：

```toml
[stages."1.qe-scf"]
calculation = "scf"
prefix = "MgB2"
ecutwfc = 40.0 # Ry
ecutrho = 320.0 # Ry
conv_thr = 1e-6 # Ry

[stages."3.qe-dos"]
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

使用本次构建的 `epw_phonon --check -c epwinput.toml --json` 只读检查当前配置。旧安装尚未识别新表达时不能用它作为验证依据。本轮不修改旧冻结文件、不提交真实作业。
