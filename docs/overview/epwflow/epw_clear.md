# epw_clear：清理历史计算

`epw_clear` 清空编号工作区的三处历史：`epw-state/history/`、`epw-state/native-attempts/history/`、`epw-state/native-jobs/history/`。保留现用计算、`results/`、现用任务记录及预算，不另建备份。

## 用法

在工作区根目录执行：

```bash
epw_clear --dry-run
epw_clear --yes
```

| 参数 | 行为 |
| --- | --- |
| `-w, --work 工作区` | 指定编号工作区根，默认当前目录 |
| `--dry-run` | 只读预览；不指定操作也默认预览 |
| `--yes` | 实际删除三处历史的全部内容，与 `--dry-run` 互斥 |

预览列出每处路径、目录数、文件／链接数和逻辑字节。逻辑字节不代表实际释放空间。已存在的 history 根目录保留，缺失的根不创建；重复执行正常完成。

## 输入与限制

只需工作区内有效的 `epw-state/layout.json`，不要求完整科学配置或启动科学程序。命令不接受任意删除路径或保留周期。历史内部链接只删除链接本身；清理根及其工作区内祖先为链接时拒绝。

删除前核查现用作业、队列工作目录以及现用 JSON／TOML／脚本和链接对历史的引用。活动作业、未知回执、无法核查的状态或现用引用会阻止全部清理，并指出原因。旧尝试快照不作为当前任务状态权威，仅当前 active 绑定的快照参与依赖检查。

清理与计算、生成输入和恢复命令顺序执行；预检及删除前重验不提供跨进程事务锁。删除失败返回非零并列出失败／剩余路径，不报告全部成功。

## 源码与验证

入口：[epw_clear.rs](../../../epwflow/src/bin/epw_clear.rs)；实现：[clear.rs](../../../epwflow/src/foundation/clear.rs)；公开 CLI 回归：[clear_history.rs](../../../epwflow/tests/clear_history.rs)。安装与真实清理证据见 [0057 验收](../../specs/0057-epw-clear-history/acceptance.md)。
