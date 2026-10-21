# 0025 维护验收

## 范围与来源

本轮由用户“开始”授权维护实施，沿用本地 Issue 约定。源码基线为 be00725，加启动时已存在的 0023 等未提交修改；原有 10 个已修改文件及未跟踪的 0023 目录单独保留，本次仅提交 0025 增量。期间另一会话提交了 3a01c5b（0026 文档），与本项范围分开。测试针对这一共享工作树，不把此前未提交实现冒充本项提交。

## 实现与依据

- 阶段回执为本阶段 completion.json，电子原生回执及后处理回执归本阶段。共享状态、提交快照、交付与历史位于 epw-state。
- 阶段复用只将 NotFound 判为需要重建；内容/链接冲突、损坏和不可读继续报错。重建上游前归档过期下游回执，防止同字节重算使旧交付复活。
- 显式 --migrate-state 借用旧控制锁并核对终态；旧目录及账本不改写，历史复制前后哈希一致。正常读取不回退到隐藏状态；未完成迁移阻止普通操作并允许接续。
- 默认预算注册表可见化，并承接旧默认 campaign 绑定；显式环境覆盖保留。

## 验证命令与分层结果

最终回归使用规范本机入口：

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p epwflow --no-fail-fast
```

去掉测试调试符号只减小被反复哈希的可执行文件，不改变生产判定。初次直接 cargo 缺少 fontconfig.pc，改用既有 scripts/build-native.sh 及本机依赖配置；未安装或升级系统依赖。先前带完整调试符号的进程回归发现模拟 sbatch 重复返回旧作业号，生产预算保护正确拒绝；修正替身后执行上述最终回归，不将该初次失败计为通过。

初次全量为 179 通过、4 失败、7 暂挂。失败定位到两个问题：迁移未识别电子子任务嵌套的 attempt；电子任务正常更换 DOS 展宽时先校验旧输入，把合法输入更新误报为篡改。已分别修正封装读取、科学键与身份检查顺序。

随后编号回归为 16 通过、1 失败、1 暂挂，覆盖上述四项初始失败中的三项及全部新增恢复/迁移用例。最后一项失败由整理证据脚本误写本地测试构建文件引起：变量复用将 JSON 写入 epw_superconductivity 构建产物，未改源码或默认安装。删除受损本地构建产物后重新链接，并单独复测完整公开链路；该完整链路最终 1 项通过、0 失败。按用例去重合并全量与两次针对性复测，EPW 共 183 项通过、7 项暂挂；没有将历史失败或重复通过累计为额外通过。

针对性命令如下，原始失败日志与成功复测日志均保留：

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p epwflow --test three_stage_execution numbered_ --no-fail-fast
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p epwflow --test three_stage_execution numbered_public_qe_wannier_coarse_and_fine_are_distinct_and_reusable --no-fail-fast -- --exact
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p qeflow
```

共享 qeflow 回归为 33 通过、0 失败、0 暂挂。7 项 EPW 暂挂需要显式真实科学作业或本机旧数据条件，未计为通过。

完整日志、复测日志、源码及程序 SHA-256 留在忽略的 `docs/local/0025-epw-visible-run-state/`；测试程序为本地 debug 产物，未切换默认安装。

## 需求与证据映射

| 标准 | 核查依据 |
| --- | --- |
| SC-001 | 三公开入口原有协议及新增目录检查，无隐藏运行目录；普通和编号路径统一使用可见状态 |
| SC-002 | numbered_deleted_wannier_directory_rebuilds_without_repeating_upstream；删除前下游真实依赖判为可复用，删除后撤下并在重算后仍不可复活 |
| SC-003 | 缺失 chk、内容篡改、损坏回执、排队/未知任务、冻结配置和锁回归；预算默认位置迁移拒绝另建账本 |
| SC-004 | 原隐藏树及账本逐字节保持，迁移中断接续、锁和未知状态阻断，旧目录移走后仍可查询/生成，重复迁移幂等 |

## 文档与工程约束

- KISS/DRY：复用现有阶段、身份校验、路径映射和锁；迁移是有界入口，不另建服务、通用状态框架或执行器。
- 完成回执归属对应阶段目录，跨阶段提交/配置快照继续独立保存；没有取消完整性、排队隔离、预算或科学质量检查。
- 已回写模块与 MgB₂ 操作说明、Overview，80 个相对链接检查通过；原 0018 内部状态路径约定由本项替代，原科学 DAG 与数据所有权不变。
- 本项没有切换默认安装、迁移真实 MgB₂ 现场、提交科学作业或推送；当前终端安装程序不因源码提交自动改变。
- 科学网格、质量门、共享预算与此前科学验收限制均不变。隔离协议和模拟输出不能代替真实科学验收。
- 线上 Issue 状态与远程双向链接仍由用户后续上线核验，不代为关闭。
