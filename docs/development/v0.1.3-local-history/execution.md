## 任务计划
- [x] 将应用流量已接纳增量接入有界分钟历史，验证归并、去重、跨日及缺失语义。
- [x] 增加应用列表/筛选/排序/趋势、共享记录开关及启动记录偏好。
- [ ] 验证文件恢复、UI 状态与完整 Windows 构建，记录本次交付并本地提交。
- [x] 实现分钟聚合、精确流量增量和 24 小时保留。
- [x] 实现后台持久化、重载与历史页。
- [x] 验证断档/跨日/损坏恢复及深浅主题，构建并交付完整运行包。

## 进度
- 应用流量历史已接入，旧网卡汇总不回填明细。按现有路径身份归并已接纳的窗口增量，去重、跨分钟/午夜、停止再开始、采集重建、丢事件/队列窗口与超限归并均有覆盖；最大 128 应用 × 1441 分钟的极值序列化仍小于 16 MiB。基础历史文件兼容，应用文件独立原子保存、损坏备份及退出排空/重载通过测试。
- 界面：今日/1/6/24 小时、名称/路径筛选、下载/上传/合计排序、单应用趋势、两页共用启停与默认关闭的启动记录偏好均已接入。沿用用户当前字体设置，Sakani 0.3.1 Table/Button/Switch/Input/Select/Alert 决定控件和主题，HTML 只作布局参考。已检查[浅色明细](assets/applications-light.jpg)、[深色明细](assets/applications-dark.jpg)、[同名筛选](assets/applications-filter-light.jpg)、[浅色趋势](assets/application-trend-light.jpg)、[深色趋势](assets/application-trend-dark.jpg)、[420×400 控件](assets/applications-narrow-dark.jpg)、[窄窗趋势](assets/application-trend-narrow.jpg)、[空态](assets/applications-empty.jpg)及[读取失败](assets/applications-query-failure.jpg)。未记录时不显示正常零流量；窄窗页面滚动宽度与可视宽度相等。
- 浏览器演示确认共享开始/停止、暂停后保留明细、启动偏好即时保存、上传排序和同名不同路径不混合；修正网络表头 portal 挂载后，往返切页控制台无错误。真实管理员 ETW 归档、自动冷启动、代理/VPN、休眠和 24 小时实机长测尚未验收，模拟与浏览器截图不代替这些结论。
- `tools/dev.ps1 check` 完整通过：347 个文档链接与分层/忽略检查、helper 和构建互斥/卸载回归、Rust 工作区测试与 Clippy、契约同步、前端 production 构建、40 项前端测试及格式检查。原有 3 项管理员 ETW/Explorer 测试保持跳过。已检测到 test4 正在运行，完整发行构建将避开该目录；本次新运行包记录待构建完成后填写。

- 对照 Sakani 0.3.1 Stat Card、Select 和 Line Chart 变量；[浅色](assets/p1-light.jpg)、[深色](assets/p1-dark.jpg)与[窄窗](assets/p1-narrow-dark.jpg)已检查，分钟曲线跨未运行区间断开，统计范围与有效覆盖说明可见。HTML 只作布局参考。真实 24 小时持续运行、休眠与管理员主窗口尚未验证；运行包统一见 [desktop-runtime](../v0.1.0-desktop-runtime/execution.md)。
- 已实现 1441 个有界分钟桶、CPU/内存时间加权均值及原始网络计数差值；跨零点按时长分配并保留总字节。24 小时趋势、今日有效采集覆盖、所选网卡范围和保存状态已接入。
- 独立线程每分钟原子保存、退出排空后保存；队列最多 64 项，遗漏显示为不完整。损坏文件保留唯一备份，备份失败时保持原件并只在内存记录；文件读写限制 4 MiB。配置目录不可用、保存失败和时钟回拨均可见。
- 核心跨分钟/午夜、缺口、回拨、有界保留、原始计数接纳与重启/退出排空测试通过；文件原子替换/损坏保留通过。Rust 全工作区测试和 Clippy、前端类型检查与 33 项测试通过。浏览器视觉已核对；功能已包含在 `test2/Pinmeter.exe`，来源与完整资源哈希见 desktop-runtime 交付记录；真实 24 小时连续运行尚未验证。
