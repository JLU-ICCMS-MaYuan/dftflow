# 新版 #49：超导独立展宽与参数目录

第 7 步已有有效细网格矩阵后，可在第 8 步按方法独立设置声子展宽。以下片段只展示本功能字段，其余原生参数沿用有效配置：

```toml
[workflow."8.epw-superconductivity"]
task = "gap"
mode = "both"
spectrum_index = 0

[stages."8.epw-superconductivity".iso]
degaussq = 0.5
fila2f = ""

[stages."8.epw-superconductivity".aniso]
degaussq = 2.0
fila2f = ""
```

`degaussq` 单位为 meV，优先采用当前方法第 8 步显式值；未填写时按 spectrum_index 回退第 7 步展宽。独立重算采用已有细矩阵，不执行第 7 步插值；不要同时指定非空 fila2f 读取旧谱。

```bash
epw_superconductivity -c "input.toml" --stage superconductivity -s
epw_superconductivity -c "input.toml" --status --json
# 原生任务结束后核验、归集及出图
epw_superconductivity -c "input.toml" --stage superconductivity -s
# 只有失败原因修复后才显式恢复剩余温度
epw_superconductivity -c "input.toml" --stage superconductivity --resume -s
```

例如 μ*=0.13、完整温度列表 20／30／40 K，上述配置形成两个独立方法作业及目录：

- `8.epw-superconductivity/iso-mu0.13-Tc20-40-degaussq0.5meV/`
- `8.epw-superconductivity/aniso-mu0.13-Tc20-40-degaussq2meV/`

数值采用普通十进制并去掉冗余零，故 2.0 显示为 2。`Tc` 是用户指定的目录前缀，不表示已经测定临界温度。恢复子批次仍保留完整计划首末温度的目录名。

改变本方法 degaussq 后再次调用，会创建相应目录并保留旧方法结果；新请求不会复用旧展宽的成功温度。旧格式目录不自动迁移，活动和未知作业仍禁止重复提交。细网格参数的嵌套分层目录不属于本轮范围。
