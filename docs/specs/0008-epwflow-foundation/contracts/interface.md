# CLI 与数据契约：配置、环境检查与受控执行基础

**版本**：1；**需求**：[Spec](../spec.md)；**模型**：[数据模型](../data-model.md)。

## 公开接口

完整命令、参数、默认值、单位与行为以 Spec 的 FR-001 为规范；本接口不另改命名。所有 --help 均无科学执行副作用。参数使用 argv，显式路径按所属配置或命令规则解析，不 eval。

配置路径须显式传入；本机 MgB₂ PBE 示例使用 `-c inputepw.toml`，保留同目录已有 qeflow `input.toml`。加载器接受任意文件名，仍执行 epwflow 严格字段校验；不按文件名切换配置格式，不自动合并 qeflow 字段。`delivery budget` 继续使用既有 `--config` 长选项；运行绑定快照仍为 `input-init.toml`。

共同执行/错误/持久协议由 [0008 基础契约](../../0008-epwflow-foundation/contracts/interface.md) 唯一拥有；科学字段由各自 Spec 定义，全部进入同一 TOML schema_version=1。check/status 的 --json 返回 schema_version、operation_status、workflow_status、diagnostics；status 查询成功不代表科学完成。

## 输入输出与错误

读取经过绑定的 input-init.toml 及前置 manifest；每次读取确认版本和指纹。缺字段/类型错误/单位不符返回非零并报告字段路径；状态未知、预算拒绝和数据无效用独立错误类别，不自动重试。可预期的未定位或部分结果以结构化状态表示，不填伪造数值。

F1 返回配置/运行绑定与阶段协议，F2 返回 qe-data/manifest.json，F3 返回 interpolation/manifest.json，F4 返回 superconductivity/manifest.json 与 tc.json，F5 返回 results/manifest.json。每个 manifest 列出完整角色清单、尺寸、哈希、生产阶段、上游摘要与验证状态。文件记录采用相对 artifact_root 的路径；绝对机器路径只存在实际运行的忽略目录。拒绝路径穿越和指向来源的可写链接。

## 本项数据字段

- `RunBinding`：schema_version,run_id,config_sha256,structure,pseudos,programs,environment,work_dir,outdir_base,budget_id；prepare 唯一建立；input-init.toml 不可修改；新输入新绑定。
- `StageAttempt`：stage_id,attempt_id,dependencies,input_digest,state,job_id,cluster,validator,artifacts,error；ready→submission_pending→submitted→running→validating→succeeded；失败/unknown/blocked 不放行。
- `BudgetLedger`：schema_version,campaign_id,limits,spent_node_seconds,reservations,allocations,lanes,storage_roots,revision；同一 allocation 仅结算一次；未知保留预留；不提供 reset。
- `EnvironmentEvidence`：executable_sha256,launcher,env_script_sha256,host,observed_version,time,status；版本证据与文件身份绑定，变化即失效。

## 权限、兼容与副作用

仅使用当前用户已有权限。不得自动修改系统库、清除预算、覆盖来源、取消无关作业或推送仓库。默认 outdir 是按配置目录解析的基址；实际保存树为 <outdir>/<run_id>/<stage_id>。同一绑定内路径一经准备不变，用户自定义优先，工作目录与保存目录分别显示。

文件接口升级须新版本并提供显式兼容检查，不能静默读取未知格式。失败输出可保留用于诊断，但不能充作下游成功数据。查询和错误输出不得泄露无关配置或认证内容。

## 统一 TOML 字段和校验（版本 1）

这是完整配置的唯一字段注册表。每层都拒绝未列字段；相对输入路径按原配置目录解析。下游模块负责各自物理校验，基础层负责解析、类型、有限性、路径和资源。共同字段默认不来自机器猜测。

