# 契约：CLI、配置、状态与输出

> 本契约以冻结的 Python 参考版本及其 `pyproject.toml` 为行为来源；只描述本次迁移必须保留的外部可见行为。变更数值容差或命令语义须先更新 Spec，不允许测试失败后临时放宽。

## 1. CLI 与安装入口

- 当前 `[project.scripts]` 共 49 个命令：待迁移 48 个（`qe_*` 17、`vasp_*` 19、`scflow` 1、`wannier_*` 4、`substitution*` 7），保留 Python `mattersim_opt` 1 个。逐命令对照参数名、短选项、默认值、帮助、退出码、工作目录与生成文件。
- 交付物为五个家族静态二进制。通过同名符号链接或等效的薄入口让全部 48 个旧命令可直接调用；由二进制按入口名分派，旧 Slurm 脚本不必整体改名。`qf-gen-dyn0` 功能并入 `qeflow`，现有生成脚本所需别名继续可用，不单独交付第六个二进制。
- `substitution` 的可选 `mattersim_opt` 步骤保持现有命令调用和脚本/状态文件名；只有启用该步骤时需要目标机 Python 环境。

## 2. 输入配置

### 递归覆盖

- `public/config_templates.py:_merge_dict_in_place` 的契约：当且仅当 `base[key]` 与 `override[key]` 均为表时递归；`override[key]` 为 `None` 时不覆盖；其余值整体替换。按各 CLI 的允许键过滤后再合并。
- 配置保持现有实际块名与嵌套结构，包括 `[qe]`、`[qe_phonon.*]`、`[slurm]`、`[scflow]`、`[vasp]`、`[wannier90]`、`[wannier_init]`、`[substitutionflow]`、`[mattersim_opt]`；不得把它们概括为不存在的 `[wannier]` 或 `[substitution]` 并据此改写输入。
- QE `executable_path` 按 `resolve_qe_command` 语义处理启动前缀、bin 目录或已给出的具体 `pw.x/ph.x/q2r.x/matdyn.x`；VASP 的完整执行命令按现有脚本语义写出，不擅自拆分或改写重定向。

## 3. 状态与执行

- `workflow_status.json`、`task_status.json`、substitution 的 `case_status.json` 等名称和字段由 Python 参考样例冻结。`unstarted/finished/error`、q 点 task、1–11 续算、跳过 finished、失败后可恢复的行为保持一致。
- QE phonon 的 pre 段为 scf→dyn0→split 提交；post 段为 merge→q2r→matdyn→收集→plot。保留当前 shell/Slurm 脚本名、步骤顺序、退出行为和计算节点可找到的命令入口。
- Rust 只编排外部 QE/VASP/LOBSTER/vaspkit；这些程序的可用性属于独立部署前提。可选 MatterSim 链路是登记的 Python 例外。

## 4. 输出与数值比较

- **精确项**：文件存在性和路径、文件名/扩展名、文本列名与顺序、单位、行数、状态枚举、求解器状态、非数值 JSON 字段、命令退出码及必要的输入卡字段。文本格式可因绘图库不同而变化时，以解析后的可观察语义为准。
- **浮点项**：在固定输入和参考版本上，解析 CSV/DAT/JSON 数值后判定 `|new-ref| <= A + 1e-6 × |ref|`。有确定小数位的字段令 `A = max(1e-8, 0.5 × 末位单位)`；未格式化字段令 `A = 1e-8`。记录每个字段的参考值、误差、容差和通过状态；NaN、Inf、缺失值与有效数字不得互相视为相等。
- `scflow` 的精确零频及原始 γ 记录、Eliashberg `resolved/resolution_limit` 与下限值单独验收；触及搜索下限的输出不能报告为已求出的物理 Tc。浮点容差不覆盖这些状态判定。
- 现有图形格式须保留：phonon 联合 JPG、各类 PNG、scflow PDF。`plotters` 用于绘制；PDF 由 SVG 后端配合纯 Rust `svg2pdf` 转换。验收曲线、图例、坐标轴、范围、标注、面板布局和图片可读取性，不要求像素级一致。
- `spglib` 只计算对称性；Rust CIF 序列化须另行生成与 Python 等价的晶格、原子位置及空间群信息。COHPCAR.lobster 解析须保留 COHP/COBI 数值与标签语义。

## 5. 参考基线与 Python 例外

- P0 将当前被忽略的 11 个 `test_auto/MgB2/` pytest 脚本纳入 Git，冻结参考提交、Python 依赖、样例哈希和外部 ISIF4 资料来源；在独立参考工作树跑 pytest，在 Rust 工作树跑集成测试。两者均须通过，不能互相替代。
- 最终 `pyproject.toml` 保留 `mattersim_opt` 所需 Python 元数据、包发现和依赖；`mattersimflow/` 只为解除 `public` 依赖作等价调整。`test_auto/` 作为验收资产保留，最终是否删除由用户另行决定。
