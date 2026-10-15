# 0009 验收记录

## 原因与修复

旧绘图直接按采样点行号索引端点表，超过 12 条记录后 171 个点横坐标回零；新 BandData 为全部 183 点插值并校验，图/CSV 共用。独立谱图与联合图共用绘制器，负频完整显示；跳段不连线，A|L/M|H 合并标签，Γ 使用正常轴刻度位置。联合图 DOS 横轴为 DOS、纵轴为频率，左右范围/上下像素边界一致，分波按元素聚合。

## 真实数据与 Python 对照

固定 origin/python 提交 7a172fed4b48c408bc7a8d509071254a225ae637。导出原 kpath/matdyn reader/tables/plot，独立调用未改原算法；仅用类型占位避开无关编排导入。

- 183 个样本 × 9 模式逐点比较：全部频率最大差为 0 cm⁻¹；CSV 距离最大差约 4.932×10⁻⁷（六位小数舍入），分数坐标最大差约 3.365×10⁻⁹（八位小数舍入）。
- DOS 逐原子/逐元素 CSV 与 Python 在六位小数精度内一致。
- Python 从 VASPKit KPATH.in 重新生成端点标签，索引为 1/21/41/61/81/101/121/141/142/162/163/183，距离与现有 Rust 标签在 10⁻⁹ 内一致，跳段前端点权重为 1。
- POSCAR 与 VASPKit PRIMCELL 晶格最大差 3.137×10⁻¹¹ Å，基底一致；不自动替换任何结构。一般标准化原胞不保证同基底，应由用户提供与计算晶胞一致的路径，本次不臆造通用坐标变换。
- 实际读取并查看旧 PNG、新 PNG、新联合 JPG 和 Python 联合 JPG：新图曲线与 Python 一致，断段无多余竖直连线，端点标签显示在横轴下方，DOS 共享纵轴且正常显示 TDOS/Mg/B。不以图片文件存在代替视觉核验。
- 隔离副本执行真实 qe-process-phonon --plot；原运行目录输入、旧图等快照哈希前后一致。未运行 pw/ph/q2r/matdyn、未提交 Slurm。

机器路径、参考源码、原图/新图、逐点差异及哈希保存在忽略目录 docs/local/0009-phonon-plot/，comparison.json 为数值证据。

## 自动验证与安装

执行 `bash scripts/build-native.sh test --release --locked --offline -p qf-phonon -p qf-input -p qeflow`。37 项 release 测试通过，覆盖全部 qeflow 测试、qf-input 的非立方路径/跳段权重测试和 qf-phonon 表格/绘图数据测试。本地最终日志 docs/local/0009-tests-final.log。最终 CLI 回归使用 selected.vasp 且工作目录无 POSCAR，六点跳段/负频、全部 CSV 点、三张图及非法索引失败验证通过。

额外人工查看合成负频联合图：左谱低至 -5 cm⁻¹、右 DOS 低至 -10 cm⁻¹，完整显示、零频像素对齐，跳段不连线。真实输入的 Python/Rust matdyn_phband.in 逐字节一致。

本机安装仅更新 qeflow，其他四二进制复用前版；既有私有 glibc 加载器打包，源二进制及打包制品分别哈希，旧版与入口备份保留。51 入口帮助及已安装 qe_process_phonon 隔离重绘通过，PNG/JPG/CSV 与原生构建逐字节一致，来源见本地 install.json。最终 source_commit 在提交后回填，本地机器路径不入库。

SC-001～004 全部满足；不将本地草稿冒充线上 Issue 已创建/关闭。所有基线、新旧图及安装验收仅在本地证据目录，用户原始计算目录未改写。

## 2026-10-15 追加验收

- 命令：`bash scripts/build-native.sh test --release --locked --offline -p qf-config -p qf-input -p qf-phonon -p qeflow`；最终 43 项通过，日志 `docs/local/phonon-additions/tests-verified.log`。旧 scflow 编译警告未在本次修改范围。
- 安装已验证 qeflow，沿用私有 glibc 加载器并保留旧版本；其他四个程序原样复制。51 默认入口帮助、三聚合入口模拟 sbatch 提交与快照脚本检查通过。安装版绘图与原生测试制品哈希一致，源码和制品哈希见 `docs/local/phonon-additions/install.json`。
- 不执行真实 QE、不提交真实 Slurm、不迁移现有运行目录；线上 Issue 仍由用户管理，远程双向链接不冒充完成。
- FR-007 / SC-005：生产输入生成器验证 PH [2,3,1]→DOS [4,6,2]、nq 三键形式、显式 qpoints 和 nk 三键优先；本地用户 input.toml 已显式改成 [4,4,2]，不会因安装自动重算。
- FR-008 / SC-005：同一归一高斯核作用于所有列；回归验证积分误差 <1e-10、投影加和误差 <1e-12、加宽峰值下降、0 关闭、负数/NaN/重复频率拒绝。真实 CLI 验证平滑图片改变但 CSV 不变。
- MgB₂ 原数据副本实际重绘 σ=0/10/20 并查看三图并排，σ=10 减弱尖峰，σ=20 更平滑且细峰开始合并；所有原始数据哈希不变，各宽度 CSV 相同。对比图 `docs/local/phonon-additions/dos-comparison.png`，记录 redraw.json。
