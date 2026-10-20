# 技术依据与取舍

- 参考只读 /data/home/mayuan/code/my_script/epw：输入与提交脚本共目录，生成时读取当前配置；生产实现保持 Rust/Bash。
- EPW 5.9 epw.f90 在 ep_coupling 分支调用 wann_run 或 loadbm，elph 单独控制后续 EPC；仅设置 ep_coupling=false 无法执行拟合。wannierization.f90 强制 write_hr，可供原生电子后处理。
- 旧 RunBinding 整份固定配置的相等约束只适用于旧运行；不能作为新工作区的缓存键。科学阶段键排除资源，必须加入上游产物身份及求解器身份。
- EPW epwread=true、epbread=false 可读取便携粗网格；细矩阵分片/原生重读仍须核对格式和布局。
- 旧数据一次性复制并重定位清单，拒绝可写链接，避免源数据被下游覆盖。旧现场不为迁移改写。
- Rust 阶段枚举、目录映射、清单与预检共享；不复制三份入口或再维护一套 Python 调度。
- 原生 Wannier90／postw90 的固定高斯 DOS 使用 `dos_adpt_smr=false`、`dos_smr_fixed_en_width`、`dos_smr_index=0`；QE `dos.x` 使用相同物理展宽。非磁标量 Wannier DOS 另乘两重自旋，原始数据不覆盖。
- 原生 EPW 完整细网格矩阵可由 `read_ephmat` 按新 pool 重新分配；失败计算的部分分片不直接跨 MPI 布局重读，归档后重新执行该原生子任务。成功的超导模式使用不含资源的绑定键保留。
- 绘图、DOS 展宽及比较路径产物具有独立后处理指纹，不纳入粗网格 EPC 完成清单；路径密度变化只重建路径参考与对比，不触发拟合或 EPC。展宽改回旧值同样与当前已发布表格比较，避免错误命中旧配置。
- 真实导入同时核验旧独立质量参考及其来源绑定，复制可移植参考数据并建立新记录；保留旧来源，避免仅因目录迁移再次执行独立 QE 参考。
