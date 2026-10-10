# 调研与技术决策：dftflow 全量 Rust 重写

> 本文件记录已确认选择与仍需 P0 实证的技术风险。用户原话见 `spec.md`；本文件不改写原始记录。

## 架构与工程布局

- 借鉴 FireWorks 的状态机、队列适配和 Workflow DAG，使用文件系统状态文件，不引入数据库或 daemon。
- 迁移期间根 Cargo workspace 收录 `crates/*` 与 `qeflow-rust/` 等过渡目录；Python `qeflow/` 已存在，P0 不能直接把 Rust 目录改为同名。全部五个 flow 经 P2–P4 对照后，在 P5 反向依赖清零时集中切换目录。
- 五个交付二进制为 `qeflow`、`vaspflow`、`scflow`、`wannier90`、`substitution`。`pyproject.toml` 当前有 48 个待迁移公开命令与 1 个保留的 `mattersim_opt`；兼容入口不复制 48 份二进制。现有 `qf-gen-dyn0` 功能并入 `qeflow` 并保留所需调用别名。

## 数据、数值与科学库

- `qf-config::merge_recursive` 复刻 `public/config_templates.py:_merge_dict_in_place`：双方均为表才递归，override 为 `None` 不覆盖，其余整体替换。
- 文件名、列、单位、排序、状态、退出码及输入卡语义精确一致；浮点 CSV/DAT/JSON 使用契约中事前写定的字段容差。零频、原始 γ、Eliashberg 下限状态分别验收，不把搜索下限当成有效物理 Tc。
- 用户选择静态 OpenBLAS：`ndarray-linalg` 处理 Eliashberg 矩阵本征值。当前 `qf-scflow` 切片的 Jacobi 是试验实现，不能代替已确认的交付方案；P0 先用真实矩阵验证 OpenBLAS 的 musl 静态链接与离线运行，失败则停止并修订方案，不擅自换库。
- `spglib` 仅提供对称性识别与等价位置；Rust 仍需自行写出符合现有结果的 CIF 文本及 POSCAR。COHPCAR.lobster 由 Rust 手写解析，覆盖 COHP/COBI、标签、能量及异常输入。

## 绘图与文件形态

- Python 现有输出不只有 PNG：声子联合图为 JPG，scflow 使用 PDF。`plotters` 用于图形内容；PNG/JPG 走位图输出，PDF 使用 SVG 后端加纯 Rust `svg2pdf` 转换。P0 必须验证可读性、字体、坐标轴及 Linux 静态构建。
- 内容与布局等价而非像素级相同；文件名、扩展名、图层、曲线、标注和坐标范围不能丢失。CSV/DAT 数值按预定容差，不再写“字节级 1:1”作为所有浮点产物的验收条件。

## Python 例外与跨包依赖

- `mattersim_opt` 保留 Python CLI，继续生成调用 ASE/MatterSim 的 Python 脚本。用户允许只为去除 `public` 依赖调整其 helper/import；启用 substitution 的可选 ML 步骤时，目标机仍需原有 Python 环境。不启用该步骤的五个 Rust flow 无 Python 运行时依赖。
- 现有 Python `substitutionflow/prepare_qe_pseudo.py` 导入 `qeflow.common`，Wannier、substitution、MatterSim 等导入 `public`；因此 P2/P3 完成后仍保留旧 Python 包，P5 才能统一删改。
- `pyproject.toml` 最终保留 `mattersim_opt` 的最小 Python 安装内容，不可整段删除。外部 QE/VASP/LOBSTER/vaspkit 仍是独立运行依赖，不由 Rust 静态二进制内嵌。

## 测试、构建与 Issue

- `test_auto/MgB2/` 的 11 个测试脚本当前被 `.gitignore` 忽略，未形成 Git 可复现黄金基线；P0 必须跟踪它们、冻结 Python 参考提交、环境与输入哈希。文档原指向的 `validate_scflow_isif4.py` 不存在，ISIF4 对照须绑定可取得的真实验收资料。
- 发布目标为 `x86_64-unknown-linux-musl` 静态链接并 strip。当前 musl 骨架不能证明未来的 OpenBLAS、spglib、PDF 全部可静态交付；P0 是阻断门。Darwin 只承担开发构建。
- 用户选择性能只记录、不设通过门槛，因此重写不能仅凭本计划宣称达成量化提速。
- 0004 作为 Epic，P0–P5 是待编号子 Issue 草稿；线上编号、标签与双向链接由用户建立后回填。P1–P4 的正式 Feature Spec 使用真实 Issue number；本目录暂保存跨阶段资料。
