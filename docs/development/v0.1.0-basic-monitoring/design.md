# basic-monitoring

> 当前工作版本：v0.1.2

## 目标与范围

- 为首版主窗口提供真实的整机 CPU 忙碌时间、物理内存、选定单网卡上传与下载速率，以及仅保存在内存中的最近 5 分钟指标历史。
- 支持 1 / 2 / 5 秒采样间隔；本功能不含温度、风扇、GPU、进程、磁盘指标或桌面常驻入口。

## 方案

- v0.1.2 P0 回归：验证一小时采样断档后的读数过期、基线重建及首帧预热，避免将休眠累计流量当成瞬时网速；使用可控样本模拟八小时验证历史有界性。模拟结果只验证核心算法，真实休眠、进程树资源与八小时常驻仍需实机记录。

- CPU 详情补充逐逻辑处理器当前占用。Windows 在原 PDH 查询中加入 `Processor Information(*) / % Processor Time`，使用 [PdhGetFormattedCounterArrayW](https://learn.microsoft.com/en-us/windows/win32/api/pdh/nf-pdh-pdhgetformattedcounterarrayw) 读取，排除总计和处理器组总计，以组号、组内编号排序；每个 counter 独立检查状态。列表读取失败不影响总占用、内存和网络。macOS/Linux 复用原 sysinfo 刷新结果，保留其已知有效性限制。
- 平台返回项目定义的编号与逐项结果；核心验证范围、重复编号及有效期，持有唯一当前列表。列表仅在状态快照中传递，不复制进五分钟历史，避免核心数放大历史和订阅开销。整批失败保留已知编号并标记失效；成功刷新按最新编号替换，消失的处理器不残留。前端不从总占用推算单核负载。

- 获取当前快照时再次按共享单调时钟检查有效期；采样线程停滞时，重新订阅不能把旧的正常帧重新标为实时。历史保留采样当时的有效状态，当前快照过期不改写历史。

- S2 首轮按固定串行工作线程执行基础采集；使用单调时间、配置代次及网络基线代次，保留最多 301 个五分钟帧。订阅每窗口最多一个在途批次，ACK 后发送后端保留的增量；超时释放订阅，Bootstrap 返回完整保留窗口。
- 自动网卡优先选择活动物理接口，保持当前接口直到不可用，再按稳定 ID 排序选择；明确呈现实际选中接口，不宣称是系统路由推断。手动失联保留所选 ID。
- 依赖核验：windows 0.62.2、sysinfo 0.39.6；PDH 同时检查调用返回码和 CStatus，并使用 SDK 的 NOCAP100 标志，超出范围由核心拒收。[PDH 格式化](https://learn.microsoft.com/en-us/windows/win32/api/pdh/nf-pdh-pdhgetformattedcountervalue)、[内存 API](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex)、[网卡表与释放](https://learn.microsoft.com/en-us/windows/win32/api/netioapi/nf-netioapi-getiftable2)。
- sysinfo 0.39.6 的刷新接口无 Result；适配器检查能力、空 CPU/内存/接口，内存和网络使用新对象避免发布旧缓存。无法完整判定底层刷新失败是已知限制，macOS/Linux 在补齐刷新有效性验证前不声明达到首发采集验收。

- 指标与状态遵循[产品设计第 6 节](../../design/product-design.md#6-指标口径与数据状态)，验证遵循[第 11 节](../../design/product-design.md#11-质量目标与验收)；分层与协议见[技术架构](../../architecture/overview.md)，Windows 来源见[采集调研](../../research/windows-monitoring.md)。
- `src/backend/core/` 定义指标语义、逐项状态、网络差分、采样计划和有限历史；不导入系统 SDK、采集库具体类型或 Tauri。
- `src/backend/platform/` 封装真实采集：Windows 经 `windows-rs` 调用 PDH、`GlobalMemoryStatusEx`、`GetIfTable2`，分别检查返回值与 PDH counter 状态。
- macOS / Linux 从初期实现 `sysinfo` 基础 provider、能力探测和条件编译隔离，核验各自来源与有效性；构建通过与实机验证分别记录，未实测不声明平台支持。
- `src/backend/host/` 装配 provider、执行调度、校验命令并转换 DTO；采样轮次不重叠，不因超时持续新建任务或线程。
- 通用 DTO 以宿主 `contracts` 为唯一来源并生成 TypeScript 类型，携带会话、序号、设备、单位、来源、语义、状态及有效采样时间；64 位计数与序号跨语言使用十进制字符串。
- CPU 按全部逻辑处理器忙碌时间归一化到整机 0–100%，与 Utility 区分；适配器管理前后样本，预热时不生成零负载。
- 物理内存展示已用量、总量与占比；Windows 已用 = 总量 − available，其他平台保留缓存与可回收内存的口径说明。
- 网络读取选定接口的 64 位累计字节，核心用实际单调时间差计算 B/s；自动选择也只选一个接口，手选后不静默改选或叠加其他接口。
- 正常、预热、不支持、权限不足、失败、过期分别表示；仅有效新读数推进有效时间，超过期望间隔 3 倍无有效更新即过期，轮询完成不能刷新旧值时间。
- 休眠恢复、网卡切换或重连、计数回退及来源变更时重建相关基线并断开曲线；丢弃旧设备或旧配置代次的迟到结果，不跨边界算速率或伪造峰值。
- 每项指标只有一个生产来源；统一后端快照与 5 分钟内存环形历史，按时间及点数双重限额，退出释放，不落盘或填补断档。
- 推送和在途批次有界，消费变慢时合并最新快照；按架构的历史游标补齐仍保留的样本，超出范围标明缺失。
- 与 `main-window` 共用 `MonitorClient` 契约和 DTO 样例；其 mock 覆盖真实状态与恢复过程，仅供界面开发，正式采集不能回退到模拟数值。

## 验收

- 2026-09-16 首轮 Windows 对照后固定容差：CPU 每点绝对差不超过 5 个百分点，物理内存每点不超过 0.5 个百分点；网络同一 GUID 接口的 60 秒平均速率差不超过参考均值的 5% 或 4096 B/s（取较大值）。CPU 首轮最大差约 2.08 个百分点，内存约 0.152 个百分点；网络边界读取非原子，因此用完整窗口评定，同时保留逐点差供复查，不声称每个瞬间完全一致。
- .NET 参考实现返回接口收发累计字节，Windows 实现同样读取接口行 `inOctets/outOctets`，用于核对项目适配与差分计算；它不是另一套系统性能语义。[官方接口说明](https://learn.microsoft.com/en-us/dotnet/api/system.net.networkinformation.ipinterfacestatistics?view=netframework-4.8.1) · [微软参考源码](https://github.com/microsoft/referencesource/blob/main/System/net/System/Net/NetworkInformation/SystemIPInterfaceStatistics.cs)
- 参考环境为执行记录中的 Windows 11 普通用户机器，采样 1 秒；独立 C# PDH `% Processor Time`、`GlobalMemoryStatusEx` 和 .NET 同网卡累计字节作为参考。`tools/compare-windows.ps1` 保留数值与时间；`tools/run-comparison.ps1` 建立四线程 CPU、256 MiB 驻留内存或限速公开文件下载负载。冻结后重跑四场景，每场景至少 60 个有效对照点，检查脚本未通过时不判定准确性通过。

- 普通权限主窗口获得四项真实读数及逐项状态，1 / 2 / 5 秒均可切换；单项失败不阻断其他指标，关闭应用后采集资源释放。
- 确定性样例覆盖真实时间差、64 位计数、预热、计数回退、切网卡、休眠和迟到样本；PDH 错误或网卡移除不生成假的 0% / 100% 或连续曲线。
- 对空闲、持续 CPU 负载、内存分配与稳定网络传输分别保留至少 60 秒同来源、同设备、同口径、同时间窗的对照数据。
- 首轮对照冻结各指标数值容差、参考环境和理由后再评定误差；容差未冻结或对照未通过，不得勾选精准性验收完成。
- 三平台构建、核心测试与实际采集验证分别记录；发布构建测量主窗口及相关 WebView 进程资源，按产品第 11 节检查长时运行的缓存、内存、线程与句柄增长。
