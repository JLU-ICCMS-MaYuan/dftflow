# 技术依据与决策

- `qeflow/input_deck/kpath.py` 从 KPATH 读取坐标，直接作为路径点；标签距离用这些坐标乘以 `2π * inv(lattice).T` 计算，约定是晶体分数坐标。
- `qeflow/input_deck/matdyn.py` 原样写入路径，却未声明 `q_in_cryst_coord`。
- 仓库 QE 源码 `external/q-e/PHonon/PH/matdyn.f90` 明确该参数默认 `.FALSE.`；为真时调用 `cryst_to_cart`。本次只读该源码，不修改外部 QE。
- SiP2 已有 ISIF4 的 matdyn 输入包含 `.true.`，与其路径约定一致；本次不改归档数据。

## 决策

固定写出 `.true.`，而不是添加切换选项：现有 KPATH 与距离逻辑已经共同约定分数坐标，增加选项会允许互相矛盾的配置。不转换成笛卡尔坐标，避免同时修改标签、晶格尺度和路径生成器。

## 环境

用户提供 `/home/mayuan/soft/qe-7.4.1/bin/`。两程序符号链接及可执行目标存在，当前终端缺少 MKL 三个动态库及 `libimf.so` 的搜索路径。仅给 `ldd` 子进程提供 `/home/mayuan/intel/oneapi/mkl/2025.0/lib` 和 `/home/mayuan/intel/oneapi/compiler/2025.0/lib` 后，两程序均无未找到的动态库。未修改当前 shell、持久环境变量或初始化脚本，未运行 QE；动态库解析通过不等于实算验收。本修复单元测试不依赖外部 QE。
