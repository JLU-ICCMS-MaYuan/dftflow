# 0044 快速验证

不需要 MPI、QE 或 Slurm 服务；测试使用临时隔离替身。

```bash
cargo test -p qf-runner --test mpi_command
bash scripts/build-native.sh test -p epwflow --test three_stage_execution direct_ --locked
bash scripts/build-native.sh test --workspace --locked
```

生成脚本应为普通 mpirun，先加载环境；缺失启动器、环境失败和原生非零退出不得成功。安装及迁移核验见 [部署记录](deployment.md)，科学数据不重算。
