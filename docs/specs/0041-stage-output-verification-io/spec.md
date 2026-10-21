# 需求规格

## 原始需求记录

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。

入口：[Issue](issue-draft.md)。无附件，images: []。

US1：用户提交已完成多阶段的工作区时，完整核验产物但减少串行等待。

- FR-001：阶段输出核验最多 8 路并发，每份文件仍经过原身份／哈希检查；流式哈希使用 1 MiB 堆缓冲，减少共享盘读取往返，并发缓冲不超过 8 MiB。
- FR-002：缺失返回不可复用，存在的损坏或读取错误仍报错；缺失不能掩盖其他损坏。严格 verify 遇缺失仍报错。线程失败不得视为成功。
- FR-003：核验结果只属于当前调用，不持久缓存；下次调用仍能发现刚发生的修改。不改变科学键、清单格式或原生调用。

SC-001：完整、混合缺失与损坏、再次调用篡改及原严格核验的回归通过。
SC-002：全量 EPW／公开入口通过；规范安装后实际继续本例，源数据哈希保持，记录实际限制而非承诺固定时限。
SC-003：文档和范围自动提交完整，不推送。
