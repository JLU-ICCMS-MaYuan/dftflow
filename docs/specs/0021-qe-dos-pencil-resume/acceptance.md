# 验收记录

## 已确认的现场与实现

- Slurm 1307064：公开 -s 提交，bands 零退出码和完成产物；DOS 32 MPI 在读取保存数据时 fft_type_set 报空进程、退出 6，尚未进入 Wannier。
- 1307068：sacct 的 SubmitLine 为 sbatch submit.sh，尝试绑定 1307064，启动保护拒绝。不能将这次归因于公开命令创建了旧尝试。
- 原始证据在忽略目录 docs/local/0021-dos-pencil-resume；本地 QE 7.4.1 源码的公共命令行支持 -pd .true.，DOS 读取入口初始化 FFT。
- 生产修复只在 post_program 为 dos 添加已有 pencil 参数，bands 保持原值，PW／EPW／其他后处理不继承。执行、生成和收据参数共用该入口；不改变 CLI／TOML 或物理参数。
- 旧脚本作业号和哈希保护保留，冲突提示补充绑定／当前作业与公开恢复命令。

## 真实数据只读核验

PW 任务收据的 191 项产物和 bands 收据的 5 项产物，连同输入及来源身份全部核验通过，PW 保存树文件集合一致。收据本身哈希见 real-receipts.json。未改写真实清单、脚本或提交科学作业。

## 验证过程与限制

修复前连续进程测试明确捕获 DOS 缺失 pencil 参数；日志 before-fix-test.log。
修复后实际脚本已完成 DOS 失败→旧脚本拒绝→公开 -s 新尝试→复用 PW／bands→补 DOS→进入 Wannier。初次测试末尾误把 calls 元组中的 EPW 调用当作 PH，断言失败；检查实际日志与 succeeded 状态后，仅修正测试计数：所有 QE 调用不增加且 Wannier 新增一次。该初次日志保留 after-fix-test.log，不计通过。

完整回归、安装、最终文件哈希结果在完成后追加。协议测试不是真实 DOS／Wannier 科学验收；本次不代提交科学作业。线上 Gitee 和远程链接仍由用户管理，不推送。

## 最终回归

规范入口一次完整执行：

```bash
bash "scripts/build-native.sh" test --locked -p epwflow -p qeflow --config 'profile.dev.package.sha2.opt-level=3' -- --test-threads=4
```

退出码 0，209 通过、0 失败、7 默认 ignored（不算通过），31 组含零用例帮助／文档目标。新连续进程测试及原有资源变更、损坏文件、排队期间修改配置、活动任务防重复、脚本哈希、三种 PH 模式、质量门与显式 shell 失败传播均通过。完整日志 regression.log，机器摘要 regression-summary.json。没有在测试运行期间覆盖构建程序。

## 构建与安装

随后独立执行规范 build 入口，参数为 build --locked -p epwflow --bins --config 'profile.dev.package.sha2.opt-level=3'；源码是 429dbf7 加本需求，精确源码 SHA256、原构建和兼容处理后制品哈希分别记录于 build-manifest.json。兼容处理仅 strip 与本机原生运行库 interpreter／RUNPATH，三个 --help 通过后用 scripts/install-bin.sh --module epwflow 安装。

安装前 squeue 为空；只更新 bin/epwflow 三个程序，其他六个程序哈希及文件集合不变。安装后逐个 --help、实际配置 --stage wannier --check --json、--status --json 通过；source scripts/env.sh 后三个公开命令均解析到本仓库 bin/epwflow。证据见 install.log、installed-verification.json、installed-paths.txt、installed-check.json、installed-status.json。配置检查成功仅表示配置／路径／并行预检，不表示真实科学计算完成。

## 来源保护、文档及交付边界

1999 份计算与内部状态文件共 315882416 字节，文件集合、大小及 SHA256 与操作前一致。2000 份初始快照中唯一预期变更是 epwflow/README.md；不删除或改写已有原生输入、提交脚本、作业日志、清单或成功结果，未新增任何 Slurm 科学任务。对真实 PW 和 bands 的全部收据身份核验通过，修复后真实 DOS／Wannier 仍待用户运行，不宣称科学验收通过。

Overview 按已验证源码和进程证据回写；未提供 explore／understand Skill，使用 rg 与文件阅读完成等价只读分析。新 Spec 与当前手册的本地链接、完整原话、需求到任务覆盖核验通过。0020 原始需求中的历史日志链接保留原文，不改写成当前日志。

本地 0021 需求实现、回归、安装及文档完成；线上 Gitee、远程 Issue／Spec 双向链接由用户管理，保持未验证，不代关闭、不推送。按 AGENTS 要求只提交本任务的代码、测试和文档。
