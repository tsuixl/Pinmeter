# process-ranking

> 当前工作版本：v0.1.3

## 目标与范围
- P1：只读 CPU / 内存 Top 10，显示进程名、PID、CPU 占用、工作集；不提供结束进程、提权或系统优化。

## 方案
- 遵循[产品设计](../../design/product-design.md)与[架构](../../architecture/overview.md)。core/processes 计算 CPU 差值、识别 PID 复用和排行；platform/processes 封装 Toolhelp、GetProcessTimes、GetProcessMemoryInfo。host 管理单工作线程，ViewModel 通过 client 读取。
- CPU 为进程内核与用户时间增量 / 墙钟采样时长 / 逻辑处理器数，与全机百分比口径一致；内存为工作集，不称为独占或私有内存。创建时间变化时重新预热；无权限不可推算为零。
- 每两秒采集；主窗口可见且页面轮询五秒租约有效时工作，切页五秒内停止。最多 4096 个进程，展示无法读取和截断计数；不读取完整路径或命令行、不保存进程历史。其他平台明确不支持。
- 使用当前受控 Sakani [Table](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/composite-table--docs)、Segmented Control、Alert 与 Badge；深浅主题沿用 tokens。HTML 仅作结构参考。

## 验收
- CPU 多核归一、PID 复用、计数回退、权限和退出、排序稳定、前十限制有测试；普通权限只读探针通过。
- 类型/构建与深浅/窄窗界面检查完成；未实测权限或规模场景如实记录。
