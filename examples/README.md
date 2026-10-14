# DFTFlow 算例索引

## 目录规则

项目自有展示算例统一放在 `examples/`，按 `examples/<体系或数据集>/<机器>/` 组织。跨机器共享材料可放在体系目录；不绑定机器的示例使用 `generic/`。各 flow 目录通过链接引用算例，不再维护独立的 examples 目录。

| 体系或数据集 | 机器 | 用途与入口 |
| --- | --- | --- |
| MgB₂ | sugon-amd | [QE 7.4.1 声子流程](MgB2/sugon-amd/README.md) |
| MgB₂ | inspur | [待配置的机器模板](MgB2/inspur/README.md) |
| MgB₂ | chem | [待配置的机器模板](MgB2/chem/README.md) |
| Si | generic | [原有结构与路径展示资产](Si/generic/README.md) |
| 合成 α²F | generic | [合成曲线参考材料](synthetic-a2F/generic/README.md) |

## 输入与输出

输入、结构、脚本和手册进入 Git；实际运行放在各机器目录的 `run/`，产物不进入 Git。机器真实路径、环境和登录信息只存 `docs/local/`。`test_auto/` 内测试夹具、冻结数值参考、`external/` 内第三方算例和本地依赖源码不属于展示算例迁移范围。
