# main-window

> 当前工作版本：v0.1.0

## 目标与范围

- 先提供一个可查看基础指标、近期趋势及设置的主界面。
- 使用单个标题栏与应用主题一体化的主窗口，采用按平台适配的窗口控件和拖动区域；窗口生命周期由宿主负责，本功能负责界面与前端状态。
- 本版包含顶部拖动区域和最小化、最大化/还原、关闭控件；不包含磁盘、通用进程页、风扇、任务栏、悬浮窗和托盘。

## 方案

- 品牌标识：侧栏品牌区与 favicon 使用用户已选的橙色固定针/仪表图标，复用 desktop-runtime 导出的 PNG；保留品牌区原尺寸、拖动与布局，控件仍遵循 Sakani，HTML 仅用于布局。图标深浅背景和缩小效果随本次执行记录检查。

### 产品审核：运行状态、帮助与总览（2026-09-23）

- 所有页面提供紧凑的“运行状态”入口：连接中断、基础指标失败、CPU/GPU 采集异常、任务栏不可见、应用监控与网络控制问题按权威快照汇总，展开后给出原因和对应页面入口。预热和可选指标不支持不冒充故障；连接正常也不将部分失效指标宣称为全部实时。
- 全局状态面板可解除全部网络限制；即使 helper 当前不可用也允许发起恢复，复用原解除用例及修订检查，失败保留原因并可重试。退出处理中禁用操作。断线提供重新连接入口，自动恢复仍保留原退避。
- 标题栏提供“使用帮助”，说明开始应用监控、托盘恢复、关闭与真正退出以及网络限制的恢复位置；不抢占首次使用、不自动改动偏好。设置页提供构建版本、源码标识、构建时间与完整目录升级说明，具体维护在 preferences。
- 总览保留六张卡片，趋势默认显示同单位的 CPU/GPU 使用率与内存。通过 Sakani SegmentedControl 切换使用率、温度或全部，全部保留原双轴；图例仍可逐项隐藏，缺失值继续断线。由 ViewModel 保存当前总览页面内的选择，不增加采集或持久化配置。
- 视觉沿用 `@sakaniui/react` 0.3.1 的 [Alert](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-alert--all-colors)、[Popover](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-popover--one-button)、[SegmentedControl](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-segmented-control--three)、Badge/Button 的深浅与状态规范；HTML 仅供结构参考。核对全局状态、帮助、窄窗和趋势分组截图及键盘操作。

### 指标说明与 CPU 型号

- CPU、内存详情的“关于这项指标”默认只显示标题和信息图标。点击图标打开 Sakani Popover，完整保留统计范围、间隔、口径、来源和诊断；再次点击、点击外部或 Esc 关闭，键盘可操作。使用官方 IconButton ghost/sm 与 [Popover](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-popover--one-button)，窄窗不溢出；GPU 的现有说明暂不改变。
- 总览及 CPU 详情两张 CPU 卡片在正常状态下显示系统实际型号，异常时仍显示诊断。型号未知时显示“CPU 型号未知”，不写死示例型号。长型号允许换行，维持 Sakani 字号。
- 型号是启动时读取的设备元数据：平台提供普通字符串，经宿主装配存入核心 Monitor，快照 DTO 增加可空 cpu_model；不塞入每个历史帧，不增加高频采样或授权。Windows 通过 [RegGetValueW](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-reggetvaluew) 读取系统 CentralProcessor\\0 的 ProcessorNameString；macOS/Linux 复用 sysinfo CPU brand。Rust DTO 生成前端类型，ViewModel 提供统一展示值。
- 验收默认收起、点击/键盘/外部关闭与窄窗浮层，CPU 正常型号/未知/异常状态，以及 Windows 型号与系统查询一致。

### 总览六项指标与资源趋势

