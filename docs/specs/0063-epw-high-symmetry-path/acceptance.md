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
