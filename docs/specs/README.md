# Spec 目录索引与 GitHub Issue 对照

本目录每个子目录对应一个需求编号，编号即 GitHub Issue 编号：`docs/specs/00NN-*` ↔
<https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/NN>。

- Issue 的 open／closed 状态是协作状态权威来源，本文件只是编号与路径对照表，不复制状态。
- 0002、0003 没有 `issue-draft.md`，Issue 正文取自各自 `spec.md`；0004 只发布一个 Epic，
  其 P0–P5 阶段已实现验收完成，作为正文历史阶段保留，未单独建子 Issue。
- Epic 0007（#7）通过 GitHub 原生 sub-issue 关联 0008、0010、0012、0013、0014。
- 发布分支为 `rust-master`；Issue 正文内指向本目录文档的链接均指向该分支。

| 编号 | 目录 | Issue | 唯一类型 |
| --- | --- | --- | --- |
| 0001 | [0001-scflow-qe-superconductivity](0001-scflow-qe-superconductivity/) | [#1](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/1) | `type:feature` |
| 0002 | [0002-matdyn-crystal-coordinates](0002-matdyn-crystal-coordinates/) | [#2](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/2) | `type:bug` |
| 0003 | [0003-phonon-band-dos-combined](0003-phonon-band-dos-combined/) | [#3](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/3) | `type:feature` |
| 0004 | [0004-dftflow-rust](0004-dftflow-rust/) | [#4](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/4) | `type:epic` |
| 0005 | [0005-git-hygiene-manual-testing](0005-git-hygiene-manual-testing/) | [#5](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/5) | `type:feature` |
| 0006 | [0006-qe-phonon-submit-chain](0006-qe-phonon-submit-chain/) | [#6](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/6) | `type:feature` |
| 0007 | [0007-epwflow-superconductivity](0007-epwflow-superconductivity/) | [#7](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/7) | `type:epic` |
| 0008 | [0008-epwflow-foundation](0008-epwflow-foundation/) | [#8](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/8) | `type:feature` |
| 0009 | [0009-phonon-band-plot-rewrite](0009-phonon-band-plot-rewrite/) | [#9](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/9) | `type:bug` |
| 0010 | [0010-epwflow-qe-data](0010-epwflow-qe-data/) | [#10](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/10) | `type:feature` |
| 0011 | [0011-build-history-cleanup](0011-build-history-cleanup/) | [#11](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/11) | `type:maintenance` |
| 0012 | [0012-epwflow-interpolation](0012-epwflow-interpolation/) | [#12](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/12) | `type:feature` |
| 0013 | [0013-epwflow-superconductivity-tc](0013-epwflow-superconductivity-tc/) | [#13](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/13) | `type:feature` |
| 0014 | [0014-epwflow-delivery](0014-epwflow-delivery/) | [#14](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/14) | `type:feature` |
| 0015 | [0015-epwflow-mgb2-real-validation](0015-epwflow-mgb2-real-validation/) | [#15](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/15) | `type:maintenance` |
| 0016 | [0016-wannier-window-selection](0016-wannier-window-selection/) | [#16](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/16) | `type:feature` |
| 0017 | [0017-build-binaries-layout](0017-build-binaries-layout/) | [#17](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/17) | `type:maintenance` |
| 0018 | [0018-epwflow-stage-workspaces](0018-epwflow-stage-workspaces/) | [#18](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/18) | `type:feature` |
| 0019 | [0019-shell-script-options](0019-shell-script-options/) | [#19](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/19) | `type:maintenance` |
| 0020 | [0020-qe-electronic-postprocess-resume](0020-qe-electronic-postprocess-resume/) | [#20](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/20) | `type:bug` |
| 0021 | [0021-qe-dos-pencil-resume](0021-qe-dos-pencil-resume/) | [#21](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/21) | `type:bug` |
| 0022 | [0022-slurm-log-filenames](0022-slurm-log-filenames/) | [#22](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/22) | `type:maintenance` |
| 0023 | [0023-epw-wannier-native-start](0023-epw-wannier-native-start/) | [#23](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/23) | `type:bug` |
| 0024 | [0024-slurm-script-simplification](0024-slurm-script-simplification/) | [#24](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/24) | `type:maintenance` |
| 0025 | [0025-epw-visible-run-state](0025-epw-visible-run-state/) | [#25](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/25) | `type:maintenance` |
| 0026 | [0026-epw-direct-native-submit](0026-epw-direct-native-submit/) | [#26](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/26) | `type:maintenance` |
| 0027 | [0027-mgb2-epwflow-example-debug](0027-mgb2-epwflow-example-debug/) | [#27](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/27) | `type:bug` |
| 0028 | [0028-phonon-postprocess-checkpoint](0028-phonon-postprocess-checkpoint/) | [#28](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/28) | `type:bug` |
| 0029 | [0029-active-slurm-log-archive](0029-active-slurm-log-archive/) | [#29](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/29) | `type:bug` |
| 0030 | [0030-wannier-guiding-centres](0030-wannier-guiding-centres/) | [#30](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/30) | `type:feature` |
| 0031 | [0031-postw90-dos-spin-normalization](0031-postw90-dos-spin-normalization/) | [#31](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/31) | `type:bug` |
| 0032 | [0032-disconnected-electronic-band-paths](0032-disconnected-electronic-band-paths/) | [#32](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/32) | `type:bug` |
| 0033 | [0033-wannier-velocity-handoff](0033-wannier-velocity-handoff/) | [#33](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/33) | `type:bug` |
| 0034 | [0034-reference-orchestrator-upgrade](0034-reference-orchestrator-upgrade/) | [#34](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/34) | `type:bug` |
| 0035 | [0035-native-retry-slurm-logs](0035-native-retry-slurm-logs/) | [#35](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/35) | `type:bug` |
| 0036 | [0036-epw-mutable-wavefunction-scratch](0036-epw-mutable-wavefunction-scratch/) | [#36](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/36) | `type:bug` |
| 0037 | [0037-matdyn-path-unused-weight](0037-matdyn-path-unused-weight/) | [#37](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/37) | `type:bug` |
| 0038 | [0038-comparison-orchestrator-upgrade](0038-comparison-orchestrator-upgrade/) | [#38](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/38) | `type:bug` |
| 0039 | [0039-comparison-path-labels](0039-comparison-path-labels/) | [#39](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/39) | `type:bug` |
| 0040 | [0040-interpolation-quality-warnings](0040-interpolation-quality-warnings/) | [#40](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/40) | `type:feature` |
| 0041 | [0041-stage-output-verification-io](0041-stage-output-verification-io/) | [#41](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/41) | `type:bug` |
| 0042 | [0042-epw-fortran-exponent](0042-epw-fortran-exponent/) | [#42](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/42) | `type:bug` |
| 0043 | [0043-epw-comparison-max-differences](0043-epw-comparison-max-differences/) | [#43](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/43) | `type:feature` |
| 0044 | [0044-mpi-submit-command](0044-mpi-submit-command/) | [#44](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/44) | `type:maintenance` |
| 0045 | [0045-epw-gap-temperatures](0045-epw-gap-temperatures/) | [#45](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/45) | `type:feature` |
| 0046 | [0046-epw-submit-simplification](0046-epw-submit-simplification/) | [#46](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/46) | `type:feature` |
| 0047 | [0047-qe-stage-plots](0047-qe-stage-plots/) | [#47](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/47) | `type:feature` |
| 0048 | [0048-epw-phonon-stage-chain](0048-epw-phonon-stage-chain/) | [#48](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/48) | `type:feature` |
| 0049 | [0049-epw-multitemperature-submit](0049-epw-multitemperature-submit/) | [#49](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/49) | `type:feature` |
| 0050 | [0050-epw-stage-results](0050-epw-stage-results/) | [#50](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/50) | `type:feature` |