- 2026-09-17 按用户反馈取消网速额外加宽：顶部按 CPU 使用率、CPU 温度、GPU 使用率、GPU 温度、内存、网速排列六张等宽等高卡片，减少宽卡右侧留白。网速同卡分两行显示下载/上传，字号与其余读数一致。内容区足够宽时六列，常用宽度三列两行，再窄为两列三行或单列；以完整行对齐，不出现四列加两张宽卡。保留 Sakani 字号、内边距与卡片规格。
- 卡片内部保持左对齐，纵向均衡分布：标题靠上、说明靠下，数值在中间区域垂直居中；网速两行作为一个数值组。沿用 StatCard 字号、内边距和最小间距，只分配等高卡片的剩余纵向空间。
- 内存主值为使用百分比，说明显示已用/总量。网速说明包含当前网卡及自动/手动选择，移除重复的“当前连接”卡片。点击指标进入对应详情。
- 下方资源趋势独占整行，默认显示 CPU 使用率、内存、GPU 使用率，切换温度或全部时显示对应曲线。左轴为 0–100%，右轴为 °C（默认 0–100，按有效温度扩展），同一时间轴与悬停提示；图例可独立开关曲线，保留 1/5 分钟、历史拖动与返回实时。使用 Sakani 五种图表变量，减少五系列填充遮挡；缺失数据独立断线，不补零。
- 总览与 GPU 详情共用当前运行期间的显卡选择，使用率、温度及历史对应同一设备。默认沿用已有选择规则，设备移除时回退并清空历史锚点。选择仅为前端 ViewModel 状态，不新增设置或采集器。
- 前端新增 `features/overview` 的 View/ViewModel，复用现有快照和历史；图表支持明确的系列读取、颜色及温度轴定义，避免总览 CPU 与 GPU 共用字段而互相覆盖。后端协议和采样不变。
- 视觉依据继续使用下述官方 StatCard、Card、Select、Button、Area Chart；HTML 仅是布局参考。验收六卡顺序、长网速单位与卡宽、五系列/双轴/图例/提示、多卡一致性、缺温度与断线，以及深浅主题和宽窄窗口；截图放原 assets。

### 既有窗口与控件

- 正式实现采用 React feature ViewModel、共享 MonitorClient 与有界只读缓存；页面结构参考 HTML，视觉、基础控件与主题严格遵循 [Sakani 官方标准](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/sakani-design-system--docs)，按页面和图表组件实现。
- 视觉整改固定 `@sakaniui/react 0.3.1`、`lucide-react 1.46.0`、`@fontsource-variable/geist 5.3.0`。npm 0.3.1 包遗漏导出的 tokens 文件，因此从官方源码 `0c9a97359001299a1e917ef67fdd0d5af99023a7` 原样保留 tokens.css 与 MIT 许可证于前端 shared/ui/sakani，控件仍直接复用官方包，字体本地打包。其余传递版本由锁文件固定。
- 选用官方 [SidebarItem](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-sidebar-item--all-states)、[StatCard](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-stat-card--grid)、Card、Button、Radio、Select、Alert、Badge 和 [SegmentedControl](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-segmented-control--two)。HTML 仅用于五页组成、四项指标、趋势和当前连接等区域安排；移除原自定义蓝色控件体系，按 Sakani 默认中性色、品牌标记、Geist 和 Lucide 组合页面。
- 业务 SVG 对照 [AreaChart / Single Series](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/charts-area-chart--single-series) 与 Linear、Dark Mode：使用 chart/5 与 chart/2、官方渐变透明度、网格、圆点、定位线和提示卡规格。保留真实时间横轴、固定百分比纵轴、分段断档、拖动与键盘查看；不平滑真实峰值，不播放采样插值动画，悬停只命中可见有效样本，全部失效时显示明确空态。
- 主题以持久化设置为唯一来源：React 同步 `.dark` 与 color-scheme，标题栏和内容消费同一套 Sakani 变量，宿主同步原生窗口主题，启动和切换均生效。system 由原生窗口解析；客户端适配器读取窗口主题并监听其变化，内容消费相同结果，避免 WebView 的 prefers-color-scheme 与窗口不一致；无宿主的演示才直接查询浏览器。控件样式不重写；业务布局与图表只消费官方变量。小尺寸依靠重排与滚动而非缩小官方控件文字。
- SVG 按实际时间定位、最多 301 个点；提供 1/5 分钟范围与拖动查看历史、返回实时，缺失样本和设备代次变化断线。演示入口限开发构建并持续显示标识；生产构建无法连接宿主时显示不可用。
- 验证分为浏览器的页面/主题/等效缩放/异常交互与原生 WebView2 的真实数据、保存和订阅恢复；原生 DPI、多屏结果单独记录，不能用 CSS 视口检查替代。

