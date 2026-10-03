## 任务计划
- [x] 实现物理磁盘采集、独立生命周期和有界短期历史。
- [x] 接入磁盘页面和异常状态，完成行为及浏览器视觉检查。
- [ ] 构建、核对完整 Windows 运行包并本地提交。

## 进度

- Sakani 0.3.1 Stat Card、Select、Line Chart 及既有业务曲线 tokens 已核对；[浅色](assets/p1-light.jpg)、[深色](assets/p1-dark.jpg)、[窄窗](assets/p1-narrow-dark.jpg)与[失败空态](assets/p1-failed-light.jpg)留存。无效值为 —，曲线保留空白；420×400 页面宽度与滚动宽度相等，图例明确读写。HTML 仅提供布局参考；原生视觉仍单独验收，统一交付见 [desktop-runtime](../v0.1.0-desktop-runtime/execution.md)。
- 已接入独立工作线程、五秒页面租约、窗口隐藏停采、设备选择及两个趋势。原生只读探针三轮通过：先预热，再获取两块物理盘的真实读写和活动时间。CPU/基础采样无新增来源。
- 核心有界历史/间断/失效测试、前端 32 项测试、类型检查与全目标 Clippy 通过。深浅截图和完整运行包待本轮统一交付；实际热插拔、原生视觉和长时开销尚未验证。
