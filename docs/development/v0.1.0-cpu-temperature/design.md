# cpu-temperature

> 当前工作版本：v0.1.0

## 目标与范围

- 根据用户要求，将 CPU 温度纳入当前版本，与 CPU 使用率在同一行等宽显示 Sakani StatCard，单位 °C、一位小数。CPU 趋势同时显示使用率与温度，保留最近 5 分钟历史；不增加单核温度图或告警。
- Windows 首先接入 LibreHardwareMonitorLib 0.9.6 的 CPU 传感器；其他平台明确不支持，后续按各平台来源扩展。原 CPU、内存和网络采样不受传感器阻塞影响。

## 方案

- CPU 页缺少 PawnIO 时提供“下载安装驱动”和“重新检测”。仅在用户点击后从 [官方 2.2.0 发行地址](https://github.com/namazso/PawnIO.Setup/releases/tag/2.2.0)下载固定安装器，校验 SHA-256 与 Windows 数字签名，再以固定静默参数安装并重新检测。已安装时不重复下载/安装；保留温度 worker 重试，检测到驱动不等于硬件一定提供有效温度。
- 下载、校验和安装封装在 platform/sensors，Rust 适配管理调用，宿主仅允许主窗口请求，View 经 ViewModel 与客户端操作。没有接受任意 URL、路径或参数的入口。使用 Sakani 0.3.1 的 [Button](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/core-button--primary) 和 [Alert](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-alert--info)，HTML 仅参考布局。
- 下载限定 HTTPS 官方发布/CDN 主机、最多 5 次跳转、16 MiB 和 120 秒；内容写入管理员专用临时目录，文件以不跟随重解析点的方式打开并拒绝符号链接，校验至安装完成保持禁止写入/删除的句柄，同时锁定目录避免替换。固定哈希为 `1F519A22E47187F70A1379A48CA604981C4FCF694F4E65B734AAA74A9FBA3032`。跨进程互斥覆盖完整操作；网络错误、损坏下载、签名失败及取消授权均明确失败，安装退出后才清理临时文件，不强制中断正在写驱动的安装器或主动重启电脑。
- 构建停止下载/复制安装器，清除受管辅助组件输出中的历史安装器；打包检查拒绝夹带该文件。保留实际使用的传感器库、嵌入模块及来源/许可信息，不卸载其他应用共享的驱动。

- 沿用[产品设计](../../design/product-design.md)、[架构](../../architecture/overview.md)和[Windows 调研](../../research/windows-monitoring.md)的可选独立传感器进程。Sakani StatCard 为视觉标准，HTML 仅参考布局。
- Windows x64 helper 放在 platform 所属 sensors 目录，用系统 .NET Framework 编译器构建，仅开启 LHM CPU 分组。CPU 优先封装温度或实际 Die 温度，再用 Tctl/Tdie、最后核心最高温度；缺失或无效值不能变成零。多 CPU 使用每颗 CPU 所选读数的最高值，任一缺失不发布部分成功。每次更新前清除固定版本 LHM 的温度缓存，避免跳过读取时复用旧温度。
- helper 通过继承的标准输入/输出与父进程通信，每次请求仅返回一个有界 JSON 结果；无端口、无任意硬件写入入口。启动时自动采集，继承启动入口统一取得的权限，不提供授权按钮或命令；统一启动及取消行为见 [desktop-runtime](../v0.1.0-desktop-runtime/design.md)。
- 父进程消失时 helper 退出；正常退出清理子进程。温度独立每秒请求一次，页面随既有监控批次刷新；独立 worker 与基础采集隔离。单次读取超时 3 秒；失败终止 helper 并退避 30 秒，不能无限积累线程或请求。
- platform 输出项目 Observation，core 维护唯一当前 Reading、摄氏度范围和 3 秒有效期；重复读取缓存不推进有效采样时间。宿主只装配、传递 DTO，ViewModel 保留失败、权限不足、缺失驱动及过期状态。
- Windows 本地构建包含 helper、固定版本 LHM 运行依赖及已收集许可文件；外部分发前仍须补齐依赖许可审查。构建产物放 backend/target，不提交二进制。Windows 主窗口在每次启动时统一提权。缺失 PawnIO 显示具体原因，安装官方签名驱动须由用户点击触发；启动和查询不触发安装，不切换到未签名版本。
- 本机 Ryzen 9 9950X3D；开始时未发现 PawnIO 或 LHM/OHM WMI 服务。不能用 ACPI 热区冒充 CPU 温度。LHM 的硬件读取依赖与行为以 [v0.9.6](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/tree/v0.9.6) 为准，驱动下载由 [PawnIO 官网](https://pawnio.eu/) 提供。

- 温度随基础历史帧保存采样时的有效状态和真实有效时间，复用 301 帧/5 分钟上限、游标、断档及恢复订阅机制；不回填未采集历史，不新增采集源。
- 趋势沿用官方 [Area Chart](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/charts-area-chart--default) 的两系列颜色、网格和提示卡，卡片参考 [StatCard Grid](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-stat-card--grid)。CPU 使用率左轴固定 0–100%，温度右轴以 °C 标识、默认 0–100°C，超界时按 20°C 扩展；两线共用时间轴，独立断档，悬停显示两项读数与单位。

## 验收

- 深浅主题下 CPU 使用率与温度同排均分（窄至无法容纳两张标准卡片时才换行）；无效温度为破折号和原因，不影响其他 CPU 图表。
- 测试有效温度、无效范围、真实零值、重复/过期读数、helper 缺失、超时和退出清理；DTO 与前端同步生成并通过相关检查。
- 保存原生缺失驱动与浏览器有效/异常截图；只有真实传感器读取和来源对照通过后，才完成实机温度验收。驱动未安装时明确保留未验证项。
