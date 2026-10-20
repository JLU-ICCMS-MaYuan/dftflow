# 研究与取舍

1. DOS 本地 QE 7.4.1 源码调用 mp_startup 和 read_file_new；公共 command_line_options.f90 接受 -pd .true.，默认 false。真实 dos.err 明确建议 pencil，退出 6。只给 bands／dos 添加该参数，保留当前 MPI，不引入 pool。
2. sacct 证明 1307068 是 sbatch submit.sh，原 attempt.job=1307064。禁止自动抹掉旧 job／reservation：否则会破坏账本和冻结语义。改善现有错误消息，并用公开 -s 重生成测试证明正确恢复。
3. 0020 原生收据的参数键含程序 args：DOS 参数变化只使 DOS 及相应后处理失效，bands 与 PW 键保持一致；沿用原协议，不迁移真实清单。
4. 不默认改 MPI 为 1 或减物理网格，不手改科学输入；不把“JOB DONE”当完整复用证明。
