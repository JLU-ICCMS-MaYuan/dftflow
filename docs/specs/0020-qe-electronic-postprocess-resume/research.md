# 调查结论

- Slurm：1306688 FAILED 1:0，19:00:46～19:01:08；1306749 FAILED 1:0，19:18:42～19:18:43。
- PW 能带：183 k 点／16 带，JOB DONE；smooth FFT 为 24×24×27。bands.x 启用 32 MPI、单 pool，fft_type_set 返回 6。
- 本机 QE 7.4.1 源码 Modules/command_line_options.f90 支持 `-pd .true.`；FFTXlib/src/fft_types.f90 在 slab 分解无平面时明确建议该参数。
- PP/src/bands.f90 在 lsym／no_overlap 都为 false 时不支持多 pool，所以不能简单复制 PW 自动选择的 4 pool。
- 0018 的电子阶段在重试时归档整目录并重跑 PW，缺原生子任务完成记录。0019 已有独立脚本修改，本缺陷不重复建立 shell 需求。
- 第二次作业因脚本身份改变而拒绝；应生成新尝试，不能弱化哈希保护。
