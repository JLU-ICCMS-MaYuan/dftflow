# 需求规格

## 原始需求记录

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。


入口：[Issue](issue-draft.md)。无附件，images: []。

US1：用户希望合法原生 Fortran 数据可完整读入，不被误认为损坏。

- FR-001：支持带小数点尾数及显式正负三位指数、省略 E 的原生表示；保留标准 E/D 表示、符号、量纲及有限数检查。
- FR-002：畸形、星号、NaN／Inf、溢出仍拒绝；不把省略指数解释为减法，不把无效数填零。
- FR-003：共用超导数值入口覆盖日志与能隙；请求／温度／频率／状态映射、残差、上游哈希及求解分类保持。
- FR-004：真实只读核验原失败的 aniso 输出及来源；规范构建安装后公开恢复，原历史不改，已有有效子结果复用。

SC-001：格式正反向、能隙／日志及相关超导与公开续算回归通过。
SC-002：真实 aniso 80 K 输出可被原请求解析；不能将 iso NaN 未收敛改写成功。
SC-003：真实续算和预算分别登记，文档／安装身份完成，范围提交不推送。
