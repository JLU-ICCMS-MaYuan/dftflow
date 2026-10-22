# 接口契约：阶段内绘图

## 触发

保留现有三个公开程序。-r 在该阶段数值任务成功核验后立即出图；-s 仍每次只提交一个原生调用，原生作业结束后再次执行同一公开命令，在核验成功时出图并按现有流程推进。不新增公开 CLI 或配置项。

复用阶段也补齐缺失图像。电子两个叶任务各自出图，互不等待；声子图不依赖 Wannier。status 只读，不创建图。

## 输出

| 目录 | 主要输出 |
|---|---|
| qe_eband/plots | electronic-bands.png、电子能带 CSV、completion.json |
| qe_eledos/plots | electronic-dos.png、电子 DOS CSV、completion.json |
| qe-phonon/plots | phband/phonon_band.png、声子谱与标签 CSV、phdos/phonon_dos.png、原子／元素 CSV、phonon_combined.jpg、completion.json |
| 第 5 步 | 原有电子 QE／Wannier 叠图，位置与接口保持 |
| 后续声子对比 | 原有声子谱、TDOS 独立图与总览，不生成元素投影面板 |

本轮沿用单侧绘图库已有格式；图像包含来源方法、物理坐标／单位与元素图例。

## 错误与恢复

来源损坏、非有限数值、路径不匹配或原子投影缺失必须报错。图像完成清单最后写入。成功科学任务已有回执／后处理检查点保留；再次调用公开命令（错误恢复沿用 --resume）只补图，不重复成功科学程序。图片删除后正常调用即可补齐。图像不能改变科学有效性或错误状态。
