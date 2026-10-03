# alerts 执行记录

## 任务计划

- [x] 建立默认关闭的核心规则、连续状态和最近 20 条会话事件，补齐边界测试。
- [x] 复用 Settings 原子持久化与基础帧接入，增加主窗口授权命令和托盘提示。
- [x] 提供 Sakani 设置、会话记录与点击定位，明确系统通知及平台边界。
- [x] 完成前端类型、行为检查和浏览器深浅主题对照。
- [ ] 统一核心测试、完整 Windows 运行包构建与交付核对；原生触发及托盘实测。

## 进度

- 2026-10-04：已确认规则和分层方案；网络控制与故障恢复不在本功能范围。Tauri 官方文档核对 Windows 通知仅安装版，本轮不增加系统通知依赖或占位开关。
- 前端 `vitest run tests/alerts-store.test.ts` 3 项通过：共用轮询、隐藏停请求和最后订阅退出停止，旧读取不覆盖保存回执，重复保存抑制和失败保留确认状态。最后一次全项目 `tsc --noEmit` 通过。Rust 通过本轮首次宿主编译；核心测试由主任务统一执行。
- 浏览器演示已验证 CPU / 内存 / 静默默认全关、开启后编辑阈值与持续时间、保存成功、失败不确认配置、放弃草稿、事件点击进入 CPU 详情并确认、清除记录。留存[默认浅色](assets/settings-light.jpg)、[已启用深色](assets/settings-dark.jpg)、[保存与已查看事件](assets/saved-light.jpg)、[保存失败](assets/save-failure-light.jpg)、[主页面提醒](assets/banner-light.jpg)和[420×400 窄窗](assets/narrow-light.jpg)。窄窗 `document.clientWidth/scrollWidth` 均为 420，三组字段容器均为 267/267，无横向溢出，保存与放弃操作可见。
- 已实际查看官方 Sakani Alert All Colors/深色、Switch、Input 组件与状态，参考留存[深色 Alert](assets/sakani-alert-dark.jpg)、[Switch](assets/sakani-switch.jpg)、[Input](assets/sakani-input.jpg)。直接复用受控 0.3.1 组件及 tokens，CSS 仅负责排列与响应式；HTML 仅作布局参考。上述浏览器范围视觉通过，不代表原生 DPI、完整键盘可访问性或真实后台触发通过。
- 尚未实测：真实持续高占用触发、托盘提示、跨午夜与系统时区变化、重启配置恢复、非 Windows 提醒和长时性能。无系统通知依赖及系统弹窗，非 Windows 本地静默时段当前明确不可用。完整 release、资源哈希核对、实际交付路径及本地提交由主任务统一补充。
