# 验证方法

当前恢复已经由作业 1316704 实测：指数格式修复有效，但现有输入的 iso 非线性及 aniso 第二初值仍未收敛。现在应先诊断原生求解参数，不把再次提交相同配置当作新验证。

使用规范构建入口运行 superconductivity 与 three_stage_execution 回归，包含原生数值格式和完整能隙负向检查。只读真实输出复验使用原 attempt.json 请求，不执行外部程序、不改历史文件。

安装后进入 examples/MgB2/sugon-amd/run/epwflow，执行 epw_superconductivity -c epwinput.toml --stage superconductivity --resume -s。成功线性子结果及上游复用，失败尝试归档后生成新尝试。原生 NaN 或未收敛继续记录为失败，不把格式修复当成科学验收完成。
