# 0046：移除 EPW 工作区控制／执行锁并简化提交脚本

唯一类型建议：`type:feature`。日期：2026-10-22。状态：本地源码与文档已实现，隔离回归已验证；目录规则与默认安装更新均已验证；线上 Issue 未关闭。

origin 为 Gitee，沿用线上由用户管理、本地保存草案的约定。0046 是本地编号，不代表平台 Issue 已创建；远程双向链接未验证。

## 原始需求记录
```text
下面是对现有任务提交脚本、epwflow 工具的设计调整要求，以及问题反馈：

1. 脚本命名规则 不需要用哈希值区分提交任务脚本，统一采用 `<提交任务的功能简述>.sh` 命名。 slurm 的 `#SBATCH --output=`、`#SBATCH --error=` 输出文件也不需要哈希区分，允许反复覆盖，命名为 `slurm-<提交任务的功能简述>.log`。
2. QE / EPW 运行命令格式 推荐提交命令格式：`mpirun -np xxx.x < xxx.in > xxx.out`，把计算输出重定向到指定 out 文件，而不是放到 slurm log 里。
3. 工作目录切换 脚本内部已经自带类似 `if ! cd -- '/xxxx'; then exit 1; fi` 的目录切换逻辑，不需要再额外写 `#SBATCH --chdir='xxx'`，二者只保留其一即可。
4. oneapi 环境加载`source '/data/home/mayuan/intel-2024/oneapi/intel2024_setvars.sh' --force` 这一行，不需要套 if 判断，直接执行就行。
5. 命令参数单引号问题 类似这条示例命令：`'mpirun' '-np' '32' '/data/home/mayuan/soft/qe-7.4.1-amd-oneAPI2024/bin/pw.x' '-inp' '/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/2.qe-nscf/nscf.in'` 不要给每个参数都单独加单引号，这种写法观感很差，即使能跑也不采用。
6. control.lock 锁文件问题 报错：`scheduler_unknown: scheduler_unknown: 锁 /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epw-state/control.lock 已存在；核对 owner 和未决提交后恢复，不能删除未知锁.` 问题现象：任务启动后中途终止，再次执行 `epw_phonon -c "epwinput.toml" --stage electronic -s` 提交任务就被锁拦截；手动删除 control.lock 也无法恢复提交。 需求：强烈要求移除这个锁机制。 另外请详细列出 epw-state 目录下全部功能，我们评估哪些保留、哪些删掉。
7. 设计原则要求 当前实现存在明显的过度设计、多余逻辑。很多上面这些复杂机制我并没有要求实现。 开发请严格遵循 SOLID、KISS、DRY、YAGNI：

- 追求代码和设计极致简洁
- 拒绝不必要的复杂性
- 优先选择最直观的解决方案

使用 issue 和 spec 完成。
```

> 删除全部锁，包括预算账本锁

```text
我觉得这样的打击面实在是太大太广了。 不利于一点点修复代码的思路。我还是主要集中于解决`ntrol.lock`、`execution.lock` 等锁限制同时操作工作区全部移除的问题。我认为我在同一个目录执行一个命令，就不应该第二次执行的之后被这个东西锁住。其他的都先不管，先只解决这一个问题。



另外要优化的地方是：

1. 脚本命名规则 不需要用哈希值区分提交任务脚本，统一采用 `<提交任务的功能简述>.sh` 命名。 slurm 的 `#SBATCH --output=`、`#SBATCH --error=` 输出文件也不需要哈希区分，允许反复覆盖，命名为 `slurm-<提交任务的功能简述>.log`。
2. QE / EPW 运行命令格式 推荐提交命令格式：`mpirun -np xxx.x < xxx.in > xxx.out`，把计算输出重定向到指定 out 文件，而不是放到 slurm log 里。
3. 工作目录切换 脚本内部已经自带类似 `if ! cd -- '/xxxx'; then exit 1; fi` 的目录切换逻辑，不需要再额外写 `#SBATCH --chdir='xxx'`，二者只保留其一即可。
4. oneapi 环境加载`source '/data/home/mayuan/intel-2024/oneapi/intel2024_setvars.sh' --force` 这一行，不需要套 if 判断，直接执行就行。
5. 命令参数单引号问题 类似这条示例命令：`'mpirun' '-np' '32' '/data/home/mayuan/soft/qe-7.4.1-amd-oneAPI2024/bin/pw.x' '-inp' '/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/2.qe-nscf/nscf.in'` 不要给每个参数都单独加单引号，这种写法观感很差，即使能跑也不采用。

设计原则要求 当前实现存在明显的过度设计、多余逻辑。很多上面这些复杂机制我并没有要求实现。 开发请严格遵循 SOLID、KISS、DRY、YAGNI：



请修改这个#0046 为当前的问题。 至于预算账本所是不是要接触，暂时不动。
```

> 开始落实

> 更新默认安装

```text
去掉 --chdir 后，工具仍可在脚本目录提交.  也就是说：无论这个提交任务的脚本在哪里，都可以 sbatch xxx.sh提交，它会自动切到目标目录下把这个任务提交了，通过 squeue 的命令也可以看到这个任务脚本是在那个--chdir 指定都目录下面提交的，是吗？这是 --chdir的作用。

但是在任务脚本里面写 cd xxx的命令，就是提交任务之后，脚本自己执行进入这个目录，squeue检查也是发现这个任务的存在路径  是在 其他目录下，而不是目标目录下。

