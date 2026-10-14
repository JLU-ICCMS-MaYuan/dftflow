# substitution：批量组织替换计算

[返回家族索引](README.md)

## 功能与入口

生成替换案例与脚本，组织可选 MatterSim、VASP 优化/SCF/DOS/后处理和 QE 声子阶段。

等价入口：`substitution workflow`。

## 使用方法

```bash
substitution -c input.toml -o cases
# 等价显式子命令
substitution workflow -c input.toml -o cases
```

-o 必填；-c 默认 input.toml；无执行标志只准备；-r 运行、-s 提交，两者互斥。使用前先读下方限制。

## 输入、输出与衔接

[substitutionflow].phonon_mode 接受 nosplit、split_index、split_coords（与算例 prepare.sh 的 index/coords 拼写不同）。每个案例/压力有配置和状态；分 q 提交调用 qeflow 依赖链。

## 使用边界

当前生成的部分子命令参数与被调用 CLI 不匹配：vasp_scf/vasp_eledos 使用 -i，而当前入口要求 -p；DOS 后处理使用当前不支持的 --elements-total；nosplit 路径还传入其不支持的 -i。完整自动链不能据此宣称可直接跑通，需修复接口并验收。本次仅记录代码事实，不修改实现。

## 实现依据与关联

[CLI 定义](../../../substitutionflow/src/bin/substitution.rs)；[工作流实现](../../../crates/df-substitution/src/workflow.rs)；[状态与模式映射](../../../crates/df-substitution/src/status.rs)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
