# 0053：验证与使用

## 隔离回归

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --locked -p epwflow --test three_stage_execution
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --locked -p epwflow --test slurm_log_filenames --test superconductivity
bash scripts/build-native.sh build --release --locked -p epwflow --bins
```

测试实际执行 Bash 与受控科学程序，覆盖实时输出、退出非零、旧日志、分配内链、两方法收尾和恢复；不提交真实集群任务。

## 使用

运行新编译并按既有方式安装的 CLI，在新任务中查看对应 `.out`；不再依赖科学程序 `.err`。Slurm 自身日志沿用原设置。已有任务继续按记录版本续算，不编辑绑定脚本或删除旧 `.err`。本次具体编译及安装边界见 [验收](acceptance.md)。
