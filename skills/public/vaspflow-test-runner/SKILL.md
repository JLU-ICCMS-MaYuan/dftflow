---
name: vaspflow-test-runner
description: 面向 vaspflow 仓库的 VASP 跑测与诊断技能，使用 mpirun -np 8 ~/soft/vasp.6.3.2/bin/vasp_std 执行 prepare/submit，提供一键命令、日志汇总与常见故障排查。触发场景：用户请求运行/验证/调试 vaspflow、prepare/submit、并行、日志收集等。
---

# 使用前提
- 仅限本仓库使用；默认路径：
  - 结构：`~/code/vaspflow/test/stdlibs`
  - 配置：`/home/mayuan/code/vaspflow/test/job_templates.toml`（勿直接用 `config/job_templates.toml`）
- VASP 命令：`mpirun -np 8 ~/soft/vasp.6.3.2/bin/vasp_std`
- 提交模式由配置 `[settings].submit` 控制；并行度由 `[tasks].max_workers` / `[settings].max_workers`。

# 快速跑测
1) 选择输入与配置：单结构文件或目录均可（目录自动批量）。必要时复制配置为局部文件再调整。
2) 使用脚本 `scripts/run_vaspflow.sh`：
   ```bash
   # 目录批量 + 自定义压强
   bash skills/public/vaspflow-test-runner/scripts/run_vaspflow.sh \
     --input ~/code/vaspflow/test/stdlibs \
     --config /home/mayuan/code/vaspflow/test/job_templates.toml \
     --pressures "0 5"

   # 单文件使用配置内压强
   bash skills/public/vaspflow-test-runner/scripts/run_vaspflow.sh \
     -i ~/code/vaspflow/test/stdlibs/Si.vasp
   ```
   - 默认工作目录为仓库根；`--pressures` 留空则沿用配置值；`--python` 可切换解释器。
3) 直接命令行（无脚本）：
   ```bash
   python cli.py -i ~/code/vaspflow/test/stdlibs --config /home/mayuan/code/vaspflow/test/job_templates.toml -p 0 5
   ```
   - prepare/submit 由配置决定；需提交时确保 `[settings].submit = true`，仅准备则置 `false`。

# 日志收集
- 跑完后使用 `scripts/collect_logs.py` 汇总文本：
  ```bash
  python skills/public/vaspflow-test-runner/scripts/collect_logs.py \
    --workdir /path/to/workdir \
    --output collected_logs.txt
  ```
- 汇总内容含 `pipeline.log`、`pipeline_report.txt`、`pipeline_checkpoint.json`、stderr/stdout（存在则截断到合理长度）。

# 常见故障提示
- POTCAR 缺失/路径错误：检查 `[potcar]` 段为绝对路径且元素完整。
- 配置引用模板：不得使用仓库内 `config/job_templates.toml`，需使用本地副本或测试路径。
- 结构未发现：确认输入路径与后缀（配置 `structure_ext` 或脚本 `--pressures` 为列表）。
- 提交脚本未生成/未执行：检查 `[settings].submit`、`mpi_cmd` 与可执行权限；并行度与结构数量匹配。
- 声子结构：`[phonon].structure` 仅支持 `primitive/conventional/relaxed`，缺少对应结构会报错。

# 报告模板
- 参见 `references/report-template.md`，可直接复制填写。

# 资源一览
- scripts/run_vaspflow.sh：一键运行 prepare/submit，支持自定义输入、配置、压强、Python 解释器。
- scripts/collect_logs.py：汇总常见输出日志为单一文本文件，便于诊断。
- references/report-template.md：跑测/诊断报告填充模板。
