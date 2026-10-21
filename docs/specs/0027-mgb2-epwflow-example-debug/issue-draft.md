# 0027：MgB₂ 指定 EPW 算例入口修复与真实链路 debug

唯一类型：`type:bug`。本地编号已查重，线上 Gitee 由用户管理，未创建线上 Issue。

## 原始需求记录

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。

用户随后调整测试范围：

> 这7 项你不光不用测，你可以把这 7 项都从我的 spec 和issue 里面删除了。

## 问题与范围

指定目录中的 epwinput.toml 仍写 structure=POSCAR，但该文件不存在，真实结构在上一级；README 仍引导上一级旧入口，与新授权位置相反。0025 新源码尚未安装，不能把源码回归当作现场已修复。修正指定配置／文档，构建安装当前代码，使用三个公开程序实际运行并查验完整科学链；出现后续代码缺陷时按证据关联独立 Bug，不伪造成功。

关联 [0015 真实验证](../0015-epwflow-mgb2-real-validation/issue-draft.md)、[0023 原生 Wannier](../0023-epw-wannier-native-start/issue-draft.md)、[0025 可见状态](../0025-epw-visible-run-state/issue-draft.md)。0026 纯原生脚本需求仍按其已有审查状态，不冒充已经实现。

## 已删除的验收范围

已从本项及关联 0010／0012／0013／0015／0020／0040 的测试、任务和验收条件中删除[七项历史专项](spec.md#已移除的专项验收)，不执行、不补跑，不以七项跳过作为待完成或关闭阻塞。历史证据保留事实含义；当前工作区实际失败仍如实记录。本次只修改本地 Issue／Spec，线上 Gitee 仍由用户管理。

## 验收与文档影响

见 [Spec](spec.md)、[计划](plan.md)、[任务](tasks.md) 和 [验收](acceptance.md)。更新本目录 README、相关 Overview 及发现缺陷的模块文档。保留历史来源、共享预算和固定质量门；不推送、不操作线上 Issue，不在旧测试目录安装二进制。
