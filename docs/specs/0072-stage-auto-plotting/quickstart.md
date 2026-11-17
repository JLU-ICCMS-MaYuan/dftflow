# 自动绘图验收与重绘

## 隔离契约测试

```bash
DFTFLOW_THREE_STAGE_TEST_ROOT="/home/mayuan/code/dftflow/docs/local/0072-stage-auto-plotting/retest/cases" \
  bash "scripts/build-native.sh" test -j 2 -p epwflow --locked \
  --test three_stage_execution issue_0072_ -- --nocapture --test-threads=2
```

探针复用既有进程夹具，保存在 `epwflow/tests/support/stage_auto_plotting_0072.rs`。首次临时目标 `issue_0072_acceptance` 的复测日志保留为证据，正式入口改为现有 `three_stage_execution` 测试目标，避免复制整套夹具。替身测试不启动真实科学程序或 Slurm，不称为 12 路科学计算。

## 构建固定入口

```bash
bash "scripts/build-native.sh" build --release -j 2 -p epwflow --bins --locked
```

成功后自动覆盖仓库 `bin/epwflow/` 固定入口；不使用 `target/release` 或额外版本目录。构建失败时不能把旧安装认作本次制品。

## 真实六阶段

使用新的测试目录及其 `input.toml`，依次执行下面六条命令。**每一条提交之后先等对应作业结束，核对退出码、科学完成状态及图片，再执行下一条。** `-s` 返回成功只说明提交成功。

```bash
epw_phonon -c "input.toml" --stage scf -s
epw_phonon -c "input.toml" --stage nscf -s
epw_phonon -c "input.toml" --stage electronic -s
epw_phonon -c "input.toml" --stage phonon -s
epw_wannier -c "input.toml" --stage wannier -s
epw_wannier -c "input.toml" --stage coarse -s
```

| 阶段 | 当前图片目录及内容 |
| --- | --- |
| 1 | `run/results/1.qe-scf/scf-band-ranges.png` |
| 2 | `run/results/2.qe-nscf/nscf-band-ranges.png` |
| 3 | `run/results/3.qe-electronic/`：范围图、近费米能及全能窗电子组合图；正常图含元素 PDOS |
| 4 | `run/results/4.qe-phonon-<模式>/phonon.png`：声子能带／DOS |
| 5 | `run/results/5.epw-wannier/`：近费米能及全能窗电子对比组合图 |
| 6 | `run/results/6.epw-coarse/`：声子对比组合图及实空间衰减图 |

本机科学程序使用 `mpirun -np 12`、Slurm `--ntasks=12`，EPW `-nk 12`；OMP／MKL 线程为 1。小网格出图验收不替代网格收敛或 Tc 验收。

## 仅重绘和当前阶段补图

```bash
epw_phonon --plot --results "run/results/1.qe-scf"
epw_phonon --plot --results "run/results/2.qe-nscf"
epw_phonon -c "input.toml" --stage electronic --plot
epw_wannier -c "input.toml" --stage wannier --plot
```

SCF／NSCF 现场 `--plot` 不支持，使用显式结果目录。结果重绘只读保存的图参，不补算科学数据。自动补图则重复当前阶段命令；缺图不使科学数据失效，也不隐式补上游图片。

仅在测试副本中删除图片或将 PNG 目标替换成目录注入写入失败。不要删除 XML、原始电荷密度或波函数模拟绘图错误。错误路径验收科学完成标记和科学数据是否保留；作业及绘图状态分别记录，不将原生 PDOS 程序失败误报为 PNG 错误。
