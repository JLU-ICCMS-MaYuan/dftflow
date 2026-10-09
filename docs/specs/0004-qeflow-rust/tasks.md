# 技术拆解：qeflow Rust 重写

按 crate 拆解可独立领取、实现与验收的技术任务。

## qf-config
- [ ] 定义 `Value` 递归合并函数，严格复刻 `_merge_dict_in_place`：`dict×dict` 递归，否则覆盖，`None`/缺失键不覆盖。
- [ ] 解析 `input.toml`，合并 `[qe]/[slurm]/[qe_phonon]` 等块，保留现有默认模板。
- [ ] 暴露 `get_section("qe")` 等查询接口供其余 crate 使用。

## qf-crystal
- [ ] POSCAR 解析（元素、坐标、晶格）。
- [ ] 化学式与 kmesh 派生。
- [ ] ibrav 自动判断（`cell2ibrav.x` 优先，Python fallback 对照）。
- [ ] 三模式晶格写出：`ibrav+ABC/cos`、`ibrav+celldm`、`ibrav=0+CELL_PARAMETERS {angstrom}`。

## qf-input
- [ ] `pw.x` 输入写出（scf/nscf/bands），对齐 `pw.py`。
- [ ] `ph.x`/`q2r.x`/`matdyn.x` 输入写出，对齐 `ph.py`/`q2r.py`/`matdyn.py`。
- [ ] kpath（vaspkit 303）集成与高对称点 label 写出。

## qf-io
- [ ] 读取 `scf.out/nscf.out/eband.out`（JOB DONE、能带、k 点、电荷）。
- [ ] 读取 `ph.out`/`dyn*`、`dos`/`elph` 输出，对齐 `io_reader`。

## qf-state
- [ ] 定义 `TaskState`/`WorkflowStatus`，三态（unstarted/finished/error）+ q 点 task 状态。
- [ ] 实现 `split_resume_step` 1–11 阶段判定，续算跳过 finished。

## qf-runner
- [ ] `Command` spawn 外部 QE/VASP，捕获 stdout/stderr 与退出码。
- [ ] Slurm 两段提交脚本生成（pre/post），1:1 对齐 `qe_phonon_split_*`。
- [ ] `ThreadPool` 并行读取（不并行 matplotlib/绘图）。

## qf-phonon
- [ ] split/merge 工作流编排，对齐 `split_workflow`/`split_merge`。
- [ ] dyn0 q 点映射、频率/模态后处理、联合图数据准备。

## qf-scflow
- [ ] a2f 重构（Allen-Dynes、Eliashberg 求解、Tc），数值对齐现有 Python `scflow`。
- [ ] `simpson` 积分、`ndarray`/`openblas-static` 线性代数。
- [ ] `plotters` 替代 matplotlib 输出 PNG/CSV。

## qf-cli
- [ ] `clap` 子命令定义，映射 `qe-scf/qe-nscf/qe-eband/qe-eledos/qe-phonon*/scflow`。
- [ ] 参数 `-i/-c/-r/-s` 行为保持兼容。
