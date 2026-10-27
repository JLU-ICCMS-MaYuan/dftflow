# 0063 实施与验收记录

日期：2026-10-27。用户已明确开始执行，并确认只在相同坐标计算能量差、保留 DOS、不提示共同点覆盖率。

## 已实现

- 第 5 步从现有高对称端点文件生成真实路径边，MgB₂ 9 条连线对应 9 条路径声明；保留标签和断段，移除人工校准、逐点及网格覆盖段。
- 保留用户 bands_num_points 设置，取消旧的固定为 1 的约束；检查空路径、奇数端点、非有限坐标、非法标签、零长度边、行长及原生短段零间隔。
- 从原生 kpt、band、labelinfo 核验实际点数、端点与逐段坐标，以真实倒空间距离保存各自完整曲线。差值只计算双方原生共同坐标，没有共同点不生成最大差，不显示覆盖率。
- DOS 沿用同一拟合模型的 postw90 均匀网格，不把网格写入路径。第 6 步使用带哈希的第 5 步电子表格、统计和采样，声子保持原协议。
- 旧包缺少新标记时保持原有同点位验证；旧拟合、输入和回执不自动改写。Overview 与 MgB₂ 测试说明同步。

## 隔离验证

在基线 `7dfedb60ed338c8f223305ddfa9f81ed5b511f9c` 的临时归档中只应用本任务改动，目录为 `/tmp/dftflow-0063-isolated`。构建沿用本机原有原生依赖环境，未发布安装。

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow --lib --test comparison_plots --test native_stage_config
```

结果：库测试 51 通过、3 跳过；绘图测试 10 通过；配置测试 13 通过。日志：`/tmp/dftflow-0063-isolated-core-retry.log`。

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow --test three_stage_execution stage_results:: -- --test-threads=8 --nocapture
```

结果：7 通过。覆盖第 5 步交付、不同原生点数及断段、第 6 步电子 CSV 原样复用、DOS 单次调用、结果包检查、旧协议迁移、损坏坐标拒绝及后处理恢复。日志：`/tmp/dftflow-0063-isolated-stage.log`。

另用同一已编译测试程序执行 `decay::decay_is_published_before_comparison_failure_and_repairs_without_rerunning_science`，1 通过。日志：`/tmp/dftflow-0063-isolated-recovery.log`。合计 82 通过；3 个依赖显式真实数据的库测试未执行，不计为通过。

混合工作树的较大进程套件曾出现阶段恢复失败，未把该次未完成运行记为通过；同名恢复用例在本任务隔离树通过。隔离首轮构建因未带入本机 libpng 的 pkg-config 路径失败，带入原有本机构建环境后重新验证通过，没有为此修改依赖或系统配置。

## 需求证据

| 标准 | 证据 |
| --- | --- |
| SC-001 | MgB₂ 九段输入单测逐行检查真实端点及数量；真实生成入口的三段受控用例无 P0／Q0 |
| SC-002 | 两种密度路径块相同；重复 Γ、断段、非正交倒格矢及不等长路径测试 |
| SC-003 | 共同坐标统计和无共同点单测；实际原生点序／坐标错误拒绝；公开入口保留不同点数 |
| SC-004 | 阶段测试核对能带、DOS、postw90 单次调用和第 6 步 CSV 哈希复用 |
| SC-005 | 非法输入拒绝；旧来源、成功科学回执和模型哈希保护；损坏原生坐标拒绝 |
| SC-006 | 上述隔离回归及本目录、Overview、MgB₂ 说明的链接和原话一致性检查 |

## 文档影响与边界

根 README 的公开入口与导航没有变化，无需修改。采用原有 Rust/Bash 生产链及 Python 进程协议替身，不新增依赖或第二套求值器；路径声明、原生采样与比较发布分别保留既有职责。

本轮没有执行真实 QE／EPW／Wannier／postw90、提交 Slurm、修改用户科学现场、切换安装、推送或操作线上 Issue。测试替身不证明真实拟合质量；图像和数值仍供用户人工判断。线上 Gitee 及远程链接仍由用户管理；图片原件仅聊天预览，PDF 核查记录保留该限制。

## 自定义文件头回归修复（2026-10-27）

### 原因与原样复现

用户恢复 SiP₂ 时在第 1 行报错。真实文件头是 `SiP2 Pbam; historical Wannier path`、`20`、`Line-Mode`、`Reciprocal`。初次测试只覆盖固定 VASPKIT 标题和裸端点；新严格解析器限定标题前缀，漏掉了现有自定义标题。这是本项实现及测试遗漏，不是 Wannier 投影或局域化收敛失败。

将真实 SiP₂ 路径逐字节复制为受控 fixture `epwflow/tests/fixtures/sip2-kpath.txt`，先在旧解析器运行新增回归，明确复现 `KPATH.in:1 应为 kx ky kz label`，退出 101，1 失败。证据：`docs/local/0063-header-debug/red.log`。

### 修复与受控验证

