# 电子图与投影数据契约

## 文件与显示

第 3 步 `qe_eband/plots` 与第 5 步 `comparison-native/electron` 各保存 `electronic-bands.png`（第 3 步近费米，第 5 步实际拟合外窗）和 `electronic-bands-valence.png`（完整价带）；第 3 步 `qe_eledos/plots` 和第 5 步电子比较目录各保存同名规则的 `electronic-dos*.png`。每张都有对应 `.plot.json`；完整 CSV 不随显示裁剪删行。

`electronic-display.json` 保存近费米／完整价带边界及阶段默认边界、最低占据能量、费米能与 NSCF XML 身份、3000×2100 像素、PNG 密度与配色。PNG 默认 10×7 英寸，密度按 PNG 像素／米规则向上量化为 300.0248 DPI。

能带 y=0、DOS x=0 是红色费米线；蓝色只用于插值。QE 深灰，元素和窗口从绿、橙、褐、紫等颜色中选择，文字默认黑色。窗口超出显示范围仍列出真实值，不映射到画布外。插值 DOS 不覆盖的低能区保持缺失，图下注明两侧实际 DOS 支持范围。

## 原生投影与恢复

`qe_eledos/pdos/` 是 NSCF 波函数的独立副本及必要后处理工作区；不得写入第 2 步或原 TDOS 保存树。

- `decision.json`：必要后处理原因、程序、源身份、实际输入和禁止重算的科学阶段。
- `request.json`：科学源、原生程序内容／参数及完整投影输入的 key；绘图尺寸／配色不在该 key 中。
- `native-checkpoint.json`：成功执行后的原始数值／日志／输入身份；在投影解析前保存。
- `completion.json`：所有原子／原子波函数、能轴及标量格式核验后记录的相容投影。

已核验旧投影遇到展宽变化时保留在历史目录，仅进行新的投影后处理；损坏的已登记数据拒绝。原生检查点仍完整时，派生收据丢失只重新核验，不重复程序调用。

原生逐原子文件使用 `E, LDOS, m 分量…`，每个 atom/wfc 的 LDOS 只加一次。原生日志中每个 atom/wfc 必须有对应文件，所有结构原子均覆盖；不把投影和强行缩放到 TDOS。`electronic-pdos.csv` 保存 TDOS、投影和、差值以及各元素；`electronic-pdos.json` 标记完整性、汇总规则、自旋边界、展宽和来源。

QE 的 projwfc 会将能轴裁剪到本征值附近，即使输入相同 Emin／Emax，也不能假定与旧 dos.x 网格相同。完整元素图读取同次 `.pdos_tot` 中的 **DOS 列**作为 TDOS，不使用该文件的投影和列冒充 TDOS，不通过插值或缩放凑齐能轴；旧 dos.x 文件原样保留并记录身份。第 5 步仍按原共同均匀 k 网格／高斯核比较 TDOS，不以第 3 步投影表替换其科学指标。

## 独立重绘

根 `results` 包含完整 CSV、所有 `.plot.json`、显示和投影元数据及阶段清单。重绘只读取包内曲线，不读取原子波函数、不执行科学程序。旧包无元素数据时明确标注 TDOS-only incomplete，不能伪装成新增的完整元素交付。

共享电子绘图模块内置未修改的 DejaVu Sans 及其许可文本，PNG／比较 PDF 不依赖计算节点的系统字体；SVG 仍可作为矢量文本由阅读器显示。

## 重合曲线的显示样式

所有 QE 参考／TDOS 均为连续实线（多曲线图线宽 4，单曲线图线宽 2）；插值最后覆盖：蓝色短虚线 6／6、线宽 3。第 3 步元素 PDOS 依既有颜色次序使用不同短虚线，避免完全遮挡其他元素。图例与实际曲线一致；能带、电子 DOS、声子比较不得另设 QE 虚线分支。数值、坐标、费米线、能轴和投影数据不变。

第 5 步默认两个 PNG 及对应 plot.json 的 bounds 必须为实际 dis_win_min／max 相对 E_F 的边界；重绘旧包也按保存的实际窗口更新。窗口缺失、重复、非有限或倒序时拒绝。`electronic-display.json` 以 `default_ev` 和 `default_preset=wannier_outer` 表示当前默认图，原 `near_ev=[-5,5]` 不再代表第 5 步默认图。`-valence` 及单侧 QE 范围不变，科学数值和实际选窗不修改。
