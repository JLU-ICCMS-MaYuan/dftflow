# qe_merge：合并分 q 计算结果

[返回 qeflow 索引](README.md)

## 功能与入口

检查分 q 输出与映射，把动力学矩阵及 PH 中间文件归并到工作目录。

等价入口：`qeflow qe-phonon-merge（也支持 qeflow qe-merge）`。

## 使用方法

```bash
qe_merge -c input.toml
```

`-c` 默认 input.toml；`-i/--input`（别名 -p/--poscar）可指定结构。命令直接合并，没有 -r/-s。

## 输入、输出与衔接

默认 `[qe_phonon.scf].outdir` 为 `./tmp`，SCF 与 PH 初始化/分 q/合并共用该设置；显式设置其他目录会覆盖默认值。已有运行目录若显式使用 `./out`，继续按原配置读取，不自动搬迁或改写历史输入。

读取数字目录与 q_index_map.json；新映射（含 q_cart）按实际 outdir 合并，旧映射缺少 q_cart 时仍走旧合并分支。coords 文件按映射归一为 prefix.dyn<i>，处理对应 q 目录及 patterns。完整链路 post 阶段已调用同一合并模块。

## 使用边界

work_dir 未设置时为当前目录；此独立入口的相对路径仍依赖启动目录，建议指定绝对 work_dir。须全部 q 正常结束且矩阵与映射对应；不凭文件名存在就接受错误 q。不会运行 q2r 或绘图；新链路支持复用 SCF 标记，旧逐 q SCF 目录亦有兼容判断。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[合并逻辑](../../../crates/qf-phonon/src/merge.rs)；[状态判断](../../../crates/qf-state/src/lib.rs)。默认目录的回归与安装验证见 [0006 US4 验收](../../specs/0006-qe-phonon-submit-chain/acceptance.md)。
