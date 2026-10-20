# 快速操作

```bash
source /data/home/mayuan/code/dftflow/scripts/env.sh
cd /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run
# 修改 input.toml 后，生成阶段原生输入和脚本：
epw_phonon
# 提交相应阶段；程序核对结果并复用相容产物：
epw_phonon -s
epw_wannier --stage wannier -s
epw_wannier --stage coarse -s
epw_superconductivity --stage fine -s
epw_superconductivity --stage superconductivity -s
```

本轮只进行生成、协议回归及旧数据导入，不代用户提交真实任务。命令是否已交付以 acceptance.md 的实现证据为准。
