# SiP₂ 的 FFT 网格与 dvscf 接续问题分析

核查日期：2026-10-27。

本文依据仓库 `external/q-e` 中的 QE／EPW 源码、当前 [epwinput.toml](epwinput.toml) 和历史 SiP₂ 电子计算 XML，解释 FFT 网格从哪里来、如何参与 `dvscf` 读取，以及哪些结论已经得到证据支持。源码核查基线为 `51fdfbd1320d31c6684b7565aa5bc99584241c7c`；不据此认定当前配置指向的已安装程序与该源码完全一致。

## 1. 六个参数分别控制什么

这些参数规定晶胞内实空间 FFT 网格的三个维度，数值没有长度单位，与电子 `K_POINTS` 网格不同。

| 参数 | 当前配置 | QE／EPW 内部描述 | 主要用途 |
| --- | --- | --- | --- |
| `nr1、nr2、nr3` | `216 × 128 × 60` | `dfftp`，密网格 | 电荷密度、自洽势，以及 PH 保存的响应势 |
| `nr1s、nr2s、nr3s` | `120 × 72 × 32` | `dffts`，平滑网格 | 波函数 FFT、电荷密度的平滑部分，以及电子声子矩阵元计算中的相应运算 |

两套网格可以不同。跨阶段要求兼容，不等于要求 `nr1 = nr1s`、`nr2 = nr2s`、`nr3 = nr3s`。

## 2. 网格尺寸来源与文件读取是先后两步

**普通首次计算粗网格电子声子矩阵元时，EPW 先从 QE 保存目录的 XML 获取 FFT 尺寸，再按这些尺寸读取 PH 写出的 `dvscf`。这不是两种可选的读取方式。**

```text
EPW 的 outdir/prefix.save/data-file-schema.xml
通常由供 EPW 使用的 NSCF 生成
        │
        │ 读取 fft_grid 和 fft_smooth 的实际尺寸
        ▼
EPW 内存中的 dfftp（密网格）和 dffts（平滑网格）
        │
        │ 按 dfftp 确定响应势数组与文件记录长度
        ▼
读取 dvscf_dir/prefix.dvscf_q*
这些文件由此前的 PH 生成
```

| EPW 数据入口 | 提供的内容 |
| --- | --- |
| `outdir/prefix.save` | 电子计算数据，以及本次 EPW 使用的 FFT 网格尺寸 |
| `dvscf_dir` | PH 已经计算好的响应势数值 |

**EPW 不会在这个读取路径中从旧 `dvscf` 识别出 PH 原始三维网格，再用它覆盖 NSCF 提供的网格。** 更准确的说法是“从 QE XML 取得网格尺寸”，而不是“从 QE 保存目录里的 dvscf 取得网格尺寸”。

### 2.1 PH 按密网格写出响应势

[solve_linter.f90](../../external/q-e/PHonon/PH/solve_linter.f90) 中，`dvscfp` 按密网格分配，并在收敛后通过 `davcio_drho` 写入响应势文件：

```fortran
allocate (dvscfp ( dfftp%nnr , nspin_mag , npe))
call davcio_drho ( dvscfp(1,1,ipert), lrdrho, iudvscf, imode0 + ipert, +1 )
```

[openfilq.f90](../../external/q-e/PHonon/PH/openfilq.f90) 根据密网格的存储尺寸定义记录长度：

```fortran
lrdrho = 2 * dfftp%nr1x * dfftp%nr2x * dfftp%nr3x * nspin_mag
```

这里 `nr1x/nr2x/nr3x` 是 FFT 数组的存储尺寸，可能包含填充；不能把任意响应文件的字节数直接当成唯一的物理三维网格。

### 2.2 EPW 从电子计算 XML 取得尺寸

[readin.f90](../../external/q-e/EPW/src/readin.f90) 在普通启动路径调用 `read_file()`；经 [read_file_new.f90](../../external/q-e/PW/src/read_file_new.f90) 调用 `read_xml_file()`，并初始化 FFT 数组。

[pw_restart_new.f90](../../external/q-e/PW/src/pw_restart_new.f90) 的 `read_xml_file` 将 XML 的 `output_obj%basis_set` 传给 `qexsd_copy_basis_set`。后者在 [qexsd_copy.f90](../../external/q-e/Modules/qexsd_copy.f90) 中执行：

```fortran
nr1 = basis_set%fft_grid%nr1
nr2 = basis_set%fft_grid%nr2
nr3 = basis_set%fft_grid%nr3
nr1s= basis_set%fft_smooth%nr1
nr2s= basis_set%fft_smooth%nr2
nr3s= basis_set%fft_smooth%nr3
```

因此，密网格和光滑网格都来自电子保存数据的实际输出。注意：XML 的 `fft_smooth` 属性仍叫 `nr1/nr2/nr3`，对应输入 `SYSTEM` 中的 `nr1s/nr2s/nr3s`。

### 2.3 EPW 按已确定的密网格读取文件

[io.f90](../../external/q-e/EPW/src/io/io.f90) 的 `openfilepw` 按当前 `dfftp` 设置 `lrdrho`；`readdvscf` 使用相应记录长度直接打开文件并读取：

```fortran
COMPLEX(KIND = DP), INTENT(inout) :: dvscf(dfftp%nnr, nspin_mag)
OPEN(iudvscf, FILE = tempfile, FORM = 'unformatted', &
     ACCESS = 'direct', IOSTAT = ios, RECL = unf_recl, STATUS = 'old')
READ(iudvscf, REC = recn) dvscf
```

