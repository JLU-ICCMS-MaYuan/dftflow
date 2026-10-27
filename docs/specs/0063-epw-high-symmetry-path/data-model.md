# 数据模型

端点边包含两端标签、三维分数坐标、连续段和源行号；不存中间插值点。原生采样保存实际路径点、连续段、物理路径距离及原生能量，与 QE 原生参考点表独立。

`band-origin.json` 区分高对称端点路径和旧人工采样，绑定原生坐标、能量及标签表。`sampling.json` 的 mesh 继续记录参考网格身份，但新 `band.eig` 不含网格求值，读取者必须按 origin 判断，不能从 mesh 长度推断尾部本征值。

第 6 步 Sources 的可选电子比较来源保存四个 `SourceFile`：能带 CSV、DOS CSV、metrics JSON 和 sampling JSON。每份含路径与 SHA-256；无此字段的旧 Sources 原有语义不变。比较包 metadata 的独立电子采样标记决定是否允许两侧不同点数；每条曲线仍须递增、端点范围相同且具备双方方法。
