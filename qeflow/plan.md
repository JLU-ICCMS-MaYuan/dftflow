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


## Assumptions

- 旧声子 CLI 全部删除，不做兼容转发。
- q 点子目录命名采用 q_00、q_01、q_02。
- qe_check -eph 本次不实现；若保留参数，也只明确提示“暂未支持”，不做半成品逻辑。
- 成功态判断以 QE 输出中的 JOB DONE 和关键产物存在性为主；如后续你有更严格的关键词标准，可在实现时统一补入解析类常量中。
- 总工作流 CLI 不做守候式轮询；每次执行只检查、推进当前一步、打印状态，然后结束，符合“未来任意时间再次执行同一命令继续推进”的要求。