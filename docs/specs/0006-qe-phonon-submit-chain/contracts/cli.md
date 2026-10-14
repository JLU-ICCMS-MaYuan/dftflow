# CLI 契约

qe_phonon_split_index / qe_phonon_split_coords 的 -c/-i 不变；无 -r/-s 仅准备，不要求预先有 XML；-r 完整顺序运行；-s 自动安排完整链路，二者互斥。qe_split_submit --chain --mode index|coords 安排 q 点和 post，原独立提交行为保留。内部阶段接口仅供生成脚本使用。

所有生成脚本使用绝对路径、当前 qeflow 制品路径与固定工作目录。模式改变需要新目录。运行例：bash examples/MgB2/prepare.sh sugon-amd index；进入对应 run/index（最终路径以 quickstart 为准）后 bash submit_phonon.sh。

失败退出非零；成功提交只报告作业号；最终成功由结果检查决定。只保留一个 SCF 设置块，不增加第二套网格。
