# 当前目录 PBE 固定温度点运行验收

## 已确认的范围

用户原话：

> 我不需要你确保算出来的超导温度是不是正确。我也不需要你做事有界诊断。我只需要你给我跑通几个温度点的 iso 和 aniso 的超导温度即可。只要能正常的跑完一个 epw.x看到最后正常跑完就行。你可以选的温度点是 20，30，40 K这些非常典型的温度点。不需要考虑是不是要拿到最合理的 Tc

当前要求改为使用指定目录 PBE 数据，在 20、30、40 K 分别执行 iso 和 aniso 共六次原生 epw.x；以退出码和完整结束日志验收执行。取消本次 Tc 区间搜索、临界附近独立参考、双初值一致性、最合理 Tc 及先做有界诊断的要求。温度点的数值收敛状态据实报告，不作为本次执行完成的额外门槛，也不据固定温度点发布 Tc。保留历史输入输出、原共享资源额度和来源记录。

## 输入与执行

复用 `examples/MgB2/sugon-amd/run/epwflow/7.epw-fine/smearing-0001/` 的谱、crystal、selecq 与矩阵元副本。运行目录为 `8.epw-superconductivity/temperature-points-001/`，两个模式各有 `020K/030K/040K`；原 TOML 和旧 attempt 不改。

保留当前原生输入的 μ*=0.05、截断 0.5 eV、1000 次、10⁻⁸、Broyden 0.7／8、初始形状 1 和 k/q 网格，统一选择已有 10 meV 初值。每个 epw.x 仅运行一个温度；一个 1 节点／32 MPI 分配顺序执行，申请上限 30 分钟，计入原共享账本。

## 验收证据

已完成实际运行。使用每点的 `epw.in`、`epw.out`、`epw.err`、`exit-code.txt` 及 Slurm 终态验证；EPW 5.9 以 `Total program execution`、完整 `EPW : ... CPU ... WALL` 计时及末尾引用段为结束证据，不强求其他版本的 `JOB DONE` 或 `This run was terminated on`。收敛声明、NaN 和能隙文件另列，不能以退出 0 冒充数值收敛，也不由三个固定温度宣称得到了 Tc。

当前关联：0013 T049、0015 T034；线上 Gitee 由用户管理，本轮不更新线上 Issue，不推送。


## 实际结果

| 模式 | 温度（K） | 作业 | 迭代次数 | 原生退出码 | 结果 |
| --- | --- | --- | --- | --- | --- |
| iso | 20 | 1318744 | 12 | 0 | 收敛，完整结束，无 NaN |
| iso | 30 | 1318744 | 12 | 0 | 收敛，完整结束，无 NaN |
| iso | 40 | 1318744 | 13 | 0 | 收敛，完整结束，无 NaN |
| aniso | 20 | 1318779 | 126 | 0 | 收敛，完整结束，无 NaN |
| aniso | 30 | 1318779 | 113 | 0 | 收敛，完整结束，无 NaN |
| aniso | 40 | 1318779 | 188 | 0 | 收敛，完整结束，无 NaN |

首轮 1318744 在 `temperature-points-001` 执行六点：三个 iso 均收敛；三个 aniso 虽退出 0 并有结束计时，但迭代出现 NaN，明确未收敛。这些输入输出保留，不选作最终 aniso 结果。

随后只重跑三个 aniso：`temperature-points-002/aniso/{020K,030K,040K}/` 使用 EPW 原生线性混合，`broyden_beta=-0.3`（负号选择线性算法，幅值 0.3），其他单点设置不变。保留 1000 次上限和 10⁻⁸；三点均达到原生收敛并输出完整 `MgB2.imag_aniso_020.00/030.00/040.00`。无需新的 SCF／NSCF／PH／Wannier／粗细 EPC，没有再做先前的 10 K／80 K 有界诊断或搜索。

最终 iso 使用 `temperature-points-001/iso/{020K,030K,040K}/`，aniso 使用 `temperature-points-002/aniso/{020K,030K,040K}/`。每点都有 `epw.in`、`epw.out`、`epw.err`、`exit-code.txt`、起止时间和原生虚轴输出。最终统一索引为 `temperature-points-001/final-results.json` 与 `final-results.csv`；首轮失败尝试也列入 JSON 的 `all_attempts`。

## 资源、保护与完成范围

- 作业 1318744 为 `COMPLETED / 0:0`，1 节点，21 秒；1318779 为 `COMPLETED / 0:0`，1 节点，7 秒，合计 28 节点秒。原共享账本累计从 4017 变为 4045／28800 节点秒，无预留或活动分配；未重置任何历史条目。
- 两轮前后各核对 42 项来源身份，涵盖原配置、原正式 F4 清单、实际 EPW 二进制、谱、crystal、selecq 和必需矩阵文件；哈希一致。每轮重读文件使用独立副本，原 `epwinput.toml`、前七阶段和旧尝试未改。
- 验证包含 Bash 语法、实际六点输入模式／温度、原生退出、完整结束计时、原生收敛声明、NaN 拒绝、能隙文件存在和 Slurm 终态；原生日志和完整输出仅存本地忽略目录。
- 0013 T049、0015 T034 的本次固定温度点执行已通过，FR-014 对应本次 SC-009／SC-008；未做 Tc 搜索或物理合理性保证，不修改原正式 Tc 清单，也不把三点运行称为测得转变温度。旧完整 Tc 任务从本次范围取消，不勾选为已实现或已验证。
- 本次只交付原生单点输入／运行证据与文档，未改 Rust 生产行为、默认安装或线上 Gitee Issue，未推送。Overview 不新增产品功能描述；本次操作和验收说明写入本 Spec 与算例 README。

## 后续公开功能与用户重跑清理（2026-10-22）

后续 [0045 指定温度功能](../0045-epw-gap-temperatures/acceptance.md) 已将温度列表接入 TOML 和公开第 8 步，并由当前 PBE 同字节输入完成干净工作区八阶段及六点实测；六点均正常结束、明确收敛且数据有效。历史本页手动调用不作为该项公开流程验收证据。

用户要求全流程验证成功后清理原算例以亲自重跑；该条件满足后，原 epwflow 中包括本页 temperature-points 在内的旧生成数据已删除，保留 README.md、epwinput.toml、KPATH.in。本页数值和预算是当时记录；当前公开原始证据、删除前清单与清理收据位于 docs/local/0045-gap-temperatures/，操作以 [当前算例 README](../../../examples/MgB2/sugon-amd/run/epwflow/README.md) 为准。
