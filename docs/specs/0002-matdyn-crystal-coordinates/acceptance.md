# 本地验收记录

日期：2026-09-22；Issue：0002（Gitee：[IKHSA0](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKHSA0)）。

## 测试先行

- Python 3.12.2；命令见 [quickstart.md](quickstart.md)。
- 修复前新增测试：**4 failed, 1 passed**；四个非立方路径用例均因 namelist 缺少 `q_in_cryst_coord = .true.` 失败。DOS 输入契约已通过。
- 修复后：新增 5 项 + 已有 54 项声子/联合图测试，**59 passed**。
- 正交不等轴和斜晶胞均覆盖插值开关两种值；断言声明恰好一次、分数坐标、权重、标签、索引及解析倒格子距离。不使用 mock 替代路径生成器。
- 对不开启 band form 的用例，仅确认坐标声明和原输出兼容，不扩展其已有标签/采样语义。
- DOS 输入逐字节期望不变；路径输入文件不变。

## 变更边界

- 生产代码仅 `qeflow/input_deck/matdyn.py` 新增一行晶体坐标声明。
- 不修改模板、CLI、路径转换、绘图、DOS writer、状态机或计算归档；不新增依赖。
- 使用临时测试目录；未对 ISIF2/ISIF3 数据或 PPT 写入，也未处理 SC-07。
- 更新 qeflow README、Overview、项目 skill 与 AGENTS。根 README 操作入口不变，无需调整。
- Spec/Plan/Tasks 覆盖与文档相对链接检查、Python 语法及 git diff 空白检查作为提交前质量门。

## 外部 QE 与发布边界

- 用户提供的 q2r.x、matdyn.x 均存在且可执行；默认终端的动态库解析缺 MKL/libimf。
- 仅给 ldd 子进程补充本机 MKL/compiler 库路径后，依赖解析无缺失；未改持久环境，不等于程序实算成功。
- 未启动 q2r/matdyn，未验收真实新算频率；真实后处理仍需独立输入核验与执行。
- 历史错误路径结果需核查后另行重算，不能靠重绘修正；既有正确归档的绘图不受影响。
- 仅本地提交，不推送。Issue 保持开放，提交号回写评论；远端 Spec 链接发布前不可访问。
