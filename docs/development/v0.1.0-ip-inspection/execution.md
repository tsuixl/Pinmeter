## 任务计划

- [x] 实现出口检测、资料缓存、网络/AI 资源探测和官方服务状态。
- [x] 保留移植来源、版权和许可声明，完成有界调度、取消及异常状态检查。
- [ ] 确认上游服务条件、品牌资源使用依据和发行对应源码。
- [ ] 补测原生 WebView 全流程、PAC/系统代理、其他平台、更多网络环境及资源预算。

## 进度

- 出口与资料由后端唯一缓存管理；请求有并发、截止时间、响应体及缓存上限，离页和最小化取消待执行请求，失败保留原数据日期。
- AI 资源收到 HTTP 响应时显示耗时，连接失败显示未取得响应；不以资源响应证明登录、对话或账户可用。429 冷却继续保留。
- 既有核心/适配/宿主/前端测试及真实服务冒烟通过，覆盖响应头计时、超时、限流、未知字段、缓存和迟到结果。Windows 支持当前账户手动 HTTPS 代理，PAC/自动发现仍不支持；其他平台系统代理未验收。
- 移植版本、变更和许可见 [NOTICE](../../../src/backend/platform/src/ip/third-party/NOTICE) 与 [许可证](../../../src/backend/platform/src/ip/third-party/one-ip.LICENSE)；[品牌来源](../../../src/frontend/src/features/ip/assets/brands/NOTICE)不代替权利依据。整体 AGPL 已确定，相关服务与发行待办见 [desktop-runtime](../v0.1.0-desktop-runtime/execution.md)。
