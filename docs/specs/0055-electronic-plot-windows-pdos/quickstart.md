# 使用与验证

## 构建与默认出图

```bash
CARGO_PROFILE_RELEASE_DEBUG=0 bash scripts/build-native.sh build --release --locked -p epwflow --bins
source scripts/env.sh
```

在计算工作区按原公开命令运行第 3 或第 5 步；作业内分别交付两种范围。第 3 步新增一次必要的 projwfc 元素投影后处理，已有相容投影会复用，不重算 SCF／NSCF／PH／Wannier／EPW。第 5 步只比较 TDOS。

第 3 步 `electronic-bands.png`／`electronic-dos.png` 为费米能上下 5 eV；第 5 步同名两个文件使用实际 dis_win_min／max 相对 E_F 的范围；`*-valence.png` 为共同 QE 最低占据态到费米能上方 5 eV。最终图片和对应数据在根 `results/3.qe-electronic/` 与 `results/5.epw-wannier/`。

费米线红色，插值蓝色，其他线／普通标签避开红蓝；每张 PNG 为 3000×2100、约 300 DPI。比较指标仍沿用原科学窗，不因换显示范围而改变。

## 验证入口

```bash
CARGO_PROFILE_RELEASE_DEBUG=0 bash scripts/build-native.sh test --release --locked -p epwflow -p wannier90flow -p qf-plot --no-fail-fast -- --test-threads=1
```

真实后处理测试为显式 ignored 用例，需给定已有 PBE 来源、新输出目录及 Slurm 分配。使用 `test_auto/MgB2/epwflow/electronic_0055.sh` 调用已构建的测试程序；测试程序留在仓库 `bin/.build`，运行数据留 `test_auto/MgB2/local_run`。不得在登录节点执行科学程序或覆盖旧验收目录。

用 `verify_electronic_0055.py <结果目录>` 检查 8 图、相同能窗、DPI、红蓝配色和真实 Mg／B 投影。实际作业、失败修复及验证边界见验收记录。

## 隔离原始来源后重绘

有 bubblewrap 的 Linux 环境可将原计算目录完全排除，只映射系统库、已构建程序和结果副本：

```bash
env -i PATH=/nonexistent /usr/bin/bwrap --unshare-all --die-with-parent \
  --ro-bind /usr /usr --ro-bind /lib /lib --ro-bind /lib64 /lib64 \
  --ro-bind /etc/fonts /etc/fonts --proc /proc --dev /dev --tmpfs /tmp \
  --ro-bind "/绝对路径/bin/epwflow/epw_wannier" /epw_wannier \
  --bind "/绝对路径/结果副本" /results --chdir /results \
  /epw_wannier --plot --results /results
```

示例中的程序需为本机可直接启动的 ELF；计算节点的私有加载器包装程序还须一并映射其仓库运行库。实际本机验收使用 `bin/.build/.../release/epw_wannier`。原 `/data` 没有映射，源科学目录在进程内不存在。以同一映射改用 `--results-check` 可核验包内完整性；重绘前后比较 CSV／plot.json 哈希。不要删除真实科学目录来制造不可访问条件。

## 已有图片更新线型与实际外窗

在工作区用 `epw_wannier --plot --results "./results"` 重绘，不调用科学程序。所有 QE 参考／TDOS 为深灰实线，EPW／Wannier 为蓝色短虚线，QE 元素 PDOS 使用不同短虚线；费米线仍为红色。该入口重绘整包，不限制为某个阶段。`epw_phonon --plot` 仍不提供此操作。
