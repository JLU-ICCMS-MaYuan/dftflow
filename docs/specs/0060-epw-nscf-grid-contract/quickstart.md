# 操作与验证

本节面向更新后的源码；本轮不切换用户安装，不运行科学程序。

```toml
[stages."1.qe-scf".pw]
K_POINTS = "automatic\n2 3 6 0 0 0"

[stages."2.qe-nscf".pw]
K_POINTS = [2, 3, 6]

[stages."2.qe-nscf".pw.CONTROL]
calculation = "nscf"
```

其余原生参数仍须完整保留。坐标只在实际 nscf.in 中展开为 36 点。SCF 可以只输出 16 个不可约点；检查目标是 NSCF 输入与输出都为 36。

若提示 QE 增加或改变 k 点，检查报错指明的输入与 XML；显式改本阶段 calculation="bands"，仍使用 pw.x，按已有失败恢复步骤重新运行 NSCF。不使用后处理 bands.x 代替计算，也不自动重复运行。

FFT 不同则先检查 XML 的 fft_grid 与 fft_smooth，再在 SCF 和 NSCF 各自 SYSTEM 显式设置兼容尺寸。例如历史 SCF 的密度 nr1=216、nr2=128、nr3=60，平滑 nr1s=120、nr2s=72、nr3s=32；它们必须同时满足当前 QE、赝势与既有 PH 数据的来源约束，不凭示例尺寸冒充完成导入。两套三元组独立，设置一套时必须写全三个正整数。已有 PH 数据保持原样。

外部 SiP₂ save 的正式导入另行处理；本次不重算声子、也不迁移历史数据。

## 由用户实施时的执行顺序

先完成 0059／0060 源码收敛并安装对应版本，再在已有完整 input.toml 的目标工作区执行。下面只到 NSCF，不会请求 PH 阶段。

1. 保持历史 PH 目录只读，在新的配置中填写结构、赝势、程序、各阶段独立参数与相同的 k 网格；FFT 的选择必须与准备接续的 PH 来源核对。
2. 提交新 SCF：

   ```bash
   epw_phonon -c input.toml --stage scf -s
   ```

   等作业完成后，使用同一命令登记并核对结果；不要仅凭 Slurm COMPLETED 判断数据有效。
3. 提交 NSCF：

   ```bash
   epw_phonon -c input.toml --stage nscf -s
   ```

   作业完成后再次使用同一命令登记结果；程序会核对实际 k 点与 FFT。只到第 2 阶段，不使用默认到 phonon 的命令。
4. 若错误指向额外或改变的 k 点，显式把本阶段 calculation 改为 bands，然后：

   ```bash
   epw_phonon -c input.toml --stage nscf --resume -s
   ```

   已验证相容的 SCF 按已有复用规则保留；FFT 不匹配需先纠正参数和来源，不能靠 bands 选项保证修复。
5. 正式导入外部 PH save 的工具与映射尚未在本项实现。完成该项后才能给出 SiP₂ 的 Wannier／EPC 接续命令，不直接运行默认 PH 链来绕过导入。
