# qf-gen-dyn0：独立 Rust 不可约 q 网格工具

[返回家族索引](README.md)

## 功能与入口

从 SCF XML 读取倒格矢和对称性，用保留的 Rust 算法生成 dyn0，不启动 QE。

等价入口：`独立 qf-gen-dyn0 可执行文件`。

## 使用方法

```bash
qf-gen-dyn0 --xml /path/to/out/MgB2.save/data-file-schema.xml --nq1 2 --nq2 2 --nq3 1 --prefix MgB2 --out-dir ./generated-dyn0
```

--xml、--prefix 必填；--nq1/2/3 默认各 1，须为正整数；--out-dir 默认当前目录，建议独立目录。

## 输入、输出与衔接

生成 MgB2.dyn0（QE 笛卡尔坐标，2π/alat）和含晶体坐标、笛卡尔坐标及权重的 q_points.json。

## 使用边界

不属于主 workspace、不由主安装器安装、不参与 index/coords 自动链。不保证任意磁性/对称性设置下与 QE 等价。主程序 qeflow gen-dyn0 已移除；生产链使用 QE only_init 列表。

## 实现依据与关联

[独立工具说明](../../../tools/qf-gen-dyn0/README.md)；[源码](../../../tools/qf-gen-dyn0/src/main.rs)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0006-qe-phonon-submit-chain/spec.md)。

## 单独构建与使用

在仓库根目录执行（本次未执行构建）：

```bash
cargo build --manifest-path tools/qf-gen-dyn0/Cargo.toml --release --locked
tools/qf-gen-dyn0/target/release/qf-gen-dyn0 --help
```

依赖已缓存时可加 --offline；无需 OpenBLAS、spglib、QE 或 Slurm。未独立安装时，用上述可执行文件路径代替命令名。要获取实际 QE 列表，请用 [qe_ph_nosplit --dyn0](../qeflow/qe_ph_nosplit.md)。
