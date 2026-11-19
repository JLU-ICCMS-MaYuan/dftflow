# 快速验证

## 构建与回归

在仓库根目录使用既有机器环境入口：

```bash
bash scripts/build-native.sh test --locked -p qf-config -p qf-plot -p qf-phonon -p qf-scflow -p qeflow -p scflow -j 4
bash scripts/build-native.sh test --locked --workspace -j 4
bash scripts/build-native.sh build --release --locked --workspace -j 4
```

此入口加载本地忽略的编译环境配置，不需要更改全局 Rust 配置。

## QE 声子只重绘

已有同一次计算的结构、标签、声子频率与 DOS 后，在其配置加入：

```toml
[plot]
formats = ["svg", "png", "pdf", "jpg"]
```

执行：

```bash
qe_process_phonon -c input.toml -i POSCAR --plot
```

在 `phband/phonon_band.*`、`phdos/phonon_dos.*`、`phonon_combined.*` 得到所请求格式；不配置时新生成 SVG，只写 PNG 时不新建其他图格式。旧格式文件保留。此命令不重算 SCF/PH。

## scflow

在现有含 `[scflow]` 的 input.toml 顶层加入同一 `[plot]` 配置，运行 `scflow -c input.toml`。三类图都保留 PDF，再生成配置指定格式；默认 PDF+SVG。完整测试样例见 `scflow/tests/fixtures/scflow_parity/`，是测试合成 EPC 材料，不作科学验收结论。

## 错误与边界

`formats = []` 或 `['gif']` 在配置加载时报错。某额外格式写入失败会告警，其他格式继续生成；全部失败或必需 scflow PDF 失败时报错。VASP、替位 DOS、Wannier 和 EPW 专属图尚未接入本配置。
