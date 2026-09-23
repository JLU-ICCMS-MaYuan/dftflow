# 实施计划

## 技术方案

1. 将 `external/a2f/src/*.py` 复制到 `scflow/`，加入 `__init__.py`，把顶层导入改为包内相对导入。
2. 新增配置数据类和 `scflow -c/--config` 入口；工作目录和结果目录按 `input.toml` 所在目录解析。
3. 增加 SciPy `simps/simpson` 兼容 helper、pymatgen CIF 文本写出兼容和 NumPy 标量 JSON 序列化。
4. 在 `pyproject.toml` 注册 console script 与 `scflow` extra。
5. 以公开 QE EPC 归档和真实用户归档分别做自动化与现场验收。

## 质量门

- 先运行 scflow 专项测试，再运行现有 qeflow/vaspflow 回归。
- 检查 `git diff --check`、包内导入和 pyproject 脚本注册。
- 真实归档验收只读输入目录，不执行 QE。

## ISIF4 真实归档修复与验证

1. 先用回归测试复现原始文件发现、零频除零和共享字典失败。
2. 文件发现收紧为明确 QE 文件模式，加入原生 elph_dir 和 split_ph.out.N。
3. 精确零频且 λ=0 时用有掩码的除法保持零贡献；非零 λ 拒绝；结果附原始模位置与 γ。
4. System 初始化实例结果字典；保持正频公式、平滑默认和输出字段兼容。
5. 原始目录直接运行两套 μ*，核验 20 份结果、所有图表、归档输入哈希、q 权重和正频贡献；更新 Issue/Spec。其他材料问题保留待办。

6. 真实输出与独立公式核对发现 float16 前因子误差，先补失败回归，再将 Tc 公式参数提升到 float64 并重新验收。
