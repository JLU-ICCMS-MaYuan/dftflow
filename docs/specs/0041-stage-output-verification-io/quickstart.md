# 验证方法

使用规范构建入口执行 EPW 全量回归：

```bash
bash scripts/build-native.sh test --locked -p epwflow -- --test-threads=4
```

核对 output_verification_never_hides_damage_behind_missing_files_or_previous_calls 和 streaming_hash_keeps_full_content_and_non_aligned_tail_identity；前者覆盖不同分块中的缺失与损坏，后者覆盖多个完整缓冲区及非整块尾部。完整公开入口回归验证续算与保护没有改变。

规范构建安装后，从算例目录运行 epw_superconductivity -c epwinput.toml --stage fine -s，记录提交前等待、实际作业及上游哈希。只比较观测，不把共享盘波动归结成确定的提速比例。提交前核验仍完整读取科学数据，不能删除记录、放宽哈希或缓存上次通过来缩短等待。

并行任务的工作树修改不进入本项制品；独立副本源码和安装 SHA256 逐份记录，详见[验收记录](acceptance.md)。