| 分组 | 字段与类型 | 约束/默认 |
| --- | --- | --- |
| 根 | schema_version:u32, work_dir:path, outdir:path, structure:path, pseudos:map(element,path) | version=1；outdir 缺省 ./tmp；其余必填 |
| programs | pw/ph/epw:Program, launcher:argv[], environment_script?:path | Program={path, args:argv[]}；路径/参数不 split；没有脚本时不加载隐式环境 |
| resources | partition:string, nodes:u32, mpi:u32, wall_seconds:u64, storage_bytes:u64 | 1<=nodes<=1，1<=mpi<=64，0<wall<=7200；storage>0；阶段可减小不能超限 |
| budget | campaign_id:string, ledger:path | 本次 campaign_id 固定 epic-0007；所有验收运行引用同一实际账本，limits 不从任意配置放宽 |
| qe | prefix:string, ecutwfc_ry:f64, ecutrho_ry:f64, occupations:enum, smearing?:enum, degauss_ry?:f64, scf:Scf, nscf:Nscf | prefix 禁止路径分隔/控制字符；截断正数，rho>=wfc；smearing=gaussian/mp/mv/fd；occupations=fixed/smearing；smearing 模式必须正 degauss |
| qe.scf | k_grid:[u32;3], k_shift:[u32;3], conv_thr_ry:f64 | 网格>0，shift 各项0或1，阈值>0 |
| qe.nscf | k_grid:[u32;3], nbnd:u32, conv_thr_ry:f64 | 网格含Γ；band>0；实际数量后验核对 |
| phonon | mode:enum, q_grid:[u32;3], tr2_ph:f64 | 默认 index；mode=nosplit/index/coords；网格/阈值正 |
| wannier | num_wann:u32, projections:string[], exclude_bands:u32[], energy_reference:enum, outer_window_ev?:[f64;2], frozen_window_ev?:[f64;2], localization:Convergence, disentanglement?:Convergence | 能量参考 qe_zero/f2_fermi；exclude 1起始不重复；窗口递增、冻结包含于外窗 |
| Convergence | max_iterations:u32, tolerance_angstrom2:f64, window_iterations:u32 | 均正；window<=max；适用的展宽泛函阈值单位 Å² |
| interpolation | k_grid:[u32;3], q_grid:[u32;3], electron_smearing:enum, degaussw_ev:f64, phonon_smearings_mev:f64[], spectral_points:u32, fsthick_ev:f64, eps_acoustic_cm1:f64, fermi_source:enum, quality:Quality | 网格正且k逐方向可整除q；smearings正、不重复；spectral_points>=2；fermi_source=f2/fine_fixed_electrons |
| Quality | k_points_crystal:[[f64;3]], q_points_crystal:[[f64;3]], energy_window_ev:[f64;2], electron_max_ev:f64, electron_rms_ev:f64, phonon_max_cm1:f64, phonon_rms_cm1:f64, reference_manifest?:path | 参考缺省自动安排独立参考任务；阈值正且RMS<=max；验收采用固定 Spec 阈值 |
| superconductivity | mode:enum, mu_star:f64, frequency_cutoff_ev:f64, max_iterations:u32, convergence_relative:f64, mixing_beta:f64, mixing_history:u32, initial_gaps_mev:[f64;2], initial_shape:f64, gap_temperatures_k:f64[], spectrum_index:u32, iso:Tc, aniso:Tc | mode=iso/aniso/both；μ*>=0；cutoff>0；mixing_beta在(0,1]；initial_gaps正且不同；温度有限、0.01K刻度；谱索引0起始且存在 |
| Tc | min_k:f64,max_k:f64,precision_k:f64,max_solves:u32,eigen_margin:f64,normal_gap_mev:f64,super_gap_mev:f64 | 0<min<max，precision>=0.01K；max_solves>0，eigen_margin>输出舍入；0<normal<super |

只选一种求解时另一模式 Tc 可省略；所选项必须提供。需要解纠缠时相应 Convergence 必须存在；首版禁止通过未知磁性/SOC 字段透传。JSON 记录统一使用 schema_version=1。

## 预算初始化、登记和执行协议

首次准备为显式配置的 campaign_id/ledger 建立授权上限，首先检查本机 campaign 注册表；注册表仅是 ledger 的固定位置索引，不是另一份用量账本。位置为用户级 epwflow 状态根下 campaigns.json，创建/更新同样加锁和原子发布。已登记 campaign 改路径、同路径改 campaign、删除已登记账本均拒绝；状态根路径与账本写入证据，不能通过换工作目录重置。复制或故意删除用户级登记属于人工破坏，不宣称能提供操作系统级不可绕过配额。

budget status --ledger L [--json] 只读；reserve --ledger L --label LABEL --nodes N --mpi P --wall-seconds S --storage-bytes B 创建外部参考预留并返回 token/必须使用的 Slurm 依赖；attach --ledger L --token A --job JOBID 校验实际分配和依赖；reconcile --ledger L 查询终态并结算。没有 reset/refund/force-release 未知作业功能。

生产 submit 使用同一 reserve 内部函数，用户无需手动为每阶段登记。run/probe -r 必须已有 allocation 记录与当前 SLURM_JOB_ID 一致，否则非零退出；不会在登录节点悄悄运行。预算命令不得执行任意用户字符串。外部参考不遵守返回约束时标为 protocol_violation，并阻止自动追加。

reserve 返回的是请求，不是计算成功；分配耗时以 Slurm allocation 终态为准。监测超过存储限额请求终止本流程写入；因未知调度或采样延迟导致超限如实记录，不自称配额保护成功。

## 持久错误与恢复

错误类别：configuration、input_conflict、environment、unsupported_version、scheduler_unknown、budget_denied、storage_denied、data_invalid、execution_failed、not_implemented。stderr 写类别/阶段/原因；JSON status 读成功可 exit 0，但明确 workflow_status。同步执行科学失败返回非零。

缺失/损坏 ledger 不得重新初始化已登记 campaign。锁目录含 host/pid/start_identity 与事务 token；同主机确认 owner 已死且没有关联未决写操作后，resume 可在独占恢复锁下原子认领；跨主机或未知 owner 给出诊断而不自动删除。恢复不能丢弃 intent，必须先核对 Slurm 后决定是否重试。

## 2026-10-19：统一入口当前实现

run/submit 支持 --until qe-data|interpolation|superconductivity|all，默认 all；resume 的可选 --until 缺省继承已有记录。workflow.json 保存 attempt_id、until、state、reservation、job_id、cluster、executable、script、completed、current_stage、error，版本 1。completed 保存清单与依赖文件身份；变更或缺失不跳过。unknown 不自动重提，不通过重启释放预算。

内部 __execute 只接受匹配 token 和 Slurm 作业的已登记分配，不能作为用户绕过预算的公开捷径。当前单份分配顺序调用已有阶段；阶段成功和全链成功分开，底层科学适配未完成时记录具体失败。未来并行依赖图仍按原 Spec 验收，不据本批边界回归关闭。新阶段不再次按资源请求预留或结算同一 allocation。
