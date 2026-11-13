# 修复 MgB₂ 旧二进制赝势输入及无 accounting 数据库的 Slurm 续算

唯一类型 type:bug；0066 为本地暂定编号，沿用线上用户管理约定，不操作不匹配的 origin。

## 原始需求记录

> 帮我解决一下，我觉得老赝势应该也是可以读取的

## 范围与验收

采用 [Spec](spec.md) 的 FR-001～004／SC-001～004。根因是二进制旧 Vanderbilt 输入被当前文本读取分支误读，以及 accounting_storage/none 下未利用真实 jobcomp 完成记录。保留物理模型与原文件，安全恢复当前 12 MPI SCF。

## Documentation Impact

- [x] 更新当前 EPW 调度说明
- [x] 更新本目录验收、Ubuntu 输入和 README
- [ ] 未操作线上 Issue，远程链接待用户管理

本地修复与 SCF 验收完成，证据见 [验收](acceptance.md)；线上关闭状态与远程链接未操作。
