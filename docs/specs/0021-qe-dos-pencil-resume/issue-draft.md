# 0021：修复 DOS FFT 空进程并明确旧尝试恢复提示

- 唯一类型：`type:bug`。
- 平台：origin 为 Gitee；沿用本地 Issue／Spec 落档、线上 Gitee 用户管理约定。本地编号经目录查重未占用，不冒充线上编号。
- 关联：[0018](../0018-epwflow-stage-workspaces/issue-draft.md)、[0019](../0019-shell-script-options/issue-draft.md)、[0020](../0020-qe-electronic-postprocess-resume/issue-draft.md)。
- 规格：[spec.md](spec.md)。

## 原始需求记录

> 为什么我刚刚在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run提交了epw_wannier --stage wannier -s然后跑了一下就结束了，又报错了！！！！ 你来看看/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/5.epw-wannier里面的报错内容？看看是不是需要修改代码，继续需要使用 issue 和 spec 来新建需求修改。不要再征求我的意见。

## 目标、范围及依据

1307064 为公开命令提交，bands 已完成，dos 在 32 MPI 下报 FFT 空进程（退出 6）。1307068 的 Slurm 提交记录为 `sbatch submit.sh`，复用了已绑定 1307064 的尝试，正确遭作业身份拒绝。原始日志和调度记录保存在忽略的 docs/local/0021-dos-pencil-resume/。

修复 DOS 自身并行参数，保留成功 PW／bands 与当前物理设置；保留旧尝试作业和脚本身份校验，改善错误提示。通过公开命令重新生成尝试并恢复；不修改历史脚本或清单、不代提交科学作业、不推送。

## 验收标准

1. DOS、bands 均使用 32 MPI 和自身 pencil 选项，无 PW／EPW pool 泄漏，其他程序不误收参数。
2. 模拟 bands 成功、DOS 失败后，再次执行公开 `-s` 生成新尝试并完成；PW、PH、bands 不重跑。
3. 重提旧脚本非零拒绝且输出旧／新作业号和正确命令；活动重复提交、脚本哈希与冻结配置保护继续有效。
4. 真实来源文件集合与哈希不变；回归、构建和安装分别留证，不以协议测试冒充真实科学通过。

## 授权处理

用户明确要求“不要再征求我的意见”，本次直接完成需求落档及实施；不将未展示的草案记作另行审查确认。此明确指令覆盖 Skill 的重复确认步骤。

## 文档影响（Documentation Impact）

- [x] 更新 docs/overview/epwflow/README.md。
- [x] 更新本目录规格、设计、任务及验收记录。
- [x] 更新 epwflow/README.md 及 MgB2 run 两级说明。
- 根 README 无需改：公开命令、TOML 及安装入口未变。
- 远程 Issue／Spec 双向链接待用户上线后核验；本次不关闭线上 Issue。

## 本地交付证据

见 [acceptance.md](acceptance.md)：209 通过、7 默认暂挂，1999 份计算及状态文件不变，三个 EPW 程序已安装并核对。尚未代执行真实 DOS／Wannier，不将协议通过冒充科学完成；线上协作状态仍由用户管理。
