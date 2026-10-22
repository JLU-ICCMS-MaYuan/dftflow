# 数据契约：QE 单独图

## 图像来源与归属

- 电子能带：本叶任务原生 XML、本次路径点／KPATH、NSCF XML 的 Fermi；归属 `3.qe-electronic/qe_eband/plots/`。
- 电子 DOS：原生 `prefix.tdos` 与 NSCF XML 的 Fermi；归属 `3.qe-electronic/qe_eledos/plots/`。
- 声子图：本阶段原生 `prefix.phband.freq`、路径标签、`prefix.phdos.dat` 及结构；归属相应 `4.qe-phonon-*/plots/`。

## 派生图清单

使用本地 `plots/completion.json` 记录来源及渲染参数生成的 key 和图片／表格 Identity。Identity 保留路径、大小和 SHA256，遵循已有 store 检查。图像缺失或来源变更时重新渲染；来源科学数据仍由原有回执先核验。清单写入最后进行，失败不得把残缺图标为完成。该清单不参与下游科学数据身份。

## 声子数值后处理检查点

阶段内 `qe-post-completion.json` 保存 q2r/matdyn 对应输入与数值输出身份。来源相容且产物可核验时可跳过成功数值后处理，仅重试绘图；缺失或损坏不能静默视为成功。正常阶段 completion 仍按既有科学证据生成，排除 plots 与派生清单。

## 校验与生命周期

电子能量有限、点数与路径一致、跳段断开；DOS 频率／能量轴递增。声子原子投影列数必须等于结构原子数；同种原子贡献汇总为元素。新图生成不得修改原生文件。后续叠图读取两侧真实数据，声子只显示 TDOS，不需要元素投影适配。
