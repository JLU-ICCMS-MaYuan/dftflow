# 恢复操作

安装完成后，在当前配置所在目录重新运行公开命令：
```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
cd "/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run"
epw_wannier --stage wannier -s
```

可加 --resume，编号工作区同样读取当前 input.toml。命令创建新尝试，先校验复用成功的 PW／bands，再继续 DOS 和 Wannier。不要 sbatch 已运行过的旧 submit.sh。未代运行上述真实提交，实际科学验收单列。
