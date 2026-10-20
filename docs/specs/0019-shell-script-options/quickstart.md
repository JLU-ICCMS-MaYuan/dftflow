# 快速验证

在仓库根执行；不需要 QE／EPW 或 Slurm 科学计算。

1. 用 `git ls-files` 列出跟踪的 shell 文件，逐份执行 `bash -n`。
2. 审计 shell 文件及 Rust／Python 生成器，确认不再自动开启严格选项；文档原文与历史验收中的描述不算可执行注入。
3. 执行 `bash scripts/build-native.sh test --locked -p qf-runner -p qf-phonon -p df-substitution -p qeflow -p wannier90flow -p epwflow`。
4. 使用已构建程序目录运行 scripts/tests/install-compat.sh、install-current.sh、install-bin.sh；环境入口存在时执行 scripts/tests/env.sh。
5. 复核关键失败被非零退出码保留，记录实际通过数、未运行的平台构建及当前源码／安装的区别。
