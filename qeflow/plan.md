我建议，将整个计算声子的流程拆分为  
qe_scf(已有)，  
qe_ph(没有，只进行ph.x的计算，包括三个qe_ph_nosplit(-dyn0是产生dyn0文件就终止，没有指定-dyn0就直接运行  
ph.x到结束),  
qe_ph_split_assign_index, qe_ph_split_assign_coords, 这两个split需要产生00，01，02，....的q点子目录，并且在里面准备好scf.in、split_ph.in、  
run_split.sh三个脚本)，  
qe_q2r, qe_matdyn(包含qe_matdyn_phdos, qe_matdyn_phband)，  
这些都是独立可以运行的CLI，可以单独使用。

另外, 重写qe_phonon里面的整体代码，我希望将上面的流程，整合为qe_phonon_nosplit, qe_phonon_split_assign_index, qe_phonon_split_assign_coords。

qe_phonon_split_assign_index, qe_phonon_split_assign_coords 工作流包含以下步骤

先写一个json文件纪录所有的状态都是unstarted

先判断，进行一系列的检查。（为了文件检查，你还需要写一个）整个检查在qe_check里面进行。  
比如检查自洽，qe_check -scf  
检查结构优化，qe_check -relax，检查结构优化的方法可以参考/home/mayuan/code/my_script/mytoolkit/qe_toolkit/cqopt.py和/home/mayuan/code/my_script/mytoolkit/qe_toolkit/VcOutPut.sh  
检查声子计算，qe_check -ph  
检查电声耦合，qe_check -eph  
检查q2r,     qe_check -q2r  
....

检查文件就涉及到复杂的读写文件，为此还需要建立专门的独写文件的代码和python 类。这里包括了其他所有文件独写的内容，都播暗含进来  
读取scf.out的类，读取ph.out的类（这里涉及对声子性质和电声耦合性质的读取, 具体可以参考/home/mayuan/code/my_script/mytoolkit/qe_toolkit/cqepc.py），读取relax.out的类，这些东西可以模仿/home/mayuan/code/dftflow/external/a2f中对代码撰写和分类。

qe_phonon_split_assign_index, qe_phonon_split_assign_coords 的检查流程如下：  
是否有scf.out并且是否完成计算，如果是，说明scf.out顺利完成，如果存在scf.out但是报错，就说明qe计算有问题，终止split PYthon程序; 如果不存在scf.out，就说明未开始scf.out，当前目录未开启任何计算。  
如果scf.out顺利完成，就再检查dyn0文件是否存在且文件大小不为0，如果是，就再检查各个q点坐标子目录中的scf.out和split_ph.out是否运行完成, 如果否，就说明产生dyn0时有问题。  
如果各个q点坐标子目录中的scf.out和split_ph.out全部是否运行，  
如果否，即不包含任何scf.out以及split_ph.out文件, 那么就产生输入文件，并且提交任务，打印所有的目录的情况, 然后退出程序，不再等待任务完成，回收任务，我会在未来任意时间执行这两个命令qe_phonon_split_assign_index, qe_phonon_split_assign_coords，这两个命令随时检查任务进行到了哪一步。  
如果是，即但凡包含一个out文件，都说明程序运行过，收集所有的目录，打印出来各个目录的情况。这里也涉及到对任务运行情况的判断。一起在qe_check里面检查。  
如果是，即所有的任务都顺利完成，scf.out和ph.out都完成，且没有报错，就执行qe_q2r, qe_matdyn_phband, qe_matdyn_phdos.

注意以上功能都是在qe_phonon_split_assign_index, qe_phonon_split_assign_coords内部完成，每次我只负责在当前目录执行e_phonon_split_assign_index, qe_phonon_split_assign_coords，你来判断我到底处于哪一个阶段，根据所处的阶段，执行任务。



scf (-s 就提交任务， -r就执行qe.execute_path的命令)  
nosplit（执行时加-dyn0 bool参数，直接获得dyn0文件就终止程序）  
然后根据dyn0的不可约q点数，产生q点子目录，并且根据assign_index 或者 assign_coords产生相应的split_ph.in文件和scf.in文件和run_split.sh的任务脚本，run_split.sh只包含pw.x的运行和ph.x的运行  
然后程序终止。

qe_phonon_split_assign_index 在开始计算前会先检查先前的声子计算是否完成，如果未完成，就查看是停止在了哪一步



新的 `qe_ph* / qe_phonon*` 系列，工作目录应该怎么组织？

› 1. 当前目录原地 (Recommended)  你在当前目录执行命令，直接检查/生成 `scf.in/scf.out/dyn0/q_00...` 和后续文件，不再额外包一层 `qe_phonon/`。先检查当前目录的base_name即最后/符号后的一个字符串是不是input.toml里面work_dir指定的。如果不是说明没在那个目录里面，就直接产生qe_phonon从头开始执行运行















