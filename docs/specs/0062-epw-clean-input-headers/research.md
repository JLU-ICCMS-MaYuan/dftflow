# 0062 调查

全部“! 参数来源”注入来自两处：共用 native_config::apply_impl 与 QeSettings::pw_input。科学生成程序自身无需这些行；输出文件的身份是字节哈希，因此不能直接修改与回执绑定的历史文件。只修改生成器后，新的执行自然生成无来源头的输入。

现有 configure_file 已写独立输入参数回执，提交快照已有 parameter-map.json。保留这两种现有记录即可满足 0059 可追溯要求，无需另建注释机制。0061 已把对外位置改为平层，本项继续使用该映射。

真实只准备路径检查发现：已有参数对应 JSON 仅在冻结提交路径写入，直接准备模式尚缺独立记录。本项在直接模式固定 current-config.toml 后复用 write_parameter_map，输出 epw-state/parameter-map.json；输入哈希继续由原生任务回执绑定，不增加另一套参数模型。
