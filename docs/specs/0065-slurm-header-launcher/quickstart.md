# 验证入口

使用 examples/MgB2/ubuntu/input.toml 的 launcher 与 slurm.header。构建使用 bash scripts/build-native.sh build --release --workspace --locked；成功后固定 bin/<模块>/ 命令被覆盖。检查 epw_phonon -c input.toml --check --json，再默认生成目标阶段脚本检查 header。不在本项新增真实科学计算。
