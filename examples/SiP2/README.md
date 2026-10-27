# SiP₂：复用历史声子的电子计算输入

`epwinput.toml` 使用 schema 2，与支持 0059/0060 的程序配套。`POSCAR` 从历史 SCF 输入保留原始晶格和 24 个原子的分数坐标；`KPATH.in` 沿用历史 Wannier 的 12 段路径。赝势使用配置中已有文件的绝对路径。

| 参数 | 当前设置 |
| --- | --- |
| SCF / NSCF 均匀网格 | 2×3×6，零位移；NSCF 完整 36 点 |
| 密度 / 平滑 FFT | 216×128×60 / 120×72×32，SCF 与 NSCF 显式一致 |
| 截断能 | 60 / 720 Ry |
| NSCF 能带 / Wannier 函数 | 96 / 96，Si:sp3、P:sp3 |
| 细电子 / 细声子网格 | 8×12×24 / 4×6×12 |
| μ* / 温度 | 0.1 / 5、10、20 K |

细网格、温度和 μ* 是用户确认的起始设置，尚未做收敛验证。历史 Wannier 的迭代上限为 500；当前配置采用流程的 5000 上限和收敛检查。费米能从新 NSCF 读取，不固定历史数值。其余数值迭代和后处理设置也需要后续核验。

## 当前执行边界

外部声子来源为：

```text
/data/home/mayuan/work/SiP2/data/Pbam/exp_8K_relax_2026-09-29/SiP2_22.2GPa/qe_phonon_2x3x6_inspur/epw_phonon_extract/save
```

当前原始 `save/` 导入尚未接通，不能直接进入 Wannier 或超导阶段。配置中的第 4 阶段用于满足完整配置要求，运行它会重算声子；**不要运行默认声子入口或 `--stage phonon`，也不要提交全流程。** 同一 FFT 设置还需要在声子导入时与来源数据逐项核验。

当前只准备重新计算电子 SCF、NSCF。下列命令通过仓库环境脚本加载正式 CLI，`epw_phonon` 应解析为 `/data/home/mayuan/code/dftflow/bin/epwflow/epw_phonon`。不再使用 `docs/local/0059-native-bin` 验证副本。这里只执行了 `--check`，未执行下面的提交命令。

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
command -v epw_phonon
cd "/data/home/mayuan/code/dftflow/examples/SiP2"

# 只读检查，不提交作业。
epw_phonon \
  --config "epwinput.toml" --stage nscf --check --json

# 第一步：用户提交 SCF。
epw_phonon \
  --config "epwinput.toml" --stage scf -s

# 等待 SCF 作业成功结束后，再运行同一命令登记完成状态。
epw_phonon \
  --config "epwinput.toml" --stage scf -s

# 第二步：SCF 完成状态登记成功后，用户提交 NSCF。
epw_phonon \
  --config "epwinput.toml" --stage nscf -s

# 等待 NSCF 成功结束后，登记完成并执行输入/输出证据校验。
epw_phonon \
  --config "epwinput.toml" --stage nscf -s
```

NSCF 配置只保存 `[2, 3, 6]`。Rust 按 `kmesh.pl` 的顺序和数值格式展开到 `nscf.in`，完成后比较 NSCF 输入、输出的完整点数和网格。若校验失败，应先保留报错证据，再考虑显式改用 `pw.x` 的 `calculation='bands'`；这是 `pw.x` 的运行模式，不是后处理程序 `bands.x`。不会自动切换模式或重试。
