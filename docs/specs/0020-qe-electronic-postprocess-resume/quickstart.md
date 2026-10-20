# 修复后的恢复操作

```bash
source /data/home/mayuan/code/dftflow/scripts/env.sh
cd /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run
epw_wannier --stage wannier --resume -s
```

该命令读取当前 input.toml，生成新的冻结尝试。相容且证据完整的 SCF、NSCF、PH 与 PW 能带数据复用；从未完成的 bands.x 后处理继续，再进入 Wannier。不要直接重提手工修改过的旧 submit.sh。本轮交付不代提交上述真实作业，实际结果以验收记录为准。
