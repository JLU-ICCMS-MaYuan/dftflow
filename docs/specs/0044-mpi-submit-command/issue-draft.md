# 0044：统一 MPI 提交脚本的普通调用与环境路径解析

唯一类型：`type:maintenance`。本地编号已查重；origin 为 Gitee，沿用线上用户管理约定，不代表线上 Issue 已创建。关联 [0019](../0019-shell-script-options/issue-draft.md)、[0026](../0026-epw-direct-native-submit/issue-draft.md)，无新增 Epic／父子关系；本需求取代 0026 中一次 `exec` 调用的写法。

## 原始需求记录

完整逐字原文与两次范围选择见 [原始需求](original-request.md)，包括用户完整引用并要求实施的最终计划。

## 目标与范围

所有自有科学任务脚本使用普通调用；mpirun 在作业加载环境后从 PATH 查找。覆盖共享 QE、VASP、Wannier、EPW 和组合工作流生成器，部署到仓库分组安装与固定 current，迁移未冻结的现用脚本。保留并行参数、科学程序路径、环境与失败处理。

## 验收与关闭条件

- 实际生成脚本从环境脚本提供的 PATH 成功找到 mpirun，缺失或非零退出仍失败；跨模块回归通过。
- 安装来源为本任务提交的干净源码，安装制品指纹一致，安装版生成与执行隔离脚本通过。
- 现用脚本完成分类迁移，冻结配置、提交回执、科学数据及预算指纹不变。
- [规格](spec.md)、[计划](plan.md)、[任务](tasks.md)和[验收](acceptance.md)保持可追踪；线上链接及状态由用户管理。

## 范围外

不改变 mpiexec／srun、自定义启动器及用户 header，不修改构建脚本的 exec；不提交科学作业、不推送、不覆盖冻结证据。

## Documentation Impact

- [x] 更新 docs/overview/README.md 的共享约束及受影响模块入口。
- [ ] 完成 docs/specs/0044-mpi-submit-command/ 的实施与部署证据。
- [x] 更新适用配置示例和模块说明；根 README 仅需导航，无新增公共入口。
- [ ] 线上 Issue／Spec 双向链接待用户发布后核验，不能以本地链接替代。

## 确认与授权

用户完整引用最终计划并要求 `PLEASE IMPLEMENT THIS PLAN:`，明确授权本地 Issue／Spec、代码、测试、提交、安装与现用脚本迁移。按该版直接实施，不重复请求批准；不将未展示的文件全文声称为曾逐字审阅。
