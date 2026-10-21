# 数据和接口

不新增持久字段或配置字段。沿用编号布局、Attempt、Completion 和电子原生任务收据。只调整 WannierOnly 原生输入的字段，不修改公开 CLI。旧失败尝试原样保留，新公开提交创建新 Attempt，成功 QE 身份继续按既有哈希验证。真实作业可在失败目录历史归档后追溯。

后处理补全的 `<prefix>.win` 和 `<prefix>.eig` 是同一已验证 NSCF 的派生输入，后者使用原始绝对 eV、原 k 序和排除后的连续带序。拟合原生 wout/chk/ukk 仍属于科学完成收据，后处理不能修改其身份。旧错误与图表原生产物归档到 `.epwflow/history/wannier-post-*`。后处理键内部版本更新为 2，科学键及持久结构不变。
