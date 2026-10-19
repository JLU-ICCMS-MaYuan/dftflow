# 第二阶段自动比较代码验收

本轮对应 0012 T056、0014 T053 和 0008 FR-016／SC-010 的代码与进程契约部分。真实 PBE 科学链、两模式 Tc 和暂缓的严格投影不据此验收完成。

## 已实现内容

| 环节 | 执行与证据 |
| --- | --- |
| 固定采样 | 共享 KPATH 解析、非正交倒格矢距离、连续／断开段；DOS 复用 F2 完整均匀 NSCF 网格，保留点序与权重 |
| QE 电子路径 | 使用 SCF 独立副本、原带数和物理参数执行路径 NSCF；共同 Fermi 仍来自 F2，不用新路径 Fermi 再对齐 |
| QE 声子曲线／网格 | 保留原始 PH，派生全部 q 星的 Γ onsite 修正和 QE Hermitian 平均；q2r／matdyn 额外 ASR 关闭，原子质量一致 |
| EPW 比较 | 隔离重读同一粗网格，生成同路径加 DOS 网格的电子／声子数据；不重新 Wannier 化，不执行细网格 EPC |
| 比较交付 | 复用五类表、自包含结果包和四张独立图加总览的 PNG／SVG／PDF；按实际原子顺序检查 3N，缺投影明确标记 |
| 阶段与恢复 | 图表和质量门均完成才发布粗阶段清单；质量失败可出诊断但阻断下游；原生成功任务重验复用，失败任务显式归档重试 |
| 绘图发布 | 部分格式重绘保留同源旧格式归属；复用发布日志，检查拒绝半写，显式绘图恢复已登记内容；未知／篡改文件拒绝 |

## 进程验证

证据位于忽略的 `test_auto/MgB2/local_run/007-auto-comparison/`，生产代码仍为 Rust，Python 仅用于独立进程替身。

三项自动阶段回归通过，命令为：

```bash
DFTFLOW_NATIVE_COMPARISON_TEST_ROOT="$PWD/test_auto/MgB2/local_run/007-auto-comparison/cases-targeted" \
  env -u TMPDIR bash scripts/build-native.sh test -p epwflow \
  --test native_interpolation automatic_comparison_ --locked --offline \
  -- --test-threads=1 --nocapture
```

分别验证成功交付与移动重绘、质量失败仅诊断、失败任务恢复与成功证据篡改拒绝。成功路径还模拟仅有 SVG 的交付，补齐三格式时原生调用次数不变；第二阶段不执行细网格。日志为 `automatic-final.log`。

单测、比较图与选窗／配置共 24 项定向回归通过，日志为 `plots-and-config-final.log`：10 项库测试、8 项比较测试、6 项选窗测试。新增矩阵测试逐项核对复数 Fortran 列主序、两 q 星、Γ 行和、QE 上下三角平均、原始文件不变及不完整星拒绝；比较测试模拟首图已替换而清单尚未发布的中断并恢复。已实际查看自动生成的总览 PNG；曲线来自 Si 进程样本，不能作 MgB₂ 科学结果。

扩大回归命令如下。三项自动阶段已单独验证，因此此命令只过滤它们，过滤不记为通过；最后统计需与前述三项合并并去除重复定向统计：

```bash
env -u TMPDIR bash scripts/build-native.sh test \
  -p epwflow -p qeflow -p qf-wannier --locked --offline \
  -- --test-threads=1 --skip automatic_comparison_
```

扩大回归日志为 `regression-final.log`，进程退出 0：181 项通过、0 失败、6 项真实环境暂挂，另过滤前述三项自动阶段测试。合并独立通过的三项后，共 **184 项通过、0 失败、6 项暂挂**；24 项定向回归不重复累加。分组统计及日志哈希见本地 `regression-summary.json`。

新自动比较与 ASR 模块通过 `rustfmt --check`；变更 Markdown 的本地链接和 Bash 代码块、两份 Python 进程替身语法及 `git diff --check` 均通过。对应记录为 `documentation-validation.json` 和 `rustfmt-check.log`。

## 发现并修复的问题

1. 初始 matdyn 替身省略第四列 q 权重，生产严格解析器正确拒绝。按实际 QE 输出修正替身；不放宽解析器。QE 显式点输入仍为三坐标，输出为三坐标加 q 权重。
2. XML q2r 在力常数基名后追加 `.xml`；生产检查实际 `reference.fc.xml`，matdyn 沿用基名读取，替身同步该约定。
3. 原比较图只重绘 SVG 时清单丢弃旧 PNG／PDF 的归属，随后全格式重绘误报未知冲突。修复为保留同源、已登记且哈希匹配的格式，并接入共享发布日志。该缺陷已由恢复测试复现并修复。

初始失败／中断日志保留，不计为通过。原始 PH／SCF／NSCF 只读，未用重复写入或删除失败点绕过检查。

## PBE 文件与脚本生成

复用现有 PBE 结构、赝势、显式质量和前轮已选的四条深能带排除，使用 `energy_only` 与显式比较配置。只生成三份阶段脚本和 `submit-all.sh`，四份脚本通过 `bash -n`，没有创建科学尝试目录、执行 QE／EPW 或提交 Slurm。配置、脚本哈希与记录见 `inputepw-generation.toml`、`pbe-generation.log`、`pbe-generation-validation.json`。

未切换默认安装、升级外部程序、追加数值扫描或重置原共享预算。进程测试、脚本生成、真实质量和 Tc 仍分别验收；LDA 历史参考不能用于 PBE。

## 文档影响与剩余边界

0012 的 Issue／Spec／设计／数据／契约／任务、0014 的绘图契约与发布设计、0008 三程序说明、模块 README 流程图和 Overview 同步当前已实现行为。0012 T056、0014 T053 为本批代码子任务；0008 T041／T042、真实 PBE 科学链、两模式完整 Tc 及原子投影均不因此关闭。线上 Issue 仍由用户管理。
