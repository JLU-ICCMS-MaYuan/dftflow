# 0046：简化 QE／EPW 提交脚本并移除全部运行锁

唯一类型建议：`type:feature`。日期：2026-10-22。状态：本地草案，待用户审查；未实施。

origin 为 Gitee，未安装 `gitee` CLI；沿用线上由用户管理、本地保存草案的既有约定。0046 是本地编号，不代表平台 Issue 已创建。未创建或关闭线上 Issue，远程双向链接待验证。

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

## 附件与推断来源

无上传附件或图片，`images: []`。用户提供的是报错文字及路径。

源码、运行目录观察、删除影响及 AI 建议分别列在[状态功能清单](state-inventory.md)。清单中的“建议”不代表用户同意删除。

## 目标与范围

让 QE／EPW 脚本直接说明任务用途、环境和计算命令；计算启动时就写入指定 `.out`，重复运行可覆盖同名脚本和 Slurm 日志；移除全部运行互斥锁，包括预算账本／注册表锁。交付完整状态清单，供用户决定其余状态的去留。

本草案聚焦 QE／EPW 脚本和运行路径，不扩展到 VASP 等其他流程。全部运行锁包括当前与旧 EPW 路径的控制／执行／workflow／链提交／迁移／导入／快照／比较／出图锁，用户级预算账本与 campaign 注册表锁，以及共享 QE 的 `.epw-save.lock`、链提交的 `.chain-submit.lock`；不以其他文件锁、进程锁或锁服务替代。

删除锁不删除账本、作业记录和科学交接数据；这些普通记录暂保留，额外删减仍待评估。去锁后不再保证多个命令同时修改同一目录或账本的互斥；顺序执行继续使用已有作业状态检查。依赖版本文件 `Cargo.lock`、正则缓存 `OnceLock` 和普通原子文件写入不属于本需求的运行锁。

## 现状与问题依据

- `foundation/native_jobs.rs::prepare` 同时产生带 token 的正式脚本和 `submit.sh`，写入带 token／作业号的两份 Slurm 日志，正文和 Slurm 参数重复设置目录，逐参数单引号及环境 `if` 包装均存在。
- `native_jobs::reconcile` 把 Slurm 输出复制为科学 `.out/.err`。因此需要修改实际输出路径和回执读取，不能只改脚本外观。
- `workspace::control` 在入口取得目录锁；锁目录一旦存在即拒绝。自动恢复依赖旧 `active.json` 和匹配的 Slurm 作业，不能普遍处理登录节点命令中断后的残留。
- 运行目录在检查期间发生变化。2026-10-22 10:48:27 最新观察：`control.lock` 不存在，`native-active.json` 登记电子能带任务 1327914 为 `submitted`。这不是对用户先前报错的否定，也不是实时 Slurm 查询结果；未复现完整“中断后手动删锁”操作序列。

## 验收结果要求

1. 新脚本仅采用任务用途名称；同一用途的 Slurm 标准输出／错误均指向 `slurm-<用途>.log`，允许覆盖，不使用随机值、哈希或作业号区分文件。
2. QE／EPW 命令直接以标准输入读 `.in`，标准输出写 `.out`；科学解析从该 `.out` 读取，不再依赖复制 Slurm 日志。
3. 保留正文 `cd`，删除重复的 `#SBATCH --chdir` 和提交参数 `--chdir`；直接加载 oneAPI；普通参数清晰可读，仅必要时引用。
4. 不再创建、取得、释放或恢复任何范围内运行锁；包括空锁、带 owner 锁和预算锁在内的旧残留不再阻止公开入口。不以换名称或换锁实现替代。
5. 同名脚本／Slurm 日志合法覆盖不会触发历史回执指纹冲突；计算失败仍不能误报成功。
6. 明确状态清单中每项的用途、读取关系、删除后果、当前路径及建议；全部锁已选定删除，其他状态不自动删减。
7. 使用实际生成脚本、进程执行和提交模拟验证输出、引用、中断及重试边界；不靠字符串检查代替行为验证。

## 关系与冲突取舍

- 关联 [0022](../0022-slurm-log-filenames/issue-draft.md)、[0024](../0024-slurm-script-simplification/issue-draft.md)、[0044](../0044-mpi-submit-command/issue-draft.md)。0024 仍是未确认草案；本项覆盖新的明确写法及控制锁问题，不重复声称旧项已实现。
- 关联 [0025](../0025-epw-visible-run-state/issue-draft.md)、[0026](../0026-epw-direct-native-submit/issue-draft.md)、[0035](../0035-native-retry-slurm-logs/issue-draft.md)。固定脚本／日志名称与原有不可变脚本／日志身份检查存在冲突，需要一并处理。
- 采用本次较新要求：允许覆盖与合并 Slurm 日志、保留正文 `cd`、直接 `source`。不再要求对环境加载包裹 `if` 或保证它单独失败时立即停止；不引入严格选项来恢复这项旧行为。
- 不新增 Epic／父子关系；作为已有 QE／EPW 执行能力的独立调整。具体需求与场景见 [Spec](spec.md)。

## 范围外

不改变 PBE、iso／aniso 温度点、物理参数和累计预算；不重新要求 LDA 或 Tc 搜索。不自动清空计算目录、账本或归档，不提交科学作业，不重写正在运行的脚本，不切换安装，不推送。不删除科学软件内部的运行机制、依赖版本文件或无关模块的锁。

## Documentation Impact

- [ ] 实施后更新 `docs/overview/epwflow.md` 及必要的共享执行约束；当前不把草案写成已实现。
- [ ] 本目录 Spec 需完成范围选择、最终确认、设计、任务和验收证据；目前只完成需求草案与只读清单。
- [ ] 实施后更新 MgB₂ 示例 README 的脚本／日志／手工提交说明，保持可复制命令形式；检查模块及根 README 是否受影响，无新增公共入口则说明不需改动。
- [ ] 由用户发布后核验线上 Issue／Spec 双向链接，当前不能勾选。

## 澄清与待确认

用户已明确选择删除全部锁，包括预算账本锁；据此扩大去锁范围。其他状态只交付评估清单，未授权的删减不纳入本次实现要求。Issue／Spec 修改后的最终草案仍未获最终确认；未建立 Plan、Tasks 或实现基线。
