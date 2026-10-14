# 独立 dyn0 生成工具

从 QE SCF `data-file-schema.xml` 读取倒格矢和对称性，用原有 Rust 算法生成不可约 q 网格。该目录可单独复制使用；不属于 dftflow workspace、不引用内部 crate、不由主项目安装器安装，也不参与声子自动工作流。

## 构建与运行

在本工具目录执行：

```bash
cargo build --release --locked
./target/release/qf-gen-dyn0 \
  --xml /计算目录/out/MgB2.save/data-file-schema.xml \
  --nq1 2 --nq2 2 --nq3 1 --prefix MgB2 --out-dir ./generated
cargo test --locked
```

`--xml`、`--prefix` 必填；三个网格维数默认 1，必须为正整数；输出目录默认当前目录。依赖已缓存时可添加 `--offline`。只需 Rust 和 Cargo 的纯 Rust 依赖，不需要 OpenBLAS、spglib、QE 可执行程序或 Slurm。

输出为 `MgB2.dyn0` 和 `q_points.json`；前者为 QE 笛卡尔坐标（2π/alat），后者同时保存晶体坐标、笛卡尔坐标和权重。请指定独立输出目录，避免覆盖计算已有数据。

本工具保留原算法及其适用范围，不宣称对任意磁性/对称性设置与 QE 完全等价。测试夹具来自项目冻结 MgB₂ SCF XML，预期输出在拆分前由原内嵌入口生成；用于证明迁移未改变算法和输出。DFTFlow 的 index/coords 工作流始终使用实际 QE 初始化生成的列表。

## 已单独安装时

若已将独立工具安装到 PATH，可以直接运行 `qf-gen-dyn0 --help`，或用 `qf-gen-dyn0` 替换上例中的 `./target/release/qf-gen-dyn0`。主项目安装器不会创建这个命令；独立安装后，升级主项目会保留它。

## 使用 QE 获取实际列表

已有完整 SCF 数据时，主项目提供：

```bash
qe_ph_nosplit -c input.toml --dyn0 -s
```

此命令调用 QE only_init 并提交初始化作业，不调用本工具。`qeflow gen-dyn0` 子命令已移除。
