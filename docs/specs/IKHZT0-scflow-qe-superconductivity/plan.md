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
