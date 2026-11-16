# 验证方式

使用独立工作树与独立构建缓存，避免并行任务替换正在运行的调试二进制：

`bash scripts/build-native.sh test --release -p epwflow --locked -j 6 --lib --test three_stage_execution --test native_interpolation --test native_outputs`

检查新生成脚本的 stdin、.out、.exitcode 使用相同主名；旧目录复用原日志且内容不变。命名回归使用轻量外部进程，不提交真实科学作业。已有绘图样式修改与共享构建干扰的边界见 [验收](acceptance.md)。

正式构建使用 `bash scripts/build-native.sh build --release -p epwflow --bins --locked`，成功后覆盖固定入口。并行任务发布期间应分别记录制品来源，不用旧测试进程核验被替换的二进制。
