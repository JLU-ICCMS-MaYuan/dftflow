# 调查证据

2026-10-21：作业 1313468 在 node14、32 MPI 执行 76 秒后失败。SCF、NSCF、电子阶段均有完成记录，PH 日志正常结束，q2r 报 Fortran EOF。原 dyn0 为 0 字节，save 副本为 164 字节。全部 516 文件身份记录及严格清单重验（零差异）保存于忽略的 docs/local/0027-mgb2-epwflow-example-debug/。

真实作业 1313468 的 PH 已完成，run_phonon 将 source_dyn0 复制到同一路径，导致文件长度变为零；q2r 发生 Fortran EOF。save 副本及严格交付清单完整。运行时整阶段归档还会导致后处理重试重复 PH。

q2r 始终从清单中 dyn0 角色的归集副本物化输入。已归集且科学身份相容的 PH 清单须重验文件集合、哈希、原生 q 映射及 SCF/NSCF 数据；通过后保留现场、仅继续 q2r/matdyn。清单损坏拒绝执行，物理参数改变仍走阶段失效，不凭 JOB DONE 复用。
