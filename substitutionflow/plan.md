
  # 按 qe_phonon_* / workflow_status.json 重构 substitution_continue -rws 与 qe_phonon 状态判定

  ## Summary

  把 substitution_continue -rws 固定为“状态清洗 + 声子残留目录输入修复”模式，而不是“重新按目录痕迹判错”模式。执行一次 -rws 后：

  - 所有步骤里当前为 error 的状态统一改写成 unstarted
  - 已存在的 qe_phonon_* 残留目录会被修复为可继续被 qe_phonon_* -r 使用
  - 再执行 substitution_continue -r -o sub_struct 时，工作流应从每个 case 当前最后一个 finished 步骤之后继续推进

  同时重写 substitutionflow 对 qe_phonon 的状态判定，不再用“目录里有任何文件”判 error。qe_phonon 的真实阶段状态应以 qe_phonon_* 目录里的
  workflow_status.json 为准：只要还没全部 finished，就读取“最后一个 finished 后面的那个任务”的状态作为当前整体状态；running 仍只按你原先的 Slurm 工作目录
  匹配方案判定。

  ## Key Changes

  - 重新定义 -rws
      - 先执行一次真实 refresh_status(output_dir)，拿到当前 case/pressure 的最新状态。
      - 再把所有步骤中等于 error 的状态统一改成 unstarted。
      - finished 与 running 保持不变，不允许 -rws 覆盖。
      - 对每个被重置过的 pressure，同步修复已有 qe_phonon_nosplit / qe_phonon_split_index / qe_phonon_split_coords 子目录的基础输入。
      - 输出逐条重写日志，明确展示 case / pressure / step / error -> unstarted。
  - 重构 qe_phonon 总体状态判定
      - 不再使用“目录存在任意文件但未完成就判 error”。
      - 若 qe_phonon_* / workflow_status.json 不存在，且 Slurm 未命中该工作目录，则判 unstarted。
      - 若 Slurm 命中该工作目录，则判 running。
      - 若 workflow_status.json 存在，则按 qe_phonon_* 自身的阶段状态推导整体状态：
          - 全部相关阶段都 finished 才判 finished
          - 否则读取“最后一个 finished 后面的那个阶段”的状态，作为当前 qe_phonon 的状态
          - 该状态只允许映射到 running / error / unstarted
      - 这里“相关阶段”必须按不同 workflow 区分：
          - qe_phonon_nosplit：scf -> ph(state) -> q2r -> matdyn_phband -> matdyn_phdos
          - qe_phonon_split_index / split_coords：scf -> dyn0 -> split tasks -> merge -> q2r -> matdyn_phband -> matdyn_phdos
      - split workflow 中：
          - split tasks 作为一个聚合阶段，其状态来自 workflow_status.json["tasks"]
          - 只有全部 tasks == finished，split tasks 才视为 finished
          - 只要有任一 task 为 running，聚合状态为 running
          - 否则若存在 error，聚合状态为 error
          - 其余情况为 unstarted
  - 修复残留 qe_phonon_* 子目录
      - -rws 不删除残留目录，主方案是修复。
      - 对已有的 qe_phonon_* 目录，至少修复：
          - input.toml：一律用 case 根目录 input.toml 覆盖同步
          - POSCAR：一律用当前 pressure 下 vasp_opt/CONTCAR 覆盖同步
      - 不在 -rws 里预生成 run_scf.sh / ph.in / q2r.in / split 输入 等脚本和中间文件；这些继续交给 qe_phonon_* -r 自己基于修复后的输入生成。
      - qe_phonon_* 子目录里旧的、不完整的 input.toml 不再允许保留原样。
  - 保留现有 Slurm running 语义
      - running 仍只基于 squeue --format='%12A %12P %5t %25R %8M %Z' 返回的 %Z 工作目录匹配。
      - 不新增 PID 级判定，不改你原先的 Slurm 运行中判断方案。
      - 文档里明确这一点，避免误解为进程号或输出文件推断。

  ## Implementation Changes

  - substitutionflow/status.py
      - 重写 _detect_qe_phonon_status()，改为：
          - 先解析 workflow_status.json
          - 再结合当前 workflow 类型，推导阶段序列
          - 根据“最后一个 finished 后面的那个阶段”的状态映射整体状态
          - running 仍优先接受 Slurm 工作目录命中
      - 新增一个专门的 qe_phonon 状态归约 helper，输入为 workflow_name + workflow_status payload，输出为 finished / running / error / unstarted。
      - 对 split workflow，要把 tasks 聚合成单独阶段，而不是只看 matdyn_phdos。
      - 删除 qe_phonon 当前“目录里有任何文件即 error”的判定分支。
      - 其他步骤 vasp_opt / vasp_scf / vasp_eledos / vasp_process_eledos 的真实状态检测逻辑保持不变。
  - substitutionflow/workflow.py
      - 重写 rewrite_workflow_states(output_dir)：
          - 第 1 步：执行 refresh_status(output_dir)
          - 第 2 步：遍历所有 case/pressure，将所有 error 状态改为 unstarted
          - 第 3 步：对发生过重写的 pressure 调用 qe_phonon 残留目录修复逻辑
          - 第 4 步：写回 case_status.json 与根 workflow_status.json
          - 第 5 步：输出重写摘要和逐条日志
      - 新增 qe_phonon 残留目录修复 helper，只做：
          - 定位当前 case 的 qe_phonon 工作目录名
          - 若目录存在，则同步覆盖 input.toml
          - 若 vasp_opt/CONTCAR 存在，则同步覆盖 POSCAR
      - continue_workflow() 主语义不变，仍是“先刷新，再继续”；但在 -rws 之后，不应再因为 qe_phonon 残留目录被误判为 error 而卡死。
  - substitutionflow/structure_substitution.py
      - 抽一个可复用的 case 配置同步函数，供：
          - generate_cases() 生成 case 根 input.toml
          - rewrite_workflow_states() 修复 qe_phonon_* / input.toml
      - 该函数的语义固定为：完整继承 case 根配置，仅做目标路径写出，不做额外裁剪。
  - substitutionflow/cli_continue.py
      - 更新 -rws help 文案，明确：
          - 它会把所有 error 重置为 unstarted
          - 它会修复残留 qe_phonon_* 工作目录的基础输入
          - 它不是“重新把错误写回去”的模式
  - 文档
      - README 和相关说明中明确：
          - -rws 的目标是“清洗状态并修复续算入口”
          - qe_phonon 的整体状态来源于 qe_phonon_* / workflow_status.json
          - finished 的含义是对应 workflow 的所有相关阶段都 finished，不只是 matdyn_phdos 单点判断
          - running 只按 Slurm %Z 工作目录匹配