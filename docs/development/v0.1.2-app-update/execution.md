## 任务计划

- [x] 核对 Cindy 更新交互、Sakani 对应组件及 Pinmeter 退出/安装边界。
- [x] 实现更新状态、原子偏好、安装形态与 Tauri 更新适配。
- [x] 接入退出准备、签名下载和可信发布清单生成。
- [x] 实现设置/侧栏/托盘入口及更新公告弹窗。
- [ ] 完成状态与失败路径检查、深浅主题及实际更新链路验证。
- [ ] 构建完整 Windows release，记录来源、资源核对、启动和未验证项；本地提交。

## 进度

- 当前：实现中。工作区开始时干净，基线 `30f3266`。本轮用户已授权先实现在线更新，再处理 P0 常驻稳定性；未授权公开发布。
- 已核对官方 Sakani Modal 深浅模式、Toast 状态、Progress 确定/不确定进度；Cindy 仅作交互和状态设计参考。当前尚未进行本功能 UI 或安装验收。
- 后续 P0：在 desktop-runtime、basic-monitoring、taskbar-display 原文档中继续常驻资源、休眠/Explorer/网卡恢复和长期检查，不另建重复功能目录。
- 自动验证：项目 `tools/dev.ps1 check` 已通过，包含 helper 自检、构建互斥/资源/卸载夹具、Rust 测试与 Clippy、受控契约、前端构建/25 项测试/格式。Rust 保留原有 3 项外部环境测试的跳过状态。新增本地 HTTP + Tauri MockRuntime 集成测试验证新版本检查、真实签名下载及篡改拒绝；不运行安装器，不等同实际 A→B 安装通过。修复测试 EXE 缺少 Common Controls v6 manifest 导致的加载失败后两项均通过。
- UI：已检查官方 Modal 深浅主题、Toast 与 Progress；浏览器演示完成查看、下载禁用/进度、就绪、失败、稍后与 Escape 关闭，420×400 最小窗口正文可滚动且按钮可用。证据：[浅色公告](assets/update-preview-light.jpg)、[下载中](assets/update-downloading-light.jpg)、[浅色就绪](assets/update-ready-light.jpg)、[深色就绪](assets/update-ready-dark.jpg)、[失败](assets/update-failed-light.jpg)、[最小窗口](assets/update-minimum-window.jpg)。仍需正式 WebView、管理员安装和原生缩放验收，不宣称完整视觉/发行验收通过。
- 分发：签名私钥位于本机 `%LOCALAPPDATA%\Pinmeter\signing\updater.key`，只提交公钥；发布前工具复核实际安装包、签名与同版本公告，篡改包/签名测试通过。已补齐 5 个新增 Windows 正常依赖的上游许可。签名包/清单尚未公开发布；GitHub latest 不包含预览发行，需在正式发布时明确推进更新通道。