# QE 声子工作流拆分与重构方案

## Summary

- 将现有 qe_phonon 体系拆成一组可单独运行的独立 CLI，以及一组带状态恢复能力的总工作流 CLI。
- 独立 CLI 负责“单步准备/执行”；工作流 CLI 负责“在当前目录判断所处阶段，并按 -r/-s 推进到下一阶段”。
- 保留可配 work_dir，但用户始终只在当前目录执行命令；CLI 从当前目录读取 input.toml，再自动进入 work_dir 做检查、写文件、提交和续算。
- 删除旧入口 qe_phonon、qe_phonon_split_assignQ、qe_phonon_split_coordinate、qe_phonon_merge_continue，全部切换到新命名和新语义。

## Key Changes

### 1. 新的 CLI 拆分

- 保留现有 qe_scf，不改其基础职责。
- 新增独立 CLI：

  - qe_ph_nosplit
  - qe_ph_split_assign_index
  - qe_ph_split_assign_coords
  - qe_q2r
  - qe_matdyn_phband
  - qe_matdyn_phdos
  - qe_check

- 新增总工作流 CLI：

  - qe_phonon_nosplit
  - qe_phonon_split_assign_index
  - qe_phonon_split_assign_coords

- qe_matdyn 总入口不新增，只保留 qe_matdyn_phband 和 qe_matdyn_phdos 两个独立 CLI。
- <code><span style="color:rgba(244,63,94,1)">eph 本次只体现在 qe_check 的接口预留层不实现实际能力；不新增 qe_eph CLI，也不在本次实现 qe_check -eph 的完整检查逻辑。</span></code>

### 2. 目录与执行语义

- 用户始终在“声子工作流根目录”执行命令。
- 当前目录必须包含 input.toml 和结构文件；CLI 从当前目录解析配置，再进入 [qe_phonon].work_dir 处理真实计算文件。
- work_dir 内部结构统一为：

  - 根级：scf.in/scf.out、ph.in/ph.out、q2r.in/q2r.out、matdyn_phband.in/out、matdyn_phdos.in/out、workflow_status.json
  - split 场景：<prefix>.dyn0、q_index_map.json、q_00/、q_01/、q_02/...

- q 点子目录固定使用 q_00、q_01、q_02 这种两位编号格式，与用户要求的 00,01,02... 对齐。
- qe_ph_split_assign_index / qe_ph_split_assign_coords 负责在每个 q 子目录下准备：

  - scf.in
  - split_ph.in
  - run_split.sh

- run_split.sh 只包含两步：

  - pw.x < scf.in > scf.out
  - ph.x < split_ph.in > split_ph.out


### 3. qe_ph_nosplit 与 split 准备 CLI 的职责

- qe_ph_nosplit

  - 只负责 ph.x 输入准备与执行。
  - 支持 --dyn0 布尔参数。
  - 指定 --dyn0 时，只运行到 dyn0 产生后即结束；不指定时，直接运行 ph.x 到正常结束。

- qe_ph_split_assign_index

  - 依赖已完成的根目录 scf.out。
  - 先生成 dyn0，再按不可约 q 点编号生成 q_00... 子目录及各自输入/脚本。

- qe_ph_split_assign_coords

  - 与上面相同，但 split_ph.in 改为按 q 点坐标写入。

- qe_q2r

  - 只负责准备/执行 q2r.x。
  - 默认输入来自根目录合并完成后的 dyn 文件集合。

- qe_matdyn_phband

  - 只负责准备/执行 matdyn.x 的声子能带部分。

- qe_matdyn_phdos

  - 只负责准备/执行 matdyn.x 的声子 DOS 部分。


### 4. 新的公共分层

- 在 qeflow/phonon 下拆出稳定公共层，替代当前“CLI 直接拼所有逻辑”的结构。
- 建议分层如下：

  - io/ 或 parsers.py

    - QeScfOutput
    - QeRelaxOutput
    - QePhOutput
    - 后续可扩展 QeQ2rOutput、QeMatdynOutput

  - status.py

    - 工作流状态文件构建、刷新、汇总

  - checks.py

    - qe_check 各类检查器分发

  - builders.py

    - scf/ph/q2r/matdyn 输入文件写入

  - workflow.py

    - qe_phonon_* 的阶段判定与推进


- 输出解析类参考 external/a2f/src/qe_outputs.py 的按职责拆分类方式，但接口收敛成当前仓库风格：(我建议，职责的分类方式适合做比较抽象，我更喜欢的是：以文件名命名的分类方式，这也更加直接，直接顾名思义就可以知道整个类在检查什么文件，比如scfout, dynout, relaxout, .....等等。)

  - 只暴露“完成态、错误态、关键提取字段”
  - 不把 CLI 打印逻辑塞进解析类

- 文件读写相关公共能力统一沉淀，不让 qe_check、独立 CLI、工作流 CLI 各自重复写 grep/扫描逻辑。

