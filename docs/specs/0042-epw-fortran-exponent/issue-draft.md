# 0042：超导原生输出省略 E 的 Fortran 指数解析失败

唯一类型：type:bug。本地编号已查重，线上 Gitee 用户管理，本编号不表示已发布。关联 [0027](../0027-mgb2-epwflow-example-debug/issue-draft.md)、[0040](../0040-interpolation-quality-warnings/issue-draft.md)。

## 原始需求记录

我想你能在/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow目录下进行全方位的 debug 和代码例子的运行。需要你用到的结构为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR.
输入的 toml 文件为：/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml

你可以修改/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow/epwinput.toml里面的内容。

如果有 bug 就新建 issue 和 spec 进行 debug。


## 已确认的故障与范围

真实作业 1316539 的 aniso 80 K 原生求解在 589 次迭代达到原容差并零退出；MgB2.imag_aniso_080.00 第 5 行有 1.7366215176-102。当前 Rust 标准浮点解析不接受省略 E 的三位指数，误报 data_invalid。本次只修复超导数值读取格式，不改数值、网格、收敛或物理参数。iso 10 K 原生 NaN／1000 次未收敛是另一真实求解结论，继续保留失败。

## 验收与文档影响

补充合法三位正负指数、标准 E/D、畸形／非有限／溢出拒绝，以及完整能隙文件解析。用实际原始请求、输出只读复验；安装后继续公开恢复，成功的十次 iso 线性求解不重做。更新 Spec／计划／任务／验收、Overview、模块及本例 README。线上发布／链接由用户管理，不推送。用户已授权遇 Bug 新建需求并修复，无需再次确认。
