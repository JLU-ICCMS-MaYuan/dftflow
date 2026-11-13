# 0068 验收记录

## 交付结论

SCF、NSCF 数据完成核验后，在同一阶段分配内即时绘制能带范围图；electronic 在路径 PW 完成核验后绘制，不等待 bands.x、DOS／PDOS 或后续阶段。共用诊断组件只读取 XML 与配置，保存 300 DPI PNG、SVG、CSV 和 JSON，发布到各阶段 results 的 band-ranges 单元。正式四个 EPW 入口已覆盖，未新增科学计算或提交作业。

本地 Issue 正文／Spec／Plan／Research／数据模型／契约／Tasks／Quickstart 已建立。线上仍由用户管理；当前 origin 是不匹配的 wannier90 上游，未向其写入 Issue、未推送，不宣称远程链接或关闭完成。参考图原文件未取得，仍为对话预览。

## FR／SC 与证据

| 要求 | 实现和可复查验证 |
| --- | --- |
| FR-001／SC-001 | native_workspace::refresh_stage 与 workspace_science::refresh_post 接入前两步；workspace_electronic::run_leaf 在路径 PW 后、bands.x 前绘图。公开提交与阶段分配测试验证每步结束已有图，SCF 时无 NSCF，NSCF 时无 electronic；另测试 DOS 非零失败前 electronic 图已生成 |
| FR-002～005／SC-002 | band_ranges::summarize／render 保存原始带号、绝对极值、采样／费米来源、四窗和排除后的计数。独立脚本直接从三份 XML 复算每条带 min/max、跨费米标记和逐点冻结／外窗计数，与 JSON 一致；有限深能级范围条最后绘制，不被窗口线遮挡 |
| FR-006／SC-003 | qf-wannier::energy_only_bounds 与正式选择共用边界转换，不做数量放行；超限真实旧配置留下图，进程及单元回归同时证明正式 window_selection 仍拒绝并保留“不自动裁剪” |
| FR-007／SC-004 | 三张真实图均 4400×2360、300.0248 DPI，最终 PNG 逐张实看；重合窗口交错绘制、图例和底注完整，CSV／JSON 可复查，SVG 可核对 |
| FR-008／SC-003 | 删除 NSCF 图后只补图，无 PW／PH／EPW／后处理重算；移动结果包后在空 PATH 下补图通过。真实调用的 execve 跟踪无科学程序、sbatch 或 mpirun；11 份 XML／原始输入 SHA-256 未改变，不读取或核对密度／波函数 |

## 回归与构建

完整 release 回归：EPW 与 qf-wannier 去重合计 **380 通过，0 失败，15 项既有 ignored**。ignored 不计通过。最后仅修改深能级范围条绘制顺序后，额外复测 1 项统计单元测试与 2 项公开进程回归，全部通过，不将重复测试累计成新总数。

初次新进程测试使用旧辅助函数将 scf 发给 epw_wannier，发生 --stage 不属于当前程序；修正测试辅助函数的程序选择后两项通过，随后完整回归通过。这是测试辅助函数缺陷，不把失败轮次算成通过。

最后执行 `bash scripts/build-native.sh build --release -p epwflow --locked -j 6`，自动覆盖固定 epw_phonon、epw_wannier、epw_superconductivity、epw_clear，与 release 制品逐字节一致。未创建额外编译入口、旧命令备份或编译版本记录。依赖中既有 unused／未来兼容警告保持原样。

## 真实 MgB₂ 诊断

初次未调窗配置下，三个采样的能量冻结态均为 8～9，num_wann=5，图中正确标记超限。随后另一个窗口独立调整输入四窗与排除带，并在 `46c5c2a4` 提交；本功能提交不包含其输入和 Ubuntu 手册改动。按最新输入再次使用已安装的 epw_wannier 不带 -s 补图，退出 0、到达 prepared，没有提交 Wannier 计算。

最终图的真实统计如下；采样不同，计数不能互相代替：

| 阶段 | k 点 | 原始带数 | 排除后外窗态／k | 能量冻结态／k | num_wann |
| --- | ---: | ---: | --- | --- | ---: |
| SCF | 8 | 12 | 8～8 | 4～5 | 5 |
| NSCF | 32 | 16 | 12～12 | 4～5 | 5 |
| electronic 路径 | 183 | 16 | 11～12 | 3～5 | 5 |

SCF 使用自身 E_F；其余电子图依照 NSCF 零点约 9.183946302 eV。图中的四界按当前显式配置显示，最终 Wannier 仍以 NSCF 为准。本验收只确认绘图、统计、时机和复用，不等同于拟合收敛、EPC 或超导验收。

实际文件：

- examples/MgB2/ubuntu/run/1.qe-scf/plots/scf-band-ranges.png
- examples/MgB2/ubuntu/run/2.qe-nscf/plots/nscf-band-ranges.png
- examples/MgB2/ubuntu/run/3.qe-electronic/qe_eband/plots/electronic-band-ranges.png

所有运行数据仍被忽略。证据位于忽略目录 docs/local/0068-stage-band-ranges：regression.log、final-band-ranges.log、build.log、installed-cli.stdout／stderr、installed-execve.log、sources-before.json、independent-numerical-check.json 和 final-verification.json。旧配置的计数与新配置的最终证据分开保留。

## 工程约束与文档

KISS／DRY：一个报告／渲染模块，三个调用点共用，纯能量边界与生产选窗共用解析。单一职责：图是派生诊断，不拥有科学阶段完成或选窗修改权。未新增依赖、CLI 参数、守护进程或科学任务；Slurm header、12 MPI 约定和密度／波函数不核对边界保持。Overview、根和模块 README 已同步；Ubuntu 手册由并行任务维护，不混入此次提交。