### 5. qe_check 设计

- 对外接口固定为单命令多开关：

  - qe_check -scf
  - qe_check -relax
  - qe_check -ph
  - qe_check -q2r
  - 后续可扩 -matdyn-band、-matdyn-dos

- 本次重点实现：

  - -scf
  - -relax
  - -ph
  - -q2r

- -relax 的完成/失败判定参考你给出的 cqopt.py 与 VcOutPut.sh：

  - 完成态：包含 JOB DONE，且若是 vc-relax 还可额外识别 End final coordinates
  - 失败态：存在输出但无 JOB DONE，或出现 QE 明显错误标记

- -scf 的完成态以 JOB DONE 为主，必要时补充关键成功标记。
- -ph 的检查需能区分：

  - dyn0 已完成
  - split 子任务未开始
  - split 子任务运行中
  - split 子任务报错
  - split 子任务全部完成

- qe_check 同时负责结合 squeue 判断 running，避免只靠文件存在误判。

### 6. split 工作流状态机

- 根目录落一份 workflow_status.json，每个 q_XX/ 子目录落一份 task_status.json。
- 状态枚举固定为：

  - unstarted
  - running
  - finished
  - error

- qe_phonon_split_assign_index / qe_phonon_split_assign_coords 的推进逻辑固定为：

  1. 初始化或刷新根状态文件，默认全部 unstarted。
  2. 检查根目录 scf.out：

     - 不存在：说明流程未开始；根据 -s/-r 触发 SCF 阶段
     - 存在且成功：进入下一步
     - 存在但失败：标记 error 并终止，不继续 split

  3. 检查 dyn0：

     - 不存在或空文件：说明 split 前置 ph.x 到 dyn0 未完成；根据 -s/-r 触发该阶段
     - 存在且非空：进入 q 子目录检查

  4. 检查所有 q_XX/ 子目录：

     - 若一个 out 都没有：说明 split 子任务尚未开始；生成全部 scf.in/split_ph.in/run_split.sh，按 -s/-r 触发后立即退出，不等待
     - 若已有部分 out：汇总每个目录的 unstarted/running/finished/error，打印状态后退出，不自动回收、不阻塞等待
     - 若全部 scf.out 和 split_ph.out 成功完成：进入后处理

  5. 自动执行：

     - qe_q2r
     - qe_matdyn_phband
     - qe_matdyn_phdos


- 总工作流 CLI 每次只推进“当前应推进的一步”，然后退出；不做长时间守候式等待。
- -s 表示写脚本并直接 sbatch。
- -r 表示顺次进入相应目录直接执行 QE 命令。
- 无 -r/-s 时只做检查、打印当前阶段和缺失项，不触发计算。

### 7. 配置与接口调整

- qeflow/templates.py 中重写 qe_phonon 模板，按新 CLI 共享同一套基础块。
- 继续复用顶层共享块：

  - [qe]
  - [slurm]
  - [pseudo_map]
  - pp_dir

- qe_phonon 配置块继续作为声子工作流公共配置来源，不为每个新 CLI 再造一套完全独立的 TOML 块。
- 新增或明确的行为包括：

  - work_dir 继续保留
  - qe_ph_nosplit --dyn0
  - split 策略按 CLI 名称决定，不再依赖旧的 split.strategy = "assignQ" 这种旧命名

- pyproject.toml 更新所有新 CLI 注册，并删除旧声子 CLI 注册。

## Test Plan

````
  - qe_ph_nosplit --dyn0 生成正确输入并只准备 dyn0 阶段命令。（我建议：终止的方案你可以参考/home/mayuan/code/my_script/qe/qe_submitjob.py里面的代码
  - qe_ph_nosplit 非 --dyn0 路径生成完整 ph.x 计算过程，不进行终止。
  - qe_ph_split_assign_index 正确生成 q_00...、scf.in、split_ph.in、run_split.sh
  - qe_ph_split_assign_coords 正确生成 q_00...、scf.in、split_ph.in、run_split.sh, 并且正确把单个 q 坐标写入 split_ph.in
  - qe_q2r、qe_matdyn_phband、qe_matdyn_phdos 正确生成各自输入文件
````

- 输出解析与检查：

  - QeScfOutput 能区分不存在、成功JOB DONE、失败
  - QeRelaxOutput 能识别 成功JOB DONE、未收敛、报错
  - QePhOutput 能识别 dyn0 是否存在且非空、split ph.out 是否成功
  - qe_check -scf/-relax/-ph/-q2r 在成功、失败、缺失、运行中（这个的检查尤其以来slurm系统，你需要好好看/home/mayuan/code/my_script/mytoolkit/many_jobsubmitting_system/slurm/prunslurm如何检查一个东西是否正在运行）四类场景下返回一致状态

