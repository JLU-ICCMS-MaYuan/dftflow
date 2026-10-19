# 三阶段流程与命令入口

当前用户已选择三个独立程序：`epw_phonon`、`epw_wannier`、`epw_superconductivity`。配置继续使用 `inputepw.toml`；原单程序子命令与 `--until` 使用说明已由本次契约替代。

完整流程图、每阶段命令、脚本生成／提交／恢复／重绘见[模块 README](../../../epwflow/README.md)，职责与验收见[三程序契约](three-programs.md)，持久记录见[接口契约](contracts/interface.md)。

三个入口及粗／细清单已接入，不能据此声称 qeflow 直接执行库、自动参考／四类图或真实两模式 Tc 全部验收完成。具体证据见[本项验收](acceptance.md)。科学质量仍采用当前最大／RMS 50／20 cm⁻¹，共享预算不重置；Mori LDA 结果不能作为此目录 PBE 输入的参考。
