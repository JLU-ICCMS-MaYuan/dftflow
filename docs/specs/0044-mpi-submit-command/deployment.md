# 0044 部署记录

## 源码与构建

源码提交：`3523c1279763a811f4260c085b0bf80698992686`。使用 git archive 导出干净源码，不切换分支；255 个 Rust 源码及 Cargo 文件与已测试候选逐字一致。已有 workspace_science.rs 未提交修改未进入构建。

按既有 scripts/build-native.sh 流程构建全部工作区，使用本机原生依赖配置、dev 配置及 sha2 单包 opt-level=3；构建成功。随后运行 scripts/linux/package-native-runtime.sh，再运行 scripts/install-bin.sh 和 scripts/install-current.sh。打包保留原始构建和兼容处理后的两组 SHA256，安装前执行既有入口检查。

## 安装结果

- 仓库分组目录：/data/home/mayuan/code/dftflow/bin/<family>/。
- 固定安装目录：/data/home/mayuan/.local/share/dftflow/current/bin/。
- 命令入口：/data/home/mayuan/.local/bin/；54 个链接目标验证通过。
- 8 个程序的打包文件、分组安装及 current 文件 SHA256 完全相同。兼容打包修改加载器和运行库路径，因此与原始构建的指纹不同。

| 程序 | 已安装文件 SHA256 |
| --- | --- |
| qeflow | `083b5ae1451f097232a9043fc969d4892c54cea8e6083e62a8ef364de5e44c8a` |
| vaspflow | `48a676b502b74dbf368f22e6ff323a54420faf92ddaf02530f14ffb8760e7e4d` |
| scflow | `12b5edac53bad02381016abbcc7929c0f0e1c05fe6b7def6dc13731a5bf7108a` |
| wannier90 | `56bafeced71755b6df7bf5dff9ae6ba82b2f842cb108c6027f8098e90ef4db5a` |
| substitution | `73406c7c69aa6f371ad5a0032cfccd7d279c2d3b55133135f026d19ff9139167` |
| epw_phonon | `247b7ed14a782c75fecd09a704c6c75c4c08a5e53738cc01194ce1d30729964f` |
| epw_wannier | `a6c48bae115948449a3bc6b632820ad2c926c0f81b179477ecde49dedee4984d` |
| epw_superconductivity | `ba162277afb061ceb4129020ac72bfeb39ff8c7f7807031a98c5bd443e81f9c2` |

## 安装版隔离验证

两处安装各执行 QE、VASP、Wannier 的真实 CLI 生成与替身执行，共 6 组通过；每组检查无旧 MPI 路径／科学 exec、环境提供 mpirun、成功及非零失败。科学替身返回 17 时，QE／Wannier 保持 17，VASP 保持原有失败判断的非零结果 1。

EPW 安装版公开入口回归 8 项全部通过，覆盖完整隔离阶段链、环境与科学失败、旧 MPI 路径、重复提交和脚本身份保护。测试科学程序与 Slurm 都是替身，不提交真实作业。安装版测试期间仅临时将测试目标路径指向分组制品，结束后已恢复构建原件。原始证据只留本机忽略目录 docs/local/0044-mpi-submit-command/，包括 clean-source-verification.json、deploy-build.log、package-install.log、installed-hashes.json 和 installed-generator-checks.json。

## 迁移与保护

迁移计划只包含两个活动配置和 35 个未冻结脚本。已保存 189 份有效绑定文件的 SHA256。另识别到其他会话提交的 1318779 作业，其 temperature-points-002/submit.sh 不在迁移名单，另存指纹并保留原样；本任务没有提交该作业。首次迁移检查在队列非空时退出，未改文件；确认该作业结束后重新执行迁移。

历史编号实验目录、历史配置、回执及科学数据不参与脚本改写。迁移后核验通过：5094 个文件、827720410 字节纳入快照；5020 个文件比较 SHA256，其余 74 个比较 inode、大小和纳秒修改时间。37 个目标按计划改变，其他 5057 个文件无差异，预算账本包含在快照中。189 份有效绑定文件及上述新增作业脚本另做 SHA256 前后比对，全部一致。历史编号实验目录整体排除改写，未重新执行科学计算。37 份原件与计划原始哈希、迁移前快照逐一一致；结果内容与计划逐字相同。重复调用迁移时旧哈希检查拒绝已迁移文件，未发生重复改写。证据包括 migration-verification.json、migration-write-check.json、bound-verification.json、science-before.json。线上 Gitee Issue 仍由用户管理，本轮不创建、不关闭、不推送。
