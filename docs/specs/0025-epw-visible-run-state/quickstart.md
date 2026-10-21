# 验证与使用

以下行为和新增参数适用于本项源码构建的新版本；本轮未更新默认安装。新工作区保持原命令。编号阶段完成记录位于本阶段 `completion.json`，共享状态位于 `epw-state/`。

旧编号工作区先执行 `epw_wannier --migrate-state`，核对输出的复用/重建阶段，再执行 `epw_wannier --stage wannier -s`。旧工作区有活动任务或未知状态时先查清原作业；不得靠删除目录绕过提交保护。迁移保留原隐藏目录，不复用旧冻结脚本；后续使用公开命令生成新脚本。

删除整个阶段目录后，正常提交会重建该阶段，保留相容上游；单个必要文件缺失同样需要重建。内容被修改或记录损坏仍报错。迁移期间不提交作业、不改变预算或科学参数。

本机验证使用 `CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p epwflow --no-fail-fast`，并执行共享 qeflow 回归；进程替身仅证明目录、复用与执行协议，不是科学验收。