那如果是这样的话，我觉得保留--chdir 就足够了，任务脚本会被自动切换到目标目录下提交，也不需要 cd  xxx的命令了。 是不是这样的
```

## 附件与推断来源

无上传附件或图片，`images: []`。源码与故障观察见[状态清单](state-inventory.md)；该清单仅为背景资料，不是本次全面状态重构的任务表。

## 当前问题与目标

在同一目录执行 `epw_phonon -c "epwinput.toml" --stage phonon -s`，命令中断后留下 `control.lock`，再次执行就被锁挡住。先解决这类工作区控制／执行锁造成的重复操作障碍，再按以下五项要求简化同一路径生成的提交脚本。

## 最新范围与优先级

1. **优先修复工作区锁**：移除 `epw-state/control.lock`、`epw-state/execution.lock`，以及超导阶段同类 `execution.lock` 的获取、释放、恢复和入口拦截；旧残留不再阻止公开命令，不增加替代锁或锁恢复框架。
2. **固定脚本和日志名称**：脚本为 `<用途>.sh`，Slurm 输出与错误均为 `slurm-<用途>.log`，允许覆盖；不使用哈希、时间或作业号，不保留一份仅用于阅读的重复脚本。
3. **科学输出直接落盘**：QE／EPW 使用 `mpirun -np <进程数> <程序> < 输入.in > 输出.out`，直接生成供解析的 `.out`，不从 Slurm 日志复制拼装。
4. **只保留调度器目录设置**：新原生脚本用 `#SBATCH --chdir=绝对目标目录`，删除正文 `cd`；工具仍在脚本目录提交，手工也可从其他目录通过脚本路径提交。不额外添加命令行 `--chdir`。
5. **直接加载 oneAPI**：使用直接 `source <环境脚本> --force`，不包裹 `if` 或注入严格选项。
6. **必要引用**：普通命令及参数直接显示，不逐个单引号；特殊路径保留必要引用，参数含义不变。

工作区锁修复可以先单独实现和验收，不以五项脚本优化或其他状态整理完成为前提。脚本／日志允许覆盖所必需的回执读取调整属于脚本优化，不扩展为重构整个任务状态体系。

## 需求变更取舍

原先“删除全部锁，包括预算账本锁”的答复作为历史原话保留，但已经被本轮“暂时不动预算账本、先解决工作区锁”的指令替代。当前不修改预算账本锁、campaign 注册表锁及其读写逻辑；不批量删除迁移、快照、提交链、QE 归档、比较／结果发布／绘图等其他锁。不合并或删除其他状态文件。

后续用户选择仅保留 `--chdir`，替代本版此前“正文 cd、不写 --chdir”的实现选择；仍只保留一种目录设置。`--chdir` 决定任务启动目录和 Slurm WorkDir，不改变调用 sbatch 的原目录或 SLURM_SUBMIT_DIR。

用户原话中的 `ntrol.lock` 按本轮上下文理解为此前复现的 `control.lock`，只在需求解释中说明，不修改原文。

## 验收要求

- 中断公开命令后再次执行同一命令，不因工作区控制／执行锁拒绝，不需要手动删除锁；同时覆盖旧空锁和带 owner 的旧锁。
- 已有活动 Slurm 作业继续按作业记录显示当前状态，不因移除锁而把失败或未知提交当作成功；普通顺序重复命令不重复提交同一活动作业。
- 各代表性新脚本满足上述五项写法；隔离执行可在计算尚未退出时读到指定 `.out`；合法覆盖脚本／Slurm 日志不触发旧指纹冲突。
- 预算账本、注册表、相关锁、科学参数、已有科学文件及其他状态机制不变。工作区多命令同时写入的互斥不再由这些已移除锁保证。

## 关系

关联 [0022](../0022-slurm-log-filenames/issue-draft.md)、[0024](../0024-slurm-script-simplification/issue-draft.md)、[0044](../0044-mpi-submit-command/issue-draft.md) 的脚本规则，采用本次较新的明确写法；关联 [0026](../0026-epw-direct-native-submit/issue-draft.md)、[0035](../0035-native-retry-slurm-logs/issue-draft.md) 的提交／恢复行为，仅修改本需求必需的部分。无新增 Epic／父子关系。详细场景见 [Spec](spec.md)。

## 范围外

预算账本和注册表及其锁、全面去锁、全部状态重构、旧流程清退、全仓库脚本重写均不在本次范围。不更改物理模型、温度点和科学验收目标；不清空计算目录，不提交科学作业，不推送。默认安装更新按后续明确授权执行。

## Documentation Impact

- [x] 已更新 `docs/overview/epwflow/README.md` 中工作区锁、脚本与科学输出的现状。
- [x] 本目录 Issue／Spec、检查表与清单采用最新范围；实施设计、任务和验收已建立。
- [x] 影响说明已更新在模块手册和 Overview；公开命令、TOML 与目录布局不变，无须改写 MgB₂ 示例命令。本次保留示例 README 的已有未提交修改，详见验收记录。
- [ ] 线上 Issue／Spec 双向链接由用户发布后核验，当前未完成。

## 确认与实施状态

本轮用户明确授权修改 0046 的范围；已按该指令修订本地文档，不要求再次确认是否保留预算锁。随后用户明确要求“开始落实”，据此实施当前收窄范围；线上 Issue 继续由用户管理。

本地实施与验证已完成，详见[验收记录](acceptance.md)。此前源码交付未更新默认安装；本次按“更新默认安装”授权完成三个 EPW 程序的制品、PATH 入口和新脚本生成核验。真实计算和远程发布不在本次操作内。
