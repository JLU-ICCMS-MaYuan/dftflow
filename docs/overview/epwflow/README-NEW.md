# EPW 工作流文档导航

**当前版本**：编号工作区（schema_version = 2）\
**最后更新**：2026-11-17

---

## 🚀 新手入门

- **5 分钟快速开始**：examples/MgB2/ubuntu/readme.md
- **完整流程图**：[workflow.md](workflow.md)
- **第一次运行前必读**：[preflight-and-reuse.md](preflight-and-reuse.md)

## 🏗️ 架构设计

- **[架构总览](architecture-overview.md)** ⭐ 必读：目录设计、数据流、状态管理、设计原则
- **阶段配置详解**：[native-stage-config.md](native-stage-config.md)
- **NSCF 网格生成**：[nscf-grid.md](nscf-grid.md)

## 📖 使用手册

### 三个主程序

| 程序                        | 负责阶段     | 主要功能                          |
|-----------------------------|--------------|-----------------------------------|
| `epw_phonon`                | 1-4          | SCF → NSCF → 电子 → 声子         |
| `epw_wannier`               | 5-6          | Wannier 拟合 → 粗网格 EPC        |
| `epw_superconductivity`     | 7-8          | 细网格 → 超导求解（Tc/Gap）       |

### 常用命令模式

```bash
# 查看状态
epw_phonon --status

# 检查配置（不提交）
epw_phonon --check

# 生成脚本并提交
epw_phonon --stage phonon -s

# 失败后恢复
epw_phonon --stage phonon --resume -s
```

### 专题文档

- **超导求解**：[native-superconductivity.md](native-superconductivity.md)
- **外部声子导入**：README.md § 外部原始声子接续
- **指定温度 Gap**：README.md § 指定温度超导能隙
- **历史清理**：[epw_clear.md](epw_clear.md)

## 📚 参考手册

- **CLI 完整参考**：README.md § 公开程序及选项
- **配置字段**：[native-stage-config.md](native-stage-config.md)
- **文件命名规范**：README.md § 科学输入与输出文件名
- **与 QE 复用关系**：README.md § 与 qeflow 的实际复用关系

## 🔧 开发与调试

- **源码组织**：README.md § 源码依据
- **测试目录**：../../../test_auto/MgB2/epwflow/
- **真实算例**：../../../examples/MgB2/

## 📜 历史与演进

- **验收记录**：../../specs/ 目录（70+ 个 spec 文档）
- **重要里程碑**：
  - 0018: 编号工作区布局
  - 0026: 原生任务直接提交
  - 0043: 常规对比最大差值
  - 0050: 阶段结果发布
  - 0058: 移除预算限制
  - 0064: 可读状态日志

- **已废弃特性**：
  - layout.json 工作区（由编号工作区取代）
  - 独立抽样参考（0043 移除）
  - 预算账本（0058 移除）
  - 旧配置嵌套格式（只接受 schema_version = 2）

---

## 常见问题

**Q: 旧工作区如何迁移？**\
A: 使用 `--migrate-state` 转换状态，或用 `--import-work` 导入科学数据

**Q: 如何判断某个阶段是否需要重算？**\
A: 查看阶段日志 `epw-state/<N>.<name>.log` 的 `input_fingerprint` 字段

**Q: 修改配置后哪些阶段会失效？**\
A: 依赖该参数的阶段及其下游会标记为 `needs_update`，自动重建

**Q: 如何只重绘图表不重算？**\
A: 使用 `--plot` 选项（如 `epw_wannier --plot`）

---

**提示**：文档中的"当前"/"最新"描述基于 2026-11-17 的源码状态。\
历史验收记录保留当时语境，不等于当前默认行为。