- 布局与视觉遵循 [产品设计](../../design/product-design.md) 第 5.3、5.4 节；初始尺寸建议为 880 × 600 逻辑像素，可按实际可读性调整。
- 轻量侧栏提供总览、CPU、内存、GPU、网络、设置页面；窄窗口和文字放大时调整布局。
- 总览按上文显示六张指标卡及五项资源趋势，历史范围为最近 1/5 分钟。
- CPU 页依次显示总占用、趋势、逻辑处理器柱状图和指标说明；统计范围与采样间隔放在页底。柱状图只显示当前快照，不随总占用历史浏览变化。
- 柱状图以官方 [Bar Chart / Default](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/charts-bar-chart--default) 与 Dark Mode 为形状、网格、字体及提示卡标准；按用户要求沿用 Heatmap 的 chart-1 橙色和固定 0.12–1 色强度。保留固定依赖 0.3.1；官方 BarChart 未提供固定纵轴、异常状态和滚动刻度接口，在 monitoring 内沿用官方变量实现业务绘制。HTML 仅参考布局，不提供视觉标准。
- **排列与刻度**：每个逻辑处理器对应一根竖柱，按后端稳定编号从左到右单排，不因负载排序。图表高 320px，每项至少 64px 以容纳完整编号和一位小数百分比，柱宽最大 48px；窗口较窄或数量较多时仅图表横向滚动，不换行、不挤压文字。左侧 0/25/50/75/100% 刻度固定，底部 CPU 编号与柱同步滚动；支持触控板、底部滚动条及键盘左右/Home/End 定位，刷新保留滚动位置。
- **占用与交互**：柱高和橙色浓度均按固定 0–100% 映射，深色主题高占用更鲜明；柱顶常显百分比，低占用仍能读数。整个编号列可悬停或聚焦，提示卡显示完整编号、实时占用或异常原因；鼠标静止时继续刷新，不通过悬停改写数据颜色。正常 0% 显示 0.0% 且柱高为零；无效读数不绘制柱，显示破折号与状态，不假装正常零值。没有采样插值动画。
- 本次复用后端当前快照，不增加采集、单核历史、物理核心拓扑；CPU 温度由 [cpu-temperature](../v0.1.0-cpu-temperature/design.md) 提供，与 CPU 使用率同排均分显示，并加入 CPU 趋势。
- 网络页显示单个选定网卡的名称、上下行速率、趋势与单位；不混加物理网卡、VPN 和虚拟接口。
- 设置页只承载 [preferences](../v0.1.0-preferences/design.md) 定义的控件与交互，配置规则、保存及生效状态由该功能负责。
- 数据与历史来自 [basic-monitoring](../v0.1.0-basic-monitoring/design.md)；前端不采集、不重算指标，不另建权威状态。
- 按 [架构](../../architecture/overview.md) 的 MVVM 分工实现 View、feature ViewModel 和统一客户端；订阅、恢复及有界缓存沿用其第 6 节协议。
- 先通过明确标注“演示数据”的可控 mock 验证完整页面和状态，再接真实数据；mock 不作为功能完成依据。
- 正常、采样中、不支持、权限不足、失败和过期分别呈现；图表保留断档，不以零值或插值掩盖缺失。
- [离线 HTML 布局预览](assets/main-window-preview.html) 仅用于五个页面的结构、信息层级和区域安排；自定义颜色、字体、间距、控件和 SVG 风格均不作为视觉依据，组件规格以 Sakani 为准。
- HTML 仅含演示数据，设置仅作用于预览；它不证明正式功能、采集准确性或 Sakani 视觉验收通过。

### 标题栏一体化

