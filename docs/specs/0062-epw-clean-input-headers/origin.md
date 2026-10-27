# 来源与责任追溯

## 用户要求与实现者自行扩展

用户原计划的要求是：“保留现有三个计算 CLI，生成参数对应记录，能从实际输入追溯到 TOML 或推导规则。”原文见 [0059 已确认计划第 22 行](../0059-epw-native-stage-config/approved-plan.md)。

该要求没有指定把记录写成科学输入头部的批量中文注释。AGENTS.md 的代码注释语言规则也没有要求往科学输入头部插入这类内容。

本助手在编写 [0059 参数契约](../0059-epw-native-stage-config/contracts/config.md) 时自行加入“实际 namelist 逐参数注释指向 TOML 路径或推导规则”（第 63 行），并按这个自行扩展的设计实现了前置注释。问题由本助手的实现选择造成，不能归责于用户要求或中文文档规则。

## 实际执行语句

首次引入的提交为 `bf60776d9f409132f348eef1c3824688c81df3b4`（0059）。以下行号固定指该提交：

1. `epwflow/src/foundation/native_config.rs:364` 把每个手填参数来源加入 provenance；375–377 行加入推导来源。
2. 同文件 404–410 行排序、去重后执行 `output.insert_str(first, &(provenance.join("\n") + "\n"));`，把全部来源说明插到第一个 namelist 之前。
3. `epwflow/src/qe_data/qe_config.rs:101–104` 另外构造中文卡片说明，再执行 `result.insert_str(0,&comments);`，形成截图最前一行。

因此不是 QE/EPW 软件自行添加，也不是用户下过写这些注释的命令；是本项目输入生成函数里的上述 Rust 语句。可复核原始实现：

```bash
git show bf60776:epwflow/src/foundation/native_config.rs
git show bf60776:epwflow/src/qe_data/qe_config.rs
git show bf60776:docs/specs/0059-epw-native-stage-config/contracts/config.md
```

0061 随后把来源路径改为平层，但保留了注释注入方式；0062 删除这两处注入，来源记录留在 JSON 中。

## 截图现场保护

只读核对的 `examples/SiP2/1.qe-scf/scf.in` SHA-256：`376bf9d25be5c28ce5d92b82d739c5a9de468adf3f311b0adae2824fc46546ec`。

`examples/SiP2/2.qe-nscf/nscf.in` SHA-256：`f4f7d8400edcd65a1aa1330e91c9b03fa9f6ee925150543fdedff4d82b906de5`。

这些输入已经与现场计算回执绑定，本项不删除或改写它们。新输入生成不再添加注释，不意味着历史文件被静默修改。
