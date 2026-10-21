# 0041：阶段产物串行核验放大共享盘延迟

唯一类型：`type:bug`。本地编号已查重，线上 Gitee 用户管理，非已发布线上编号。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0040](../0040-interpolation-quality-warnings/issue-draft.md)。

## 原始需求记录

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。

## 真实问题与范围

0040 的全量 200 项回归通过后，实际第 7 步在 sbatch 前已等待超过 17 分钟。跟踪显示共享盘小文件 read 约 0.1～0.3 秒，CPU 时间仅数秒；stages::Completion 的 available／verify 逐个串行读取全部产物，多个阶段检查进一步放大延迟。等待没有执行新 QE 或消耗节点预算。该优化不声称修复共享盘本身。

仅将同一阶段互相独立的产物核验改为有界并发，最多 8 个工作线程；保留每份身份、哈希、链接目标和缺失／损坏区别。将原 64 KiB 流式读取缓冲改为 1 MiB 堆缓冲，保持 SHA256 完整性与有界内存。调用之间不缓存结果，不跳过数据检查，不修改科学指纹或资源。不创建后台常驻线程池或新依赖。

## 验收与文档影响

- 对完整清单、缺失和损坏混合清单验证结果一致；缺失不能掩盖另一文件损坏。
- 公开三程序／续算全回归通过；安装后继续真实测试，原成功数据不变。
- 更新 [Spec](spec.md)、计划、任务、验收、Overview／模块及算例说明；线上链接待用户发布，不推送。

用户已授权全方位 debug、发现 Bug 新建需求并实施，不再征求确认。