完整识别四行文件头后允许任意标题，不再逐条宽泛跳过正文中的数字或关键字；保留端点严格校验和原始物理行号。生产变更仅在 `high_symmetry_edges`，不改用户 KPATH、共享 QE／声子解析器、科学参数、哈希规则或回执。

从 `0cfc1b126520bfc87d423052c0c140a1af89a6a4` 归档创建 `/tmp/dftflow-0063-header-debug-20261027`，只叠加本任务源码、测试和 fixture；排除无关脏改动 `native_config.rs`。本机依赖环境沿用既有配置，不修改系统依赖。源码身份见 `docs/local/0063-header-debug/source-manifest.json`。

- 库测试 56 通过、3 个显式真实数据测试默认跳过；配置测试 14 通过；比较绘图 10 通过。完整进程套件包含无关超导批处理，主动停止并保留记录，不把未完成套件记为通过或回归失败。证据：`green.log`、`full-suite-interrupted.json`。
- 第 5、6 步相关交付、坏坐标拒绝、历史读取、局部恢复及不同原生采样测试 7 通过，见 `stage-results.log`。
- SiP₂ 完整 12 段路径的公开 CLI 测试 1 通过：在带空格的独立目录先完成替身拟合，再改为 500 次显式恢复；仅拟合重跑，SCF／NSCF／PH／电子参考调用不增加，路径与电子回执哈希不变，阶段交付完整。见 `sip2-cli-resume.log`。
- 显式使用当前 `examples/SiP2/epwinput.toml`，从 SiP₂ 目录执行既有只读真实数据测试 1 通过：244 个 QE 路径点、36 个 NSCF 网格点，`wdata` 共 17 行（12 条真实边及块设置），完整原生输入生成成功。没有调用科学程序或调度器。见 `sip2-readonly.log`。

以上合计 89 个不同测试通过；默认跳过的 3 项中，真实配置只读测试已单独执行，另 2 项不计通过。FR-011／SC-007 由新增三项解析／生成测试、公开 CLI 恢复和真实配置只读输入生成共同覆盖。

### 编译与安装

隔离源码执行 `bash scripts/build-native.sh build --release -p epwflow --bins --locked` 成功，4 个 CLI 的已打包制品经现有安装器覆盖正式 `bin/epwflow/`。安装器逐入口检查启动和活动引用；正式文件与已验证隔离制品及发布清单的 SHA-256 全部一致，证据：`release.log`、`install.log`、`published.json`。

加载 `scripts/env.sh` 后，实际入口为 `/data/home/mayuan/code/dftflow/bin/epwflow/epw_wannier`，发布 SHA-256 为 `0b23530eb35693c4c71363d6b4096e5d716389bea6159be791ca3ca1e57d5fab`。

安装后另用正式 CLI 从 `/tmp` 调用带空格的受控 fixture 配置，3 次公开调用完成恢复拟合、原生 DOS 和最终交付。此项仅将替身配置从 500 改为 501 触发新拟合，不修改真实 SiP₂ 配置；保留 SiP₂ 原样路径，前置科学调用及路径、电子回执哈希不变。所有调度和科学程序均为 fixture 替身，不触达真实 Slurm。证据：`installed_cli_replay.py`、`installed-cli-calls.json`、`installed-cli-result.json`；这是正式安装入口验证，不属于真实拟合验收。

### 数据保护与验收边界

真实 SiP₂ 的 KPATH、TOML、结构、已有阶段回执及 SCF／NSCF XML 共 10 份选定输入证据哈希保持不变，受控 fixture 与真实 KPATH 逐字节一致，见 `baseline.json`、`checks.json`。本轮未对全部大型响应文件重做哈希核验，不能把上述检查描述为完整来源重验。

本轮更新正式 CLI，但没有向真实 Slurm 提交计算，没有重算真实 SCF／NSCF／PH 或 Wannier，没有放宽收敛检查。新拟合质量及收敛仍待用户运行当前 500 次配置后判断。未推送，也未操作或关闭线上 Gitee Issue。

## 归档后悬空活动记录续修（2026-10-27）

### 现场复现与上一轮测试缺口

用户再次报 `epw_wannier.in` 缺失。正式 CLI 的 SHA-256 仍为上一节的 `0b23530e…d5fab`，确为已编译安装的文件。以该正式入口在真实 SiP₂ 目录不带 `-s` 调用，原样复现缺失输入错误，退出 1；见 `docs/local/0063-archived-resume-debug/real-old-cli-red.log`。

旧活动任务 `18e2555b1a3cde2c-d84b-26` 已明确失败；其输入、脚本及日志实际在 `epw-state/history/wannier-18e25df120f3ab0b-2c48eb-1d/calculation/`，但活动记录仍引用当前第 5 步。后续一代输入生成又失败，未创建新任务，恢复入口因此先被旧输入核验拦住。上一轮公开恢复测试从成功模型改参数，旧任务为 `validated`，没有覆盖失败任务的这条路径；真实只读输入生成也没有经过任务归集。不能把这些测试描述为真实失败现场的恢复验收。

