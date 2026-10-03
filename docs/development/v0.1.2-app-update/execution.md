## 任务计划

- [x] 2026-10-04：补齐更新错误分类、安装器版本绑定和错版材料回归，保留未发布通道与真实安装验收边界。

- [x] 核对 Cindy 更新交互、Sakani 对应组件及 Pinmeter 退出/安装边界。
- [x] 实现更新状态、原子偏好、安装形态与 Tauri 更新适配。
- [x] 接入退出准备、签名下载和可信发布清单生成。
- [x] 实现设置/侧栏/托盘入口及更新公告弹窗。
- [ ] 完成状态与失败路径检查、深浅主题及实际更新链路验证。
- [x] 构建完整 Windows release，记录来源、资源核对、启动和未验证项；本地提交。

## 进度

- 2026-10-04 本批交付：源码 `96b8c4677384bf32f09f7aec0920698832492bfd`，实际入口 `E:\dev\github\Pinmeter\src\backend\target\test6\Pinmeter.exe`。统一检查通过（Rust 167 项、前端 49 项、格式/Clippy/契约/打包工具）；完整 release/NSIS 构建、42 个运行文件及 376 个源码指纹核对通过。原生、管理员、真实升级及长期未验证项保留；[交付详情](../v0.1.0-desktop-runtime/execution.md)。

- 2026-10-04：区分签名版本不匹配、签名失败、无有效清单、平台包缺失、网络和本地文件错误；不会将发行材料错误统一提示为网络故障。清单生成前核对安装器名称/目标版本、PE 产品版本、可信签名版本（存在时）并检查核对期间文件未变化。5 项 Node 回归通过，宿主错误映射测试通过；实际构建的 NSIS 将再次走同一校验。固定稳定版入口不变，预览版仍到官方发行页手动获取；未发布清单明确未就绪，未执行真实安装/卸载或网络规则恢复。

- 当前：代码实现与本地交付完成，真实安装升级及正式发行验收未完成。工作区开始时干净，基线 `30f3266`；用户已授权先实现在线更新，再处理 P0 常驻稳定性，未授权公开发布。
- 已核对官方 Sakani Modal 深浅模式、Toast 状态、Progress 确定/不确定进度；Cindy 仅作交互和状态设计参考。当前尚未进行本功能 UI 或安装验收。
- 后续 P0：在 desktop-runtime、basic-monitoring、taskbar-display 原文档中继续常驻资源、休眠/Explorer/网卡恢复和长期检查，不另建重复功能目录。
- 自动验证：项目 `tools/dev.ps1 check` 已通过，包含 helper 自检、构建互斥/资源/卸载夹具、Rust 测试与 Clippy、受控契约、前端构建/25 项测试/格式。Rust 保留原有 3 项外部环境测试的跳过状态。新增本地 HTTP + Tauri MockRuntime 集成测试验证新版本检查、真实签名下载及篡改拒绝；不运行安装器，不等同实际 A→B 安装通过。修复测试 EXE 缺少 Common Controls v6 manifest 导致的加载失败后两项均通过。
- UI：已检查官方 Modal 深浅主题、Toast 与 Progress；浏览器演示完成查看、下载禁用/进度、就绪、失败、稍后与 Escape 关闭，420×400 最小窗口正文可滚动且按钮可用。证据：[浅色公告](assets/update-preview-light.jpg)、[下载中](assets/update-downloading-light.jpg)、[浅色就绪](assets/update-ready-light.jpg)、[深色就绪](assets/update-ready-dark.jpg)、[失败](assets/update-failed-light.jpg)、[最小窗口](assets/update-minimum-window.jpg)。仍需正式 WebView、管理员安装和原生缩放验收，不宣称完整视觉/发行验收通过。
- 分发：签名私钥位于本机 `%LOCALAPPDATA%\Pinmeter\signing\updater.key`，只提交公钥；发布前工具复核实际安装包、签名与同版本公告，篡改包/签名测试通过。已补齐 5 个新增 Windows 正常依赖的上游许可。签名包/清单尚未公开发布；GitHub latest 不包含预览发行，需在正式发布时明确推进更新通道。
- 交付（2026-10-03）：从干净源码 `6986f395cbdc69212abc613a7c32b0321d10ce33` 通过 `node tools/desktop.mjs build -- --locked` 构建；构建锁和 `test1` 独占保持至交付。实际 EXE 为 `E:\dev\github\Pinmeter\src\backend\target\test1\Pinmeter.exe`，完整目录包含 40 个运行文件；整理时和交付前分别对源/副本 SHA-256 复核，287 项源码清单也一致。主 EXE SHA-256：`34098e6160b17eec20b781d5db5951b73f94fcf301349f56c9171a15d320dfbe`。
- 安装材料：`test1/update-artifacts/Pinmeter_0.1.2_x64-setup.exe`、相邻 `.sig`、`latest.json` 和公告；安装器来源为 `target/desktop-build/release/bundle/nsis/`，复制后哈希一致，文件版本为 0.1.2，SHA-256：`69fbe4845c600711cb9090500e4de7017f1253bc0a4c7c3f54407501659119fd`。这里的签名是 Tauri 更新包签名，未配置 Windows Authenticode。构建来源、逐文件清单与独立核对记录保存在该交付目录。
- 运行边界：从交付目录启动 CPU/GPU helper 均正常退出且 stderr 为空；CPU 如实返回未安装 PawnIO 的 `unsupported`，GPU 三轮读取为 `normal`，识别 RTX 4070 Laptop GPU。本机普通权限环境下未启动管理员正式主窗口或执行真实 A→B 安装；系统干预测试时机已询问用户，尚未执行休眠/Explorer 重启或八小时常驻。模拟样本、MockRuntime 和资源核对不替代这些验收。
