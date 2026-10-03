## 任务计划
- [x] 实现只读采样、进程身份与 CPU 差值、Top 10。
- [x] 接入界面，检查权限/空态与排序，完成测试及截图。
- [ ] 构建、核对完整 Windows 运行包并本地提交。

## 进度

- 对照 Sakani 0.3.1 Table 的默认 44px 行、深色与窄窗堆叠示例，并复用其 Segmented Control/Alert；[浅色](assets/p1-light.jpg)、[深色](assets/p1-dark.jpg)、[窄窗](assets/p1-narrow-dark.jpg)及[权限空态](assets/p1-permission-light.jpg)已留存，CPU/内存切换、部分数据提醒和空态可用，420×400 页面无横向溢出。HTML 仅作结构参考；真实管理员主窗口仍未验收，统一交付见 [desktop-runtime](../v0.1.0-desktop-runtime/execution.md)。
- 已实现 Windows Toolhelp + 受限查询权限的进程时间/工作集采集，页面 CPU/内存 Top 10、时间戳与部分数据提示。无终止进程或提权入口，未读取完整命令行，完整路径不出适配器。
- 原生只读探针两轮成功：379 个进程、143 个部分不可读，单轮约 12 ms；自身 CPU 与工作集、两种前十排行均有效。此为当前权限的短探针，不等于管理员/所有受保护进程验收。
- CPU 多核归一、PID 复用、回退、间断、权限及稳定排序回归通过；全目标 Clippy 与前端类型检查通过。深浅/窄窗截图和最终 Windows 运行包待本轮统一核对。
