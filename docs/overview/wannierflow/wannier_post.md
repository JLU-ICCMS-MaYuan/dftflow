# wannier_post：Wannier90 DOS 后处理

[返回家族索引](README.md)

## 功能与入口

为已有 .win 和 Wannier 结果生成 postw90 脚本；-r 分支按配置轮换投影 DOS。

等价入口：`wannier90 post`。

## 使用方法

```bash
wannier_post -c inputwannier.toml -i wannier90/MgB2.win
wannier_post -c inputwannier.toml -i wannier90/MgB2.win -r
```

-c 默认 inputwannier.toml；-i 必填，接受 .win 文件或其前缀；-r 直接运行，-s 提交。

## 输入、输出与衔接

工作目录为 .win 所在目录；[postw90].executable_path 指定执行命令，[wannier_post] 控制 DOS。-r 临时写各组 dos_project、运行 postw90、归档为 `prefix_dos_<tag>.dat`，最后恢复原 .win。

## 使用边界

-s 只提交 run_postw90.sh，当前未包含 -r 中修改 .win、轮换投影和恢复的逻辑。需要配置轮换时，应在计算资源内用 -r。

## 实现依据与关联

[CLI 定义](../../../wannier90flow/src/bin/wannier90.rs)；[配置模板](../../../wannier90flow/src/config.rs)；[后处理实现](../../../wannier90flow/src/post.rs)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
