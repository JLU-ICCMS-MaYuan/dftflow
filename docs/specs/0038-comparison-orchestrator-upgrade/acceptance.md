# 验收记录

实施中。原生 EPC 已完成；质量失败与路径后处理故障分别记录，不宣称细网格或 Tc 通过。证据留 docs/local/0027-mgb2-epwflow-example-debug/。

## 最终源码验证

规范入口完整 EPW 回归 198 通过、0 失败、7 项专门真实数据用例暂挂不计通过。覆盖原生固定宽度星号占位、普通权重严格检查、坐标／频率／维度损坏拒绝，以及实际 ELF 原路径覆盖后恢复、矩阵损坏拒绝、成功 EPC 收据／原请求不变、PW／PH 不重复。公共程序返回成功并继续原生比较；初次路径文字过严检查及预期 q2r／matdyn 重试的测试断言修正过程见 research，失败版本未安装。最终规范构建、安装和真实数值另行记录。

规范 scripts/build-native.sh build --bins 成功，随后通过 scripts/install-bin.sh --module epwflow 更新 bin/epwflow。三程序 --help、实际本例配置 --check --json 及源／安装身份核验通过，其余六份产品哈希未变；安装前队列为空，未覆盖运行中作业。build-release.log、install-release-verification.log 和完整 source/program SHA256 保存在本地。

真实公开恢复已完成：1314972 在 24 节点秒内复用原生粗 EPC 和比较 PW／matdyn，生成 15 图；24 份成功原生／请求证据保持。作业当时只因旧数值质量门退出；该门随后由用户新需求 0040 改为告警，不属于本缺陷修改。