该例程检查文件是否足够长，但没有读取原始网格维度，也没有把旧 PH 密网格自动转换为另一套 NSCF 密网格。文件没有报“太短”，不能证明网格兼容或读取正确。

## 3. 历史 SiP₂ 已确认存在的网格差异

2026-10-27 只读解析了以下历史目录中两份 `tmp/Si8P16.save/data-file-schema.xml`：

```text
/data/home/mayuan/work/SiP2/data/Pbam/exp_8K_relax_2026-09-29/SiP2_22.2GPa/epw_2x3x6_sugon_amd
```

| 历史子目录 | XML 密网格 | XML 平滑网格 |
| --- | --- | --- |
| `000_scf_nk236` | `216 × 128 × 60` | `120 × 72 × 32` |
| `001_nscf_nk236` | `216 × 125 × 60` | `120 × 72 × 32` |

如果待复用 PH 的确按前一套密网格写出了响应势，而 EPW 读取后一份 NSCF 保存目录，就会按 `216 × 125 × 60` 解释原本对应 `216 × 128 × 60` 的文件。这不能作为正确的数据接续。

这里应说“不能正确读取和使用”，而不应保证“必定立即报错”。记录长度改变会影响每个扰动记录的读取位置；程序也可能读入错误数据后继续运行。

这两份 XML 直接证明的是历史电子计算网格存在差异；待导入外部 PH 的来源和实际保存格式仍需独立绑定核验。不能仅凭目录名或这份表格宣称所有 PH 文件身份已经确认，也不能由此断定网格差异是历史崩溃的唯一原因。

当前 `epwinput.toml` 在 SCF、NSCF 和电子能带输入中显式设置相同的六个数，目的是固定网格，避免阶段间自动选择不同尺寸。固定输入后仍应核对实际 XML。对直接读取 `dvscf` 而言，关键约束在 PH 与 EPW 的密网格及存储布局兼容；DFTFlow 对 SCF／NSCF 两套 FFT 均一致的检查属于更严格的工作流接续要求，见 [verify_fft](../../epwflow/src/qe_data/xml.rs)。

## 4. 哪些重启分支不再读取原始 dvscf

下表针对启用相应电子声子处理路径的运行；不表示任意参数组合都可跳过其余前置检查。

| 工作方式 | 参数条件 | 读取 QE 保存目录 | 读取原始 `dvscf` |
| --- | --- | --- | --- |
| 首次计算粗网格矩阵元 | `epwread=false`、`epbread=false` | 是 | 是，使用上述读取路径 |
| 从 Bloch 表象粗网格矩阵元恢复 | `epbread=true` | 是 | 否，读取已有 `.epb` |
| 从 Wannier 表象数据恢复 | `epwread=true`、`epbread=false` | 否 | 否，读取已有 Wannier 数据 |

[readin.f90](../../external/q-e/EPW/src/readin.f90) 只有在 `epwread .AND. .NOT. epbread` 时跳过 `read_file()`。[ep_coarse_unfolding.f90](../../external/q-e/EPW/src/ep_coarse_unfolding.f90) 将原始响应势计算放在 `.NOT. epbread .AND. .NOT. epwread` 分支；`epbread` 分支读取 `.epb`。[wannier.f90](../../external/q-e/EPW/src/wannier.f90) 的 `epwread` 分支读取 Wannier 数据。

这些重启路径复用已经生成的矩阵元，不能修复首次生成矩阵元时错误读取响应势的问题。

## 5. 密网格转平滑网格是另一个步骤

[ep_coarse.f90](../../external/q-e/EPW/src/ep_coarse.f90) 在 `doublegrid` 分支中调用：

```fortran
CALL fft_interpolate(dfftp, dvscfin(:, is, ipert), dffts, dvscfin_tmp(:, is, ipert))
```

这是在文件已经读入后，将本次 EPW 的密网格响应转换到本次 EPW 的平滑网格。它不负责识别旧 PH 文件的不同密网格，也不能补救前面的记录长度错误。

该源码还存在独立的双网格处理缺陷：`dvscfin` 按密网格声明，`dvscfin_tmp` 按平滑网格分配，却执行整数组赋值 `dvscfin(:, :, :) = dvscfin_tmp(:, :, :)`；两者首维不同时不满足数组形状要求。之后的 `newdq2` 仍按密网格使用响应，而下游 `ep_coarse_compute` 按平滑网格接收。

因此，“PH 与电子数据的密网格匹配”和“EPW 自身双网格处理正确”是两个独立条件。显式固定六个参数只能处理前者，不能据此宣称后者已修复。相关既有核查见 [双网格源码分析](../../docs/specs/0012-epwflow-interpolation/native-double-grid-audit.md)。

## 6. 本次结论的证据边界

- 已确认：所核查源码先从电子计算 XML 建立 FFT 描述，再按其密网格读取原始响应势；相关重启分支会跳过原始响应势读取。
- 已确认：上述历史 SCF／NSCF 的密网格不同，平滑网格相同。
- 尚未证明：密网格自动选择差异的唯一原因、历史崩溃的唯一原因，以及当前已安装 EPW 与本次源码的逐字一致性。
- 本文仅记录分析；未修改 QE／EPW、计算配置或历史数据，未重算 SCF／NSCF／PH，未提交 Slurm 作业，也未完成外部声子导入或科学验收。
