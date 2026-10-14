# CLI 契约

qe_phonon_split_index / qe_phonon_split_coords 的 -c/-i 不变；无 -r/-s 仅准备，不要求预先有 XML；-r 完整顺序运行；-s 自动安排完整链路，二者互斥。qe_split_submit --chain --mode index|coords 安排 q 点和 post，原独立提交行为保留。内部阶段接口仅供生成脚本使用。

所有生成脚本使用绝对路径、当前 qeflow 制品路径与固定工作目录。模式改变需要新目录。运行例：bash examples/MgB2/prepare.sh sugon-amd index；进入对应 run/index（最终路径以 quickstart 为准）后 bash submit_phonon.sh。

失败退出非零；成功提交只报告作业号；最终成功由结果检查决定。只保留一个 SCF 设置块，不增加第二套网格。

## 独立工具与初始化入口（补充）

- 工具：qf-gen-dyn0 --xml <SCF XML> --nq1 N --nq2 N --nq3 N --prefix NAME --out-dir DIR；网格默认 1，输出目录默认当前目录，XML 和 prefix 必填。无配置回退。工具不随主项目安装。
- 主入口：qe_ph_nosplit -c input.toml --dyn0 [-r|-s]，可用 -i POSCAR。无执行参数只准备。配置内 work_dir 相对配置目录，明确 outdir 对应已有 SCF 保存树；不自动寻找其他 SCF 目录。
- --dyn0 使用 ph_init.in/ph_init.out/run_ph_init.sh；成功检查后写 ph_init_complete.json，不刷新完整 PH 为完成。执行失败清除旧初始化成功标记并返回非零，提交失败保留输入但不标记完成。
- qeflow gen-dyn0 子命令移除，主安装入口总数 51。历史数据只读兼容不等于支持旧生成命令。

## 默认 outdir（US4）

`[qe_phonon.scf].outdir` 缺省为 `./tmp`，相对声子工作目录解析；各 q 子任务复制到自己的 tmp。显式配置优先，原有 `outdir="./out"` 继续生效。不修改 CLI 参数、fildyn 或工作目录命名，也不搬迁旧结果。