### 修复与隔离回归

归档失败第 5 步时，完整核验归档身份并原样保留旧活动记录，立即撤下旧指针。对旧版本遗留状态，仅显式恢复、旧任务明确退出失败、另一代进度已失败、当前旧输入确实缺失且归档唯一完整匹配时接续。输入、脚本、版本对应的日志及外部来源都参与核验；不改旧哈希、不删除失败证据、不复制旧输入回来或登记旧模型成功。

以 `110047ce4c0c8e01f288180419800b38c744b546` 为基线，在原隔离目录只叠加本任务 4 个 Rust 文件；与工作树源码逐字节核对，另有 284 个 Rust／Cargo 文件全部与 HEAD 一致，继续排除无关 `native_config.rs` 改动。证据：`baseline.json`、`source-manifest.json`、`isolated-unchanged-source-proof.json`。

- 库测试 58 通过、4 个真实数据测试默认跳过；见 `lib.log`。归档单测覆盖显式恢复、原记录字节保持、幂等，以及输入／脚本／日志／来源损坏、文件缺失、归档重复、进度缺失、同代进度、运行进度、未知任务、退出状态缺失及当前输入冲突 12 类拒绝条件。
- 第 5、6 步进程回归 9 通过，86 项未选入本轮；见 `stage-results.log`。新增完整失败链为“拟合缺少收敛证据 → 改参数归档后短段生成失败 → 修复参数显式恢复”，同时核验新归档交接和测试重建的遗留活动指针；前置科学调用不增加、旧记录原样保留。
- 第一轮真实目录调试测试因大文件哈希耗时较长，在任何恢复修改前中断；活动记录字节不变、输入仍缺失，见 `real-resume-prepare-debug-interrupted.json`。该中断运行不计通过，改用同一源码的优化测试程序完成相同核验。

### 重新编译与正式安装

隔离目录执行 `bash scripts/build-native.sh build --release -p epwflow --bins --locked` 成功；4 个正式入口通过既有安装器覆盖。实际加载 `scripts/env.sh` 后的 `epw_wannier` 路径为 `/data/home/mayuan/code/dftflow/bin/epwflow/epw_wannier`。正式文件、隔离打包文件和发布清单的 SHA-256 一致，新 SHA-256 为 `71428b654e71ac448e42b294947802e2b3e8cdc0a7a3d73b354ce23f8c4196f0`。证据：`release.log`、`install.log`、`published.json`。

### 真实 SiP₂ 恢复准备

从真实 `examples/SiP2` 目录显式启用 `real_wannier_resume_prepare_without_submission`，优化测试程序执行成功，1 通过，耗时 429.44 秒。先以既有回执完整核验 SCF、NSCF、电子参考及约 31 GB 声子交付，再调用生产 `advance(config, Wannier, submit=false, resume=true)`；没有使用调度或科学程序替身。输出明确复用 `scf`、`nscf`、`electronic`、`electronic/projwfc`，最终生成当前目录中的输入和脚本，任务 `18e2629c0eaa5d5a-335462-23` 为 `prepared`，`job=None`、`reservation=None`。见 `release-test-build.log`、`real-resume-prepare.log`、`real-prepared-before-formal.json`。

新输入逐项核对为 `Si:s,p`、`P:s,p`、96 个 Wannier 函数、`num_iter=500` 和原有 `bands_num_points=1`。执行中曾误判密度 1 必然产生零间隔，实际生产路径校验通过后已更正说明；未修改原 TOML 或 KPATH。旧活动记录逐字节保留在原历史目录的 `native-active-retired.json`，另有版本 1 的归档绑定说明；原 10 份选定配置、路径、结构、阶段回执及 XML 哈希不变。

本轮累计 68 个不同测试通过（58 个库测试、9 个进程测试、1 个显式真实准备测试）。默认跳过的其余 3 个真实测试未执行，不计通过；真实准备不等于拟合或收敛验收。

### 正式 CLI 在真实目录复核

加载 `scripts/env.sh`，在真实 `examples/SiP2` 执行正式 `epw_wannier --config "epwinput.toml" --stage wannier`，不带提交选项，退出 0。输出复用四项前置工作并到达 `workflow_status: prepared`；原先的缺失输入错误不再出现。调用前后任务 ID 相同，输入及脚本哈希相同，状态仍为 `prepared`，作业号及预留为空，没有重复创建或提交任务。见 `real-installed-cli-prepare.log`、`real-state-after.json`。

现已可以用普通 `epw_wannier --config "epwinput.toml" --stage wannier -s` 提交该准备任务。已有 SCF／NSCF／PH 不重算；本轮未提交真实计算、未执行新拟合，也不宣称 500 次一定收敛。线上 Gitee 状态及实际拟合判断继续由用户管理。本轮仅提交本任务差异，不推送，保留原有 7 个无关修改文件。
