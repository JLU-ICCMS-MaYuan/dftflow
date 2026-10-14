# qe_ph_nosplit：单步 PH 与仅生成真实 dyn0

[返回 qeflow 索引](README.md)

## 功能与入口

普通模式准备/运行一次 ph.x；增加 --dyn0 时改为 QE 的 only_init 初始化，正常结束后验证不可约 q 点列表。

等价入口：`qeflow qe-ph-nosplit`。

## 使用方法

```bash
qe_ph_nosplit -c input.toml --dyn0
qe_ph_nosplit -c input.toml --dyn0 -s
# 已获计算资源时，也可改用 -r
# 普通单步 PH（不加 --dyn0）
qe_ph_nosplit -c input.toml -r
```

`-c` 默认 input.toml；`-i/--input` 可选结构；无执行标记只准备；-r 与 -s 二选一。以上为不同用法，通常只选需要的一条。

## 输入、输出与衔接

默认 `[qe_phonon.scf].outdir` 为 `./tmp`，SCF 与 PH 初始化/分 q/合并共用该设置；显式设置其他目录会覆盖默认值。已有运行目录若显式使用 `./out`，继续按原配置读取，不自动搬迁或改写历史输入。

--dyn0 需要既有 SCF 的 XML、电荷密度与波函数。生成 `ph_init.in`、`run_ph_init.sh`，执行后生成 `ph_init.out`、`prefix.dyn0` 和 `ph_init_complete.json`。默认 work_dir 为 qe_phonon，--dyn0 的相对 work_dir 按配置位置解析，outdir 按该工作目录解析。

具体文件用途与底层重定向见 [dyn0 初始化文件问答](phonon-workflow.md#dyn0-初始化文件问答)：`ph_init.in` 是输入，`ph_init.out` 是日志，`prefix.dyn0` 是生成的 q 点列表；分 q 响应另用数字目录中的 `split_ph.in/out`。

## 使用边界

--dyn0 不补跑 SCF，不计算完整声子，不自动安排分 q 或后处理。设置 ldisp/only_init/lqdir=true，清除 q/表示范围限制并关闭 only_wfc/recover；等待正常退出，不监测到 dyn0 就杀进程。重初始化前归档旧 dyn0，成功标记仅代表初始化完成。普通 PH 与 --dyn0 是两个用途；自动 split 已包含初始化，无需手动预跑。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[独立初始化实现](../../../qeflow/src/phonon_init.rs)；[初始化覆盖](../../../qeflow/tests/phonon_init.rs)。默认目录的回归与安装验证见 [0006 US4 验收](../../specs/0006-qe-phonon-submit-chain/acceptance.md)。

独立算法工具另见 [qf-gen-dyn0](../tools/qf-gen-dyn0.md)，它不调用 QE。
