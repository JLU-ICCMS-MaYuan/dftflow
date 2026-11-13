# 本机验证

在仓库根目录使用现有构建环境，测试结束后重新构建覆盖：

    bash scripts/build-native.sh build --release --workspace --locked -j 6

对已完成 SCF 的当前现场运行：

    epw_phonon -c examples/MgB2/ubuntu/input.toml --stage scf -s

应复用 SCF，不使用 --resume，不提交新科学作业。strace -f -e trace=openat 可核对复用不打开原始电荷／波函数；NSCF 新任务实际复制仍会读取这些数据，不能把复制读取误判为核验。

新回归位于 epwflow/tests/unchecked_payload.rs；模块与公开进程验证见验收记录。
