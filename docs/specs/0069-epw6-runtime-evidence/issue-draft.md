# 修复 EPW 6.0 被硬编码 5.9 运行证据误拒绝

唯一类型 type:bug，本地暂定编号；线上由用户管理，未操作当前不匹配的 origin。关联 [Spec](spec.md)。

## 原始需求记录

> $ epw_wannier -c "input.toml" --stage wannier -s\
> 复用：1.qe-scf\
> 复用：2.qe-nscf\
> 复用：3.qe-electronic\
> reuse: electronic/projwfc\
> 诊断图：/home/mayuan/code/dftflow/examples/MgB2/ubuntu/run/5.epw-wannier/diagnostics\
> unknown: environment_incompatible: 缺少 EPW 5.9 运行证据

## 范围与文档影响

修正唯一运行版本证据，保留退出码、结束和原生错误检查；有界恢复因此误标失败且真实成功的当前 Wannier 结果，不重跑拟合，不自动提交新科学作业。不将输入标题中的 5.9 当运行版本，不宣称 EPW 6 的全部科学布局已验收。

- [x] 更新模块手册与当前 Overview
- [x] 实现、回归、覆盖构建和当前现场核验
- [ ] 线上状态／远程链接由用户管理

本地修复、重验和部署见 [验收](acceptance.md)，后续 DOS 由用户手动提交；未关闭线上 Issue 或声称远程链接完成。
