# 0069 验收记录

## 原因与结论

本机真实运行头为 Program EPW v.6.0；旧 checked_log 只匹配 5.9，多温度入口另有同类硬编码。当前 job 8 实际退出 0、拟合与解纠缠均收敛，随后被版本门误拒绝；输入标题 DFTFlow EPW 5.9 workflow input 不是运行版本。

已统一运行版本识别，接受唯一 5.9／6.0 运行头，继续拒绝未知版本、重复运行头、标题冒充、非零退出、原生错误和缺结束计时。只是运行证据兼容，不宣称所有 6.0 科学格式均已验收。

## 验证

- FR-001～002／SC-001～002：库与 native_outputs、superconductivity、three_programs 回归通过。5.9、6.0 正例及 6.1、重复头、输入标题、非零码、错误和缺计时负例覆盖；多温度使用同一 runtime_version。
- FR-003／SC-003：公开进程回归模拟旧误拒绝，正常 -s 重验且科学调用次数不变；无关失败不自动采纳，原生输入记录缺失也拒绝恢复；该负例已在最终恢复测试中通过。限定 Wannier、特定旧错误、6.0 运行证据、零退出、真实 COMPLETED 和输入存在，随后仍完整验证模型与窗口。
- 分批去重 99 通过、4 既有 ignored，无未解决失败。
- FR-004／SC-004：测试后全工作区 release 构建并覆盖 9 个固定命令，逐文件与 build 制品一致，无版本安装目录或发布版本记录。

## 当前真实现场

通过正式入口、不带 -s／-r／--resume 的准备模式调用当前 Wannier 阶段。正常程序将旧误拒绝恢复到待验证，完成当前模型、四窗、bvec／mmn 等检查；阶段科学状态已完成。旧批次、作业 8 和原拟合保持。

执行前后 epw.out、MgB2.wout、MgB2.chk、MgB2.ukk、MgB2.bvec、MgB2.mmn 六份日志／模型字节身份一致。这些是拟合模型与日志，不是 0067 排除核对的原始电荷密度／波函数。原始 wfc／charge-density 继续按 0067 排除核验；本次未重跑计算。

已准备 5.epw-wannier/comparison-native/electron/postw90.sh，任务状态 prepared；未提交 DOS／新科学作业。用户下一步仍执行正常 epw_wannier -c input.toml --stage wannier -s；后处理结束后再执行以发布图，不使用强制 --resume 重跑拟合。

## 证据与文档

本机回归、构建、准备模式输出和前后模型证明留在忽略目录 docs/local/0069-epw6-runtime-evidence。当前模块说明与 Overview 已同步；Issue／Spec 本地关联有效。线上状态由用户管理，未操作当前不匹配的 origin，未推送。


## 2026-11-18：chem 部署的 EPW 6.1 运行头兼容

用户要求安装最新 DFTFlow，并为 `examples/MgB2/chem` 准备可测试的 `input.toml`。本机已安装 EPW 6.1；原版本门将其直接拒绝。新增断言先复现 `environment_incompatible: 未适配的 EPW 运行版本 6.1`，再将唯一运行头的支持集合扩展为 5.9／6.0／6.1。

原始验收中“6.1 为未知版本负例”是当时的支持边界；本次以 6.2 替代未知版本负例，并增加 6.1 的非零退出、重复运行头、缺结束计时和原生错误拒绝回归。旧 6.0 特定误拒绝恢复条件不变，不扩展历史结果自动恢复。

`native_outputs` 11 项及 `three_stage_execution` 108 项通过，后者 1 项既有本机环境测试跳过。日志留 `docs/local/2026-11-18-chem-install/red.log` 与 `regression.log`。全工作区 release 及独立 dyn0 发布成功，56 个入口在旧 glibc 计算节点启动通过；公开 EPW 阶段入口完成一次真实 MgB₂ SCF，作业 2751176，4 MPI，总能量 −138.11356494 Ry。

本次没有完成 EPW 6.1 的 Wannier／EPC／超导科学验收；矩阵格式、收敛、原生错误和下游检查没有放宽。
