# 调查证据

1314095 的 CRASH：vmebloch2wan，You selected vme =.true. but error openingMgB2.bvec。EPW 5.9 bloch2wannier.f90 第 929～997 行先读 bvec 网格／近邻／向量权重，再读排除后带空间的 mmn 复矩阵；两者都必需，不能只补第一个文件。

实际源 bvec 为 32 k×8 邻居，mmn 为 36864 个复数＝12²×32×8。此前成功拟合收据的 76 项中没有这两项，不允许直接升级哈希伪装原始绑定。当前结构／网格／能窗保持；需一次重新生成完整第 5 步，已有参考另行保护。

原生 W90 plot.F90 第 92 行受 write_bvec 控制重写 bvec（含生成时间）；独立 restart=plot 需要显式关闭，保持已发布拟合配套身份。