- split 工作流：

  - 根目录无 scf.out 时，仅推进 SCF
  - scf.out 完成但无 dyn0 时，仅推进 dyn0 阶段
  - 有 dyn0 但无任何 q 子任务输出时，生成所有 q 子目录并按 -s/-r 触发后退出
  - q 子目录部分完成时，正确打印每个目录状态，不自动执行后处理
  - 全部 q 子任务完成时，自动执行 q2r + phband + phdos

- 提交/执行：

  - -s 场景断言实际提交脚本名正确
  - -r 场景断言实际执行命令序列正确
  - 不带 -r/-s 只检查不触发任何命令


› A previous agent produced the plan below to accomplish the user's task. Implement the plan in a fresh context. Treat the plan as the source of user
  intent, re-read files as needed, and carry the work through implementation and verification.

  ## QE 声子总工作流续推语义修正与 split 三阶段提交重构

  ### Summary
  后续实现只调整 `qe_phonon_*` 总工作流，不改变你要求保留的 8 个 CLI 外壳文件名与基本框架。目标语义固定为：

  - `qe_phonon_nosplit -r` 在同一次执行里，会从最后一个已完成步骤之后继续串行跑完整条剩余链，直到 `matdyn_phdos` 完成或某一步失败。
  - `qe_phonon_split_assign_index` / `qe_phonon_split_assign_coords` 同样支持续推，但 `-s` 固定采用“三阶段续推”语义，而不是第一次就把后处理直接串到同一个立
  即执行脚本里。
  - 三个总工作流命令都只需要在根目录重复执行同一个命令；程序自己依据状态文件和输出文件判断从哪里继续。

  ### Key Changes
  #### 1. `qe_phonon_nosplit` 改成真正的整链续推
  - `qe_phonon_nosplit -r` 的推进逻辑固定为：
    - `scf`
    - `ph`
    - `q2r`
    - `matdyn_phband`
    - `matdyn_phdos`
  - 每一步执行后立即刷新状态；若该步已 `finished`，同一次命令继续推进下一步，不再在 `ph` 阶段提前 `return`。
  - 若未来再次执行同一命令，则从最后一个已完成步骤之后继续跑，不重跑已完成步骤。
  - `qe_phonon_nosplit -s` 改为提交整条链总脚本，脚本内部顺序执行全部剩余未完成步骤。

  #### 2. 全部 `qe_phonon_*` 总工作流统一提前准备输入
  - `qe_phonon_nosplit` 在工作流入口就一次性准备：
    - `scf.in`
    - `ph.in`
    - `q2r.in`
    - `matdyn_phband.in`
    - `matdyn_phdos.in`
    - 各自 `run_*.sh`
  - `qe_phonon_split_assign_index` / `qe_phonon_split_assign_coords` 在入口也一次性准备所有当前可确定的根目录输入与脚本：
    - `scf.in`
    - `ph.in` 或 `dyn0` 所需输入
    - `q2r.in`
    - `matdyn_phband.in`
    - `matdyn_phdos.in`
    - 根目录 `run_*.sh`
  - split 场景里依赖 `dyn0/XML` 的 `q_XX/scf.in`、`q_XX/split_ph.in`、`q_XX/run_split.sh`，仍在 `dyn0` 完成后一次性全部生成，不提前伪造。

  #### 3. split 总工作流的 `-s` 固定为“三阶段续推”
  - `qe_phonon_split_assign_* -s` 不采用“单总脚本串到底”。
  - 它固定分成 3 个阶段：
    - 前计算阶段：`scf -> dyn0`
    - split 阶段：全部 `q_XX/run_split.sh`
    - 后处理阶段：`q2r -> matdyn_phband -> matdyn_phdos`
  - 第一次执行时：
    - 若 `scf/dyn0` 未完成，则生成全部阶段脚本，但当前只提交前计算阶段；
    - 若 `dyn0` 已完成但 `q_XX` 未完成，则生成全部 `q_XX` 输入和 split 阶段总脚本，并提交 split 阶段；
    - 程序提交后退出，不守候。
  - 以后再次执行同一命令时：
    - 若检测到全部 `q_XX` 都 `finished`，则自动生成并提交后处理阶段总脚本；
    - `q2r` 与两个 `matdyn` 后处理始终作为一个整体后处理阶段连续执行。
  - 这满足你说的“split 总体分为前计算、split_ph、后处理 3 个步骤”，并且后处理仍然通过再次执行同一个 `qe_phonon_split_*` 命令触发。

  #### 4. 状态判断与续推依据
  - 三个总工作流都统一依赖根目录状态刷新结果决定是否推进：
    - `scf`: `JOB DONE`
    - `ph`: `JOB DONE` 或 split 完成态
    - `q2r`: `JOB DONE + <prefix>.fc`
    - `matdyn_phband`: `JOB DONE + <prefix>.phband.freq`
    - `matdyn_phdos`: `JOB DONE + <prefix>.phdos.dat`
  - split 子目录仍按 `q_00`、`q_01`、`q_02` 命名。
  - `qe_check -eph` 继续只保留接口，明确输出“暂未支持”。
  - 不增加守候式轮询；每次命令只做“检查 -> 生成必要输入/脚本 -> 执行或提交当前应推进的阶段 -> 刷新状态 -> 退出”。

  #### 5. 文件与目录边界
  - 保留以下 8 个 CLI 文件，不做删除或合并：
    - `qe_matdyn_phband.py`
    - `qe_matdyn_phdos.py`
    - `qe_ph_nosplit.py`
    - `qe_ph_split_assign_coords.py`
    - `qe_ph_split_assign_index.py`
    - `qe_phonon_nosplit.py`
    - `qe_phonon_split_assign_coords.py`
    - `qe_phonon_split_assign_index.py`
  - `qgrid.py` 继续保留在 `qeflow/phonon/`，作为唯一需要重点保留、便于后续对比 QE `dyn0` 生成逻辑的核心算法文件。
  - `workflow.py` 继续保留在 `qeflow/phonon/`，作为这 8 个 CLI 共同依赖的调度中心。
    - `write_dyn0()`
  - 如需提升可比对性，只补充高信号注释，重点标明：
    - XML 读取
    - 不可约约化与权重累积
    - `dyn0` 写出
    - `scf` 未完成时，本次执行应顺次跑到 `matdyn_phdos`
    - `scf` 已完成、`ph` 未完成时，应从 `ph` 开始继续跑完整条后续链
    - `q2r` 已完成、`matdyn` 未完成时，只继续后半段
  - `qe_phonon_nosplit`
    - 首次 setup 后即存在 `scf.in`、`ph.in`、`q2r.in`、`matdyn_phband.in`、`matdyn_phdos.in` 及对应脚本
  - `qe_phonon_split_assign_index` / `qe_phonon_split_assign_coords`
    - 工作流开始时就存在根目录 `scf/ph/q2r/matdyn` 输入与脚本
  - `qe_phonon_* -s`
    - `nosplit` 生成整条链总脚本
    - `split` 生成三阶段脚本，并只提交当前应推进阶段
    - 所有脚本内容都只包含剩余未完成步骤
  - `qgrid.py`
    - 现有 gamma 优先与权重归一化测试保持通过
    - `dyn0` 写出格式测试保持通过

  ### Assumptions
  - 你确认保留现有 8 个 `qeflow/phonon/` CLI 外壳，不删除、不合并。
  - `source /work/home/mayuan/intel/oneapi/setvars.sh` 报错不是本次修复目标，且脚本里不需要加 `set -e`。
  - 总工作流里：
    - `nosplit -s` 采用整条链总脚本语义
    - `split -s` 采用“三阶段续推”语义
  - split 工作流中依赖 `dyn0/XML` 的 `q_XX` 输入，只在 `dyn0` 完成后生成；这不违背“全部预生成”，因为它们在入口时尚不可确定。




  ## QE Split 声子工作流补齐 Merge 阶段并扩展状态机

  ### Summary

  当前 qe_phonon_split_assign_* 和 qe_ph_split_assign_* 在所有 q_XX split 任务完成后，直接进入 q2r，中间缺少把各 q 点结果回收到根目录的 merge 步骤，导致
  q2r 在 split 路径下语义不完整。后续实现应把 split 工作流从“前计算 -> split -> 后处理”改为“前计算 -> split -> merge -> 后处理”，并把 merge 纳入
  workflow_status.json 的显式状态字段。

  这次实现采用你确认的范围：

  - merge 语义接入 qe_phonon_split_assign_* 和 qe_ph_split_assign_*
  - merge 状态单独记录为 "merge": "unstarted|running|finished|error"
  - merge 内容按旧脚本完整语义对齐，而不是只做最小 dyn 汇总

  ### Key Changes

  #### 1. split 状态机补一个独立 merge 阶段

  - 在 qeflow/phonon/workflow.py 的 split 分支中，把“全部 q_XX finished 后直接 q2r”改为“先 merge，再 q2r”。
  - workflow_status.json 顶层新增 "merge" 字段，位置语义上位于 ph 与 q2r 之间；当 split 全部完成但尚未汇总时，应出现：
      - "ph": {"state": "finished", "dyn0": "finished", ...}
      - "merge": "unstarted"
      - "q2r": "unstarted" 或 "q2r": "error" 不再作为“未 merge”替代态
  - split 总工作流的 -s 语义从 3 阶段改为 4 阶段：
      - 前计算：scf -> dyn0
      - split：全部 q_XX/run_split.sh
      - merge：根目录 merge 阶段
      - 后处理：q2r -> matdyn_phband -> matdyn_phdos
  - split 总工作流的 -r 语义改为：当全部 q_XX 完成后，同一次命令先执行 merge，merge 完成后继续执行 q2r -> matdyn_phband -> matdyn_phdos。

  #### 2. 新增并收口 merge 实现

  - 在 qeflow/phonon/ 恢复一个专用 merge 模块或在 workflow.py 中抽出等价 helper，但实现语义必须完整覆盖旧脚本需求，不只复制 dyn。
  - merge 的遍历顺序以根目录 q_index_map.json 为唯一真源，不依赖目录自然排序。
  - merge 输出目标固定回收到根目录，至少包括：
      - 根目录 prefix.dyn1..N 系列文件
      - _ph0/<prefix>.phsave
      - _ph0/<prefix>.q_<index>/... 下的 dvscf/dvscf1/<prefix>.dvscf/<prefix>.dvscf1 等候选文件
      - 旧脚本语义里要保留的 split_ph.in.<i>、split_ph.out.<i> 这类留档文件
  - merge 的命名规则必须稳定，不允许混用 q_00 目录号和物理 q 点编号；根目录输出编号应与 q_index_map.json 中的 index 对齐。
  - 若某个 q 点缺少 merge 必要文件，merge 状态记为 error，并阻止进入 q2r。

  你可以创建一个qe_merge.py的功能，用以放置merge功能。我也希望这个qe_merge就像qe_q2r.py, qe_matdyn_*.py一样，既可以单独运行执行，也可以综合到qe_phonon_split_*.py里面执行。

  #### 3. 状态判定与阶段推进规则同步更新

  - refresh_workflow_status() 需要新增 merge 检查函数，并把 merge 纳入打印和 JSON 落盘。
  - merge 状态判断建议采用以下规则：
      - unstarted：split 任务未全部完成，或根目录尚未出现任一 merge 目标
      - running：根目录 merge 脚本正在运行(没必要，因为这个回收过程非常快，只保留三个就行。)
      - finished：所有必须回收的 merge 目标都已在根目录到位
      - error：split 已完成，但 merge 目标缺失/不完整/脚本报错
  - q2r 不再承担“merge 尚未发生”的暗含语义；只有在 merge=finished 后才允许 q2r 进入执行或提交候选集。
  - qe_ph_split_assign_* 单步 split CLI 也要对齐：当 split 全部完成后，再次执行同一命令时应先执行 merge；该单步 CLI 仍然停在 merge 阶段，不负责继续跑 q2r/
    matdyn。
  - split 路径的阶段提示同步更新为 4 段编号，至少体现：
      - 前计算
      - 后处理

  - qe_phonon_split_assign_* -s 新增独立 merge 阶段总脚本，例如 submit_merge_stage.sh，并只在全部 q_XX finished 且 merge 未完成时提交它。
  - submit_post_stage.sh 中不再隐含 merge，只包含剩余 q2r -> matdyn_phband -> matdyn_phdos。
  - 若保留/恢复 qeflow/phonon/merge.py，对外仍不新增新 CLI；merge 只作为现有 split CLI 的内部阶段。
  - 对外可见接口变化只有：
      - workflow_status.json 新增 "merge" 字段
      - split 总工作流 -s 的阶段语义从 3 阶段变为 4 阶段

  ### Test Plan

  - qe_phonon_split_assign_index / qe_phonon_split_assign_coords
      - 全部 q_XX finished 后，workflow_status.json 出现 "merge": "unstarted"，且尚未执行 q2r
      - -r 时先执行 merge，再顺推 q2r -> matdyn_phband -> matdyn_phdos
      - -s 时第一次只提当前阶段；全部 q_XX finished 后再次执行命令时，只提交 merge 阶段；merge 完成后再次执行命令时，再提交后处理阶段
  - qe_ph_split_assign_index / qe_ph_split_assign_coords
      - 全部 q_XX finished 后，再次执行命令会执行 merge
      - 单步 split CLI 在 merge 完成后停止，不进入 q2r
      （我没明白你这个-s写的什么意思. 我的想法是：假设从split都已经完成了的步骤开始，-s意味着先在本地merge, 进接着就是提交q2r和matdyn的计算脚本。）
  - merge 本身
      - 能按 q_index_map.json 遍历并把所有约定文件回收到根目录
      - 缺少任一必要文件时抛错并把 merge 记为 error（还要详细爆出来哪些文件缺少，不要因为一个文件缺少就抛错终止，并且我希望merge的时候还要检查每一个split_ph.out是不是都算完了，只有检查到真的都算完了，再执行后面的q2r和matdyn，不是说合并就完事了，特别是合并的时候，
      我还想知道每个dyn文件频率最低的前5个频率的值是多少，而是所有文件都报错缺少后再终止程序。）

      - 根目录 merge 结果的编号与 q_index_map.json.index 一致，不受 q_00/q_01 字符串排序误导
  - 回归
      - nosplit 工作流行为不变
      - 现有 qgrid.py gamma 优先、权重归一化、dyn0 写出测试继续通过
      - 原有 split 输入生成与阶段推进测试更新为包含 merge 阶段

  ### Assumptions

  - merge 的实现以你给的旧脚本语义为准，不做“只够 q2r 用”的裁剪。
  - 不新增独立 qe_merge CLI；merge 只作为 split 工作流内部阶段。
  - split 总工作流从现在开始以“前计算 / split / merge / 后处理”四阶段为正式语义，相关脚本命名和状态打印都应随之调整。


  › A previous agent produced the plan below to accomplish the user's task. Implement the plan in a fresh context. Treat the plan as the source of user
  intent, re-read files as needed, and carry the work through implementation and verification.















  # QE split 声子工作流修正计划：q 目录从 `q_01` 起编号，并按 `coords/index` 分离 merge 语义

  ## Summary
  当前 `qeflow` 的 split/merge 有两个核心问题需要一起修正：

  1. q 子目录仍按 `q_00`、`q_01`... 编号，而你要求从 `q_01` 开始。
  2. `merge` 现在把 `assign_coords` 和 `assign_index` 当成同一种产物结构处理，但我核对了你给的旧脚本和 `/home/mayuan/code/my_script/qe/
  qe_inputpara.py::merge()` 后，确认两种模式的 dyn/tmp/phsave 语义必须分开实现。

  这次计划会把 split 工作流改成“目录编号规则明确 + merge 按模式分流 + 状态检测按模式判定”的完整方案，避免继续用“一个规则兜所有模式”。

  ## Key Changes

  ### 1. q 子目录编号统一改为从 `q_01` 开始
  - `materialize_split_directories()` 中子目录命名从 `q_{dir_index:02d}` 改为 `q_{dir_index + 1:02d}`。
  - `q_index_map.json` 的 `directory` 字段同步写成 `q_01`、`q_02`...，不再出现 `q_00`。
  - 所有遍历 split 任务的逻辑继续以 `q_index_map.json` 为真源，不依赖目录自然排序，也不依赖字符串中的数字起点。
  - 相关测试全部改成断言 `q_01` 为首目录。
 （我的建议：你可以在q_index_map.json纪录q点的分割方式。但是没必要根据q_index_map.json确定要用哪种合并方式，因为我希望你可以把两种不同的
 合并方式分别置于qeflow/phonon/qe_phonon_split_assign_coords.py和qeflow/phonon/qe_phonon_split_assign_index.py里面。我自己看了
 q_index_map.json就知道改用哪个CLI了。这样也可以避免两个CLI都实现两次merge方案。比如qeflow/phonon/qe_phonon_split_assign_coords.py自动
 利用qe_merge来运行coord的合并方式。）
  ### 2. 明确 split 模式元信息，并作为 merge 唯一判据
  - 在 `q_index_map.json` 顶层新增明确模式标记，例如记录当前 split 是 `split_assign_coords` 还是 `split_assign_index`。
  - merge 与 `check_merge()` 不再通过“猜文件名”推断模式（我说的也不是猜文件名，而是通过CLI脚本内置了指定的qe_merge合并方式。），而是直接读取 `q_index_map.json` 的模式字段。
  - `qe_merge`、`run_merge_stage()`、`refresh_workflow_status()` 全部统一复用这份模式信息，避免不同入口出现不同判定。

  ### 3. `assign_coords` 的 merge 语义按旧脚本重做
  依据你给的 `pre_for_specifyQ_with_coordinates.sh` 和旧 `qe.merge()` 逻辑，coords 模式固定按下面语义实现：

  - 每个 q 子目录中的 dyn 源文件优先按“无编号”版本读取，即 `<prefix>.dyn`；若实际 QE 输出是 `<prefix>.dynN`，可作为兼容兜底，但不是主语义。
  - 根目录合并结果仍按物理 q 编号统一回收到 `<prefix>.dyn1..N`，保证 `q2r` 输入稳定。
  - 每个 q 子目录的 `split_ph.in/out` 仍回收到根目录 `split_ph.in.<index>`、`split_ph.out.<index>`。
  - `_ph0/<prefix>.phsave` 在 coords 模式下只要求单 q 版本的 `patterns.1.xml` 布局；不要求存在 `patterns.<index>.xml`。
  - coords 模式下每个 q 子目录里若存在 `_ph0/<prefix>.q_<index>` 则复制；若不存在该层而 dvscf 直接位于 `_ph0/`，也必须识别并按旧脚本语义回收到根目录。
  - `check_merge()` 不能再对 coords 模式强制要求根目录必须存在 `_ph0/<prefix>.q_<index>`；它应按 coords 实际必需产物判定 finished/error。

  ### 4. `assign_index` 的 merge 语义按旧脚本重做
  依据你给的 `pre_for_specifyQ_with_Index.sh`、`QE_error_collecting.md` 和旧 `qe.merge()` 逻辑，index 模式固定按下面语义实现：

  - dyn 源文件主语义是带物理 q 编号的 `<prefix>.dyn<index>`；若个别环境输出成 `<prefix>.dyn`，可以兼容读取，但最终回收到根目录时必须命名为
  `<prefix>.dyn<index>`。
  - 第 1 个 q 点和其余 q 点的 tmp 结构要区别处理：
    - 第 1 个 q 点允许 dvscf 直接位于 `_ph0/`。
    - 从第 2 个 q 点开始，dvscf 主语义位于 `_ph0/<prefix>.q_<index>/`。
  - `phsave/patterns` 在 index 模式下要保留“物理 q 编号语义”：
    - merge 校验需要识别 `patterns.<index>.xml` 这类真实编号文件。
    - 若为了后续 QE 继续读取需要回写成 `patterns.1.xml` 并修正 XML 内 `<QPOINT_NUMBER>`，则按旧脚本语义显式执行，不做推断式处理。
  - `check_merge()` 对 index 模式应要求根目录具备 `<prefix>.dyn<index>` 和对应 `_ph0/<prefix>.q_<index>` 等必要产物；第 1 个 q 点允许走“无 q 子目录
  dvscf”特例。

  ### 5. `qe_merge` 与工作流状态机同步收紧
  - `qe_merge` 继续作为唯一 merge 实现入口，但内部实现拆成：
    - 读取 `q_index_map.json`
    - 根据模式分派到 `merge_split_results_for_coords()` / `merge_split_results_for_index()`
  - `merge` 成功摘要继续打印每个 dyn 文件最低前 5 个频率，但来源解析要兼容 coords/index 两种 dyn 命名。
  - `workflow_status.json` 的 `"merge"` 仍只保留 `unstarted / finished / error` 三态。
  - `check_merge()` 的 finished/error 规则必须按模式区分，不能继续用“所有模式都要有 `<prefix>.dynN + _ph0/<prefix>.q_N`”这种统一要求。
  - `qe_ph_split_assign_* -s`、`qe_phonon_split_assign_* -r/-s` 的推进逻辑保持不变，但 merge 成败判定改用新的模式化检查结果。

  - `q_index_map.json` 增加 split 模式字段，作为 merge/状态检测唯一真源。
  - `qe_merge` 的对外命令行参数不变，但其内部行为会根据 `q_index_map.json` 中的模式自动切换 coords/index merge 规则。
  - `workflow_status.json` 结构维持现有字段，但 `"merge"` 的判定规则会变成模式相关。

  ## Test Plan
  - split 目录生成
    - `qe_ph_split_assign_index` 首目录改为 `q_01`，不再生成 `q_00`。
    - `qe_ph_split_assign_coords` 首目录改为 `q_01`，并保留单 q 坐标写入逻辑。
    - `q_index_map.json` 记录 `directory=q_01...` 且包含 split 模式标记。

  - coords merge
    - 子目录中 dyn 只有 `<prefix>.dyn` 时，`qe_merge -r` 能成功回收为根目录 `<prefix>.dyn1..N`。
    - coords 模式下 `_ph0` 若不存在 `<prefix>.q_<index>`，但 dvscf 直接位于 `_ph0/`，merge 仍能成功。
    - coords 模式 finished 判定不再错误要求根目录存在 `_ph0/<prefix>.q_<index>`。

  - index merge
    - index 模式优先读取 `<prefix>.dyn<index>` 和 `_ph0/<prefix>.q_<index>/...`。
    - 第 1 个 q 点走 `_ph0/` 直存 dvscf 特例时，merge 仍能成功。
    - patterns 文件按 index 语义复制/改写后，根目录校验通过。

  - 工作流推进
    - `qe_phonon_split_assign_*` 在全部 q 子任务 finished 后，第一次 `-s` 只提交 merge 阶段；merge 完成后再提交 post 阶段。
    - `qe_ph_split_assign_* -s` 在 split 全部完成后先本地 merge；coords/index 各自成功时提交 post，失败时不提交。
    - `workflow_status.json` 在 coords/index 两种模式下都能正确给出 `"merge": "unstarted|finished|error"`。

  ## Assumptions / Defaults
  - 根目录最终给 `q2r` 使用的 dyn 文件统一命名为 `<prefix>.dyn1..N`，不因 coords/index 模式改变。
  - `q_index_map.json` 是 merge 的唯一真源；不会再通过目录名或文件名反推 split 模式。
  - coords 模式的主语义按你给的旧脚本收敛为“子目录 dyn 可无编号、patterns 主语义为 `patterns.1.xml`、tmp 结构与 index 不同”。
  - index 模式的主语义按你给的旧脚本收敛为“dyn/`q_<index>`/patterns 都保留物理 q 编号语义，第 1 个 q 点允许特殊目录结构”。