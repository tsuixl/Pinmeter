## 任务计划
- [x] 将应用流量已接纳增量接入有界分钟历史，验证归并、去重、跨日及缺失语义。
- [x] 增加应用列表/筛选/排序/趋势、共享记录开关及启动记录偏好。
- [x] 验证文件恢复、UI 状态与完整 Windows 构建，记录本次交付并本地提交。
- [x] 实现分钟聚合、精确流量增量和 24 小时保留。
- [x] 实现后台持久化、重载与历史页。
- [x] 验证断档/跨日/损坏恢复及深浅主题，构建并交付完整运行包。

## 进度
- 应用流量历史已接入，旧网卡汇总不回填明细。按现有路径身份归并已接纳的窗口增量，去重、跨分钟/午夜、停止再开始、采集重建、丢事件/队列窗口与超限归并均有覆盖；最大 128 应用 × 1441 分钟的极值序列化仍小于 16 MiB。基础历史文件兼容，应用文件独立原子保存、损坏备份及退出排空/重载通过测试。
- 界面：今日/1/6/24 小时、名称/路径筛选、下载/上传/合计排序、单应用趋势、两页共用启停与默认关闭的启动记录偏好均已接入。沿用用户当前字体设置，Sakani 0.3.1 Table/Button/Switch/Input/Select/Alert 决定控件和主题，HTML 只作布局参考。已检查[浅色明细](assets/applications-light.jpg)、[深色明细](assets/applications-dark.jpg)、[同名筛选](assets/applications-filter-light.jpg)、[浅色趋势](assets/application-trend-light.jpg)、[深色趋势](assets/application-trend-dark.jpg)、[420×400 控件](assets/applications-narrow-dark.jpg)、[窄窗趋势](assets/application-trend-narrow.jpg)、[空态](assets/applications-empty.jpg)及[读取失败](assets/applications-query-failure.jpg)。未记录时不显示正常零流量；窄窗页面滚动宽度与可视宽度相等。
- 浏览器演示确认共享开始/停止、暂停后保留明细、启动偏好即时保存、上传排序和同名不同路径不混合；修正网络表头 portal 挂载后，往返切页控制台无错误。真实管理员 ETW 归档、自动冷启动、代理/VPN、休眠和 24 小时实机长测尚未验收，模拟与浏览器截图不代替这些结论。
- `tools/dev.ps1 check` 完整通过：347 个文档链接与分层/忽略检查、helper 和构建互斥/卸载回归、Rust 工作区测试与 Clippy、契约同步、前端 production 构建、40 项前端测试及格式检查。原有 3 项管理员 ETW/Explorer 测试保持跳过。
- 本次应用明细交付：2026-10-04 从干净提交 `651b9f61d7f1a373d602f7d3821291af23a31c4e` 通过 `PINMETER_HOLD_DELIVERY=1 node tools/desktop.mjs build -- --locked` 完成 Windows release 与 NSIS 构建。入口为 `E:\dev\github\Pinmeter\src\backend\target\test5\Pinmeter.exe`，版本 0.1.3、56,818,688 字节。test1–3 占用无法确认，test4 正在运行，均已跳过；没有停止或覆盖旧实例。构建锁与 test5 目录预留保持至交付。
- 348 项源码指纹与 42 个运行文件的来源/目标 SHA-256 全部复核，安装器复制至 `test5/update-artifacts/` 后与构建源一致，Tauri/minisign 更新签名验证通过；记录在 `test5/delivery-verification.json`、`package-hashes.json` 和 `source-manifest.json`。EXE SHA-256：`33821baa5683b10ad00652199c9f4d2d2e8781976682eb20175c8cabdbffe683`；安装器 SHA-256：`3dd807f3342f0c5d251d7ba048933b4895402d8d9b623fbf1904add509fa7912`。完整 sensors/network/network-control/licenses 等伴随资源须保留，未推送或发布。
- 窄窗图片初次截取发生画面缩放异常，已在字体加载完成、DOM 确认 420×400 后重取并检查正确图像；补验使用已完成构建之后的独立缓存预览，未写共享 dist/Cargo 产物。预览已停止；`test5/qa-vite-cache` 的清理命令被自动审批策略拒绝，只返回“blocked by policy”，未提供细分原因，缓存保留且不计入运行资源清单。后续空闲编号目录复用可由项目入口统一处理。
- 构建完成后仅更新交付文档与窄窗截图，编译源码未改变。新版主程序因已有实例保留而未独立冷启动；管理员真实 ETW 归档、自动冷启动和长时精度仍未实测。使用时先完全退出旧版，再打开 test5；历史页点击“开始记录”，或主动开启下次启动自动记录。此前只有网卡汇总的时段不能补拆应用明细。

- 对照 Sakani 0.3.1 Stat Card、Select 和 Line Chart 变量；[浅色](assets/p1-light.jpg)、[深色](assets/p1-dark.jpg)与[窄窗](assets/p1-narrow-dark.jpg)已检查，分钟曲线跨未运行区间断开，统计范围与有效覆盖说明可见。HTML 只作布局参考。真实 24 小时持续运行、休眠与管理员主窗口尚未验证；运行包统一见 [desktop-runtime](../v0.1.0-desktop-runtime/execution.md)。
- 已实现 1441 个有界分钟桶、CPU/内存时间加权均值及原始网络计数差值；跨零点按时长分配并保留总字节。24 小时趋势、今日有效采集覆盖、所选网卡范围和保存状态已接入。
- 独立线程每分钟原子保存、退出排空后保存；队列最多 64 项，遗漏显示为不完整。损坏文件保留唯一备份，备份失败时保持原件并只在内存记录；文件读写限制 4 MiB。配置目录不可用、保存失败和时钟回拨均可见。
- 核心跨分钟/午夜、缺口、回拨、有界保留、原始计数接纳与重启/退出排空测试通过；文件原子替换/损坏保留通过。Rust 全工作区测试和 Clippy、前端类型检查与 33 项测试通过。浏览器视觉已核对；功能已包含在 `test2/Pinmeter.exe`，来源与完整资源哈希见 desktop-runtime 交付记录；真实 24 小时连续运行尚未验证。
