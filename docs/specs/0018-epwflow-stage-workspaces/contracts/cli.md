# 命令契约

默认 input.toml；epw_phonon 管理 1～4，epw_wannier --stage wannier|coarse 管理 5/6，epw_superconductivity --stage fine|superconductivity 管理 7/8。省略 stage 执行本程序两步。默认只生成，-s 提交，--resume -s 重新读取当前配置续算。活动/未知作业禁止重复。

每个 submit.sh 定位自己的物理目录，执行冻结的输入与配置，先做同一并行检查。submit_all.sh 只组织依赖。

输入错误、空 pool、数据身份错误和冲突均非零退出；完成状态必须基于校验后的原生产物，不以 sbatch 成功替代。