- **集成方案**：主窗口仅保留一层顶部操作区，平台装饰、窗口按钮、拖动区域和主题由既有宿主与前端窗口桥接协作。普通设计参考统一见项目 README。
- **平台策略**：按平台区分：Windows 隐藏系统标题栏，由应用绘制窗口控件；macOS 隐藏标题文字和标题栏背景并保留原生红绿灯，为按钮预留空间；Linux 采用应用绘制控件，桌面环境与窗口管理器行为单独验证。使用 Tauri 对应平台配置实现，不将一份全局无装饰配置用于所有平台。
- **顶部布局**：去掉独立系统标题条，侧栏品牌区延伸到窗口顶部；复用现有“此电脑 / 当前页”顶栏，右侧安排实时状态和窗口按钮。窗口按钮固定在窗口右上角，页面切换、内容滚动和侧栏折叠不改变其位置；顶栏预留按钮空间，小窗口仍可操作。主窗口只保留一层顶部窗口操作区域。
- **拖动与操作**：顶部空白及适用品牌区域承担窗口拖动，按钮和其他交互区域排除拖动；双击空白区最大化/还原。Tauri 使用 `data-tauri-drag-region` 或客户端封装的窗口拖动接口。最大化状态从实际窗口读取并监听变化，按钮名称与图标反映最大化/还原。拖动限于指定空白区域，不影响交互控件。
- **视觉边界**：一致性指窗口结构、平台分流、固定按钮、拖动和主题共用机制。颜色、字体、间距、控件尺寸与状态仍遵循 Sakani，顶栏尺寸随标准组件和平台布局确定。控件参考官方 [Icon Button / All Variants](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/core-icon-button--all-variants)、[Dark Mode](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/core-icon-button--dark-mode) 与 [Top Bar / Compact](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/navigation-top-bar--compact)、[Dark Mode](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/navigation-top-bar--dark-mode)；实施时核对默认、悬停、焦点和禁用状态。HTML 仍只提供页面布局参考。
- **职责与生命周期**：View 只呈现与接收输入，ViewModel 组织窗口状态及操作，客户端适配器封装 Tauri API；宿主负责窗口配置、权限和原有生命周期。关闭按钮与系统关闭统一使用 Pinmeter 的关闭选择与退出清理，最小化到托盘继续采集并暂停界面更新；不引入会话确认或多窗口逻辑。宿主工作见 [desktop-runtime](../v0.1.0-desktop-runtime/design.md)，分层见 [技术架构](../../architecture/overview.md#22-前端与本地后端边界)。
- **当前实现**：app 内的 WindowControls / useWindowViewModel 与 shared/client/window-client 分开呈现、状态和框架调用。宿主在创建窗口前按平台调整配置；前端先读取已确认主题，在布局阶段应用主题后显示窗口。窗口按钮直接复用 Sakani IconButton ghost/sm，右侧与顶栏同高固定；窄窗口收起顶部状态标记，底部实时状态仍保留。Windows 11 实测支持拖动、最大化后拖动还原、边缘缩放及 Win+方向键贴靠；自绘最大化按钮暂不提供原生悬停贴靠面板，此项不计为通过。当前状态与剩余验证见 execution.md。

## 验收

- 按上述方案检查平台分流、顶部融合、按钮固定和拖动排除；对照 Sakani 保存标题栏深浅主题及控件状态截图到本功能 assets，并在 execution.md 记录结果。原生标题栏的旧主题截图不能证明一体化完成。
- 浅色、深色、跟随系统、启动恢复和系统运行中切换主题时，顶部与内容共用主题，无独立系统标题色带；检查首次显示是否闪白。窄窗口、侧栏折叠和页面切换不遮挡窗口按钮。
- 在 Windows 实机验证拖动、双击最大化/还原、最大化后拖动还原、边缘缩放、贴靠、最大化按钮悬停布局、Alt+F4、键盘焦点及按钮可访问名称；核对 100% / 150% / 200% 和混合 DPI。各平台实际系统行为存在差异时记录差异与处理结果，未解决项不标通过。
- 最小化/恢复、关闭退出、尺寸位置及最大化状态恢复回归通过；macOS 红绿灯/全屏及 Linux 各目标桌面环境分别实测，缺少环境明确标为未验证。

- 对照选定的 Sakani 组件和示例检查五页深浅主题、字体、颜色、间距、尺寸、圆角、边框、阴影、图标、图表及适用交互状态；来源与截图可复核，结果写入 execution.md。视觉整改及对照未完成时不标为界面验收通过。
- 五个页面可切换，主要操作支持键盘；深浅主题、100% / 150% / 200% 缩放及文字放大后可读，读数更新不引起明显布局跳动。
- 总览与详情消费相同有效样本，单位及状态一致；曲线有点数上限，页面切换不重复创建采样源或订阅。
- 最终验收使用真实 CPU、内存及选定网卡数据；演示数据始终可辨识，不用于证明采集准确性。
- 初次采样、失败、过期及断档可复现；恢复订阅后按协议补齐可用历史，旧会话数据不接入新曲线。
- 设置页展示真实保存结果和实际生效状态，不把表单草稿当作已确认配置；具体设置验收引用 preferences。
