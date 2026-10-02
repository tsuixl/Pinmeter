# desktop-runtime

> 当前工作版本：v0.1.2 · 状态：实现中，验收以 execution.md 为准。

## 目标与范围

- 为主界面优先的首版提供工程骨架、主窗口生命周期、持续集成与 Windows 安装包。
- 界面放 `src/frontend/`，本地后端按 `src/backend/{core,platform,host}/` 分工，共享数据放 `src/shared/`；工作目录与依赖方向遵循 [技术架构](../../architecture/overview.md) 第 7、8 节。
- 基础指标与真实 provider 由 [basic-monitoring](../v0.1.0-basic-monitoring/design.md) 负责；本功能完成装配和生命周期联动，主界面内容由 [main-window](../v0.1.0-main-window/design.md) 负责。
- 关闭主窗口默认询问最小化或退出，支持记住选择；与任务栏显示开关独立。悬浮窗后置；2026-09-18 开机自启纳入 [preferences](../v0.1.0-preferences/design.md)，默认关闭，开启后用当前用户最高权限登录任务启动，沿用主窗口和单实例行为。

## 方案

- v0.1.2 在线更新共用退出准备；NSIS 写入与实际 EXE 路径绑定的安装标记，便携和受管测试包不在线覆盖。安装器启动失败重建已停止的采集和托盘；包签名与公告由 [app-update](../v0.1.2-app-update/design.md) 维护。P0 常驻及环境恢复验收继续在本功能执行记录推进。

- v0.1.1 图标更新预览发布：同步前后端、宿主和请求标识版本；保留现有预览状态及功能验收边界。推送发布准备提交并等待该提交的三平台 CI，通过后发布指向同一源码的标签及 Release。Windows 使用项目入口从干净提交构建，提供安装器、由运行文件清单生成的完整便携 ZIP、`git archive` 对应源码 ZIP 与 SHA-256 清单；先上传草稿并校验远端附件，再发布并核对匿名下载。此次用户明确授权版本发布，包含必要的推送和新标签，不覆盖旧版本。

图标设计稿：[pinmeter-icon-concept-v1.png](assets/pinmeter-icon-concept-v1.png)。参考 [Sakani 官方文档](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/sakani-design-system--docs)及仓库 tokens.css 的中性色 `#141414`、品牌橙 `#FF4700` 和暖白 `#FAFAF9`；生成像素不保证严格等于设计变量。

<details>
<summary>图标设计稿 v1 完整生成提示词（内置 imagegen，transparent_background=true）</summary>

```text
Use case: logo-brand.
Asset type: a finished standalone desktop application icon for Pinmeter, also used unchanged for its Windows installer.
Product: a small, precise, clean system-performance monitor. Pinmeter means "Pin + Meter", a little instrument pinned to the corner of the screen.
Primary request: design one strong, original symbol fusing a pushpin and a performance gauge. A compact, almost circular orange instrument head, a short broad tapering pin stem pointing downward, and a very simple gauge inside the head: dark negative-space dial, one thick warm-white partial arc and one bold warm-white needle pointing upper-right. The orange outer silhouette suggests a pinned instrument, not a navigation/location app. Integrate the parts into one cohesive logo, not separate pictograms. Generous negative space, bold geometry.
Composition: single icon centered, straight-on, filling about 84 percent of a square canvas; a dark charcoal rounded-square app tile behind the symbol, with consistent generous inset; completely transparent outside the rounded tile. Tile corners soft and precise. The inner symbol occupies roughly 68 percent of tile width.
Style: premium minimal flat geometric software identity, crisply antialiased, confident and quiet, compatible with Sakani UI's visual language. Very restrained subtle edge definition only, no glossy plastic or dimensional mockup.
Color palette based on project Sakani tokens: charcoal #141414 tile, vivid orange #FF4700 symbol, warm white #FAFAF9 gauge needle/arc. Keep colors flat, high contrast.
Small-size requirements: clearly recognizable at 32x32, thick shapes, wide gaps, no tiny ticks, no fine hairlines.
Constraints: exactly one icon, transparent exterior, no text, no letters, no numbers, no badge, no installer box, no download arrow, no screenshot, no presentation board, no watermark, no texture, no cast shadow outside the tile, no blue/purple gradient, no map-marker hole. Create a clean 1024x1024 master.
```

</details>

- 应用与安装图标：用户已采用上述设计稿。保留原稿，使用锁定版本的 Tauri CLI 导出宿主 PNG、Windows 多尺寸 ICO 与已有其他平台格式；移除已被替换且无引用的旧 SVG。应用窗口、托盘和 NSIS 安装器使用同一标识，并显式配置安装器图标；前端品牌区和 favicon 复用导出图片，中英文 GitHub README 展示同一图标。只更新仓库内展示资源，不修改个人头像，不自动 push 或发布。Sakani 是视觉与组件标准，HTML 预览仅提供布局结构。检查小尺寸与深浅背景并保留预览，构建 Windows release/NSIS，分别记录资源核对和实际运行边界。

- 首次预览发布：先核对远端 CI，修复托管 Windows 的 PowerShell 模块搜索路径及非 Windows 条件编译问题；Windows PowerShell 子进程使用系统内置模块目录，避免继承 PowerShell 7 模块而丢失命令。通过 CI 后，从修复后的干净提交重新构建 Windows 预览包，发布标签、二进制和源码附件对应同一构建提交。GitHub Release 标记为预览版，说明未完成的实机验收；上传并核对全部附件后再发布，不把本地校验代替远端 CI。

- 首版收尾：保留现有服务图标与声明；补齐固定依赖的 UNIC 版权。Windows 项目入口增加共享的辅助组件准备操作，本地检查和 CI 在宿主测试前调用，继续遵守构建互斥。卸载使用明确的卸载清理命令：网络规则清理并复查后，仅移除指向本安装 EXE 的 Pinmeter 自启任务；缺辅助程序、清理失败或验证失败均中止卸载并保留应用文件，同目录升级保留原规则与自启。自动回归使用隔离夹具，不操作用户日常应用的网络规则。最终从已提交源码构建 Windows 运行包与安装包，记录适用的启动、资源及卸载检查；公开仓库和发布版本另行执行。

- 2026-09-28 第三方材料简化补齐：沿用现有 `licenses/` 与传感器许可目录，核对 Windows 构建所用 npm/Cargo 依赖及实际分发 DLL 的版本、上游版权和许可原文，集中生成第三方声明；来源信息指向准确版本或提交，保留已有源码包。所需材料随现有资源映射打包，不新增发行平台或统一要求源码附件。来源或对应源码尚不能证实的项目如实保留待核对，不把告知文件存在当成许可完成。

- 2026-09-28 AGPL 落地：Tauri 通用配置声明 `AGPL-3.0-only` 与根 LICENSE，把项目法律声明、源码获取说明及已有 one-ip/Sakani/Geist/品牌告知映射到运行包 `licenses/`；补充文本归属 `host/resources/legal/`。Windows 整理器合并通用和平台资源映射，必需许可缺失或被损坏时拒绝交付，逐项检查路径及哈希。构建输入清单纳入根 LICENSE 和 README，避免许可修改不被识别。公开版本仍须另行提供匹配 EXE 的完整对应源码、构建脚本及所需第三方源码；本批提供声明与说明，不把仓库地址等同于已完成公开源码交付。

- 2026-09-23 产品审核整改：性能测量显式指定本次 `testN/Pinmeter.exe`，记录主程序哈希、整个进程树（包括提权宿主、WebView、全部 helper）的 CPU、私有工作集、线程和句柄。支持只读附加现有实例，按实际可见/托盘状态分组；不将旧版、短时检查或受干预的窗口测量充当新版十分钟基线，不强停用户实例。
- 本轮按网络可靠性、性能基线、应用监控连续性、全局恢复入口、常驻使用提示与版本信息、总览可读性的顺序完成原审核建议。各功能的实现、验证和剩余环境边界回写原目录，最终统一构建完整运行包。

- **构建交付自动化（2026-09-22）**：Windows 项目入口使用进程持有的跨进程构建互斥锁，覆盖 helper、前端、Rust 和运行包核对；helper 独立入口使用同一把锁。release 自动从 test1 起取得目录独占，跳过运行中、无法确认或未确认归属的目录；仅清理已核验且无联接/符号链接的受管目录。按资源配置复制并核对路径与 SHA-256，记录源码清单和构建来源；失败保留证据且不提供旧 EXE。交付预留可延续到调用方明确释放，锁文件只作信息记录。

- **跨电脑测试包（2026-09-18）**：通过项目入口完整构建 Windows x64 release 与 NSIS，先在独占的 `target/testN` 核对运行资源，再新建 `src/backend/target/releases/` 下的独立版本目录。提供完整便携 ZIP、安装包、中文测试说明及 SHA-256 清单；不包含本机配置、日志或源码绝对路径。压缩包按源清单逐文件校验，说明运行依赖、启动授权及尚未完成的跨电脑验收；仅本地交付，不上传发布。
- 指定 `CARGO_TARGET_DIR` 时必须使用绝对路径，使 Tauri/Cargo 实际输出与打包入口定位一致；入口会拒绝相对环境路径。默认使用 `target/desktop-build` 作为共享缓存，运行包整理到 testN 后再启动。便携包附对应受控源码与构建脚本，以及已有依赖来源和许可证。

- **Windows 运行包资源校验（2026-09-18，2026-09-22 接入自动预留）**：修复 test8 手动整理时因源目录末尾斜杠加固定长度截取而丢失文件名首字符的问题。项目构建入口支持 `--package-dir`，构建成功后按 Windows 资源映射整理到独占预留的 testN；用路径相对关系保留完整文件名和嵌套目录，禁止字符串长度截取。复制前检查 CPU/GPU/网络/网络控制入口与传感器依赖；复制后同时验证相对路径集合、关键入口实际存在及 SHA-256，而非只对同一错误映射核对内容。构建锁、选目录与安全复用由 Windows 协调脚本完成，旧目录无归属标记时保守跳过。
- 打包工具与回归测试归属 `tools/`；测试覆盖带/不带末尾斜杠、嵌套许可证、单文件映射及丢首字符的损坏运行包。交付前从实际 testN 中启动 CPU/GPU helper 读取真实指标；不停止用户正在运行的旧实例。

- **关闭选择（2026-09-17 新要求，覆盖原自动收起/退出规则）**：关闭按钮、Alt+F4 与系统关闭首次均弹出 Sakani 对话框，提供“最小化”“退出”和默认不勾选的“记住我的选择”。最小化收起到系统托盘，隐藏窗口及任务栏按钮，后台采样继续；“最小化到托盘”使用 Sakani 主按钮，“退出”使用次按钮。托盘从启动起独立于任务栏直显建立，提供恢复和退出入口；确认托盘可用后才隐藏，失败保持窗口并说明原因。标题栏最小化按钮复用同一收起用例；退出复用网络清理流程。勾选后持久化为 close_action（ask/minimize/exit），旧配置默认 ask，可在设置页改回每次询问。不勾选仅执行本次选择；取消/关闭对话框不执行动作。保存失败保持窗口及对话框，不执行退出。托盘及设置中的明确退出不重复询问关闭方式。


- 2026-09-16 用户确认 Windows 每次启动统一请求一次 UAC，授权后主程序与 CPU/GPU/网络子进程共用管理员权限；已提权启动不再请求。启动适配放 `platform/src/startup.rs`，宿主在创建窗口、配置和采集器前调用；原启动进程等待授权实例退出，兼容开发命令生命周期。取消时结束本次启动，不循环请求；其他启动错误明确报告。macOS/Linux 不改变启动权限。
- 本次替换原“普通主窗口、按功能提权”约束。不保存系统授权、不创建服务；开机自启的用户登录任务由 preferences 单独管理。CPU 温度默认采集并移除授权按钮和命令；GPU 自动采集，网络按明确开始/停止控制 ETW，切页与托盘继续采集。helper 直接继承权限，失败重启也不二次授权。缺少驱动仍报告原因，不自动安装。
- 开发重编译或启动器异常退出时，授权实例经已有退出流程清理采集器。调用系统启动接口前初始化 COM，保留参数转义和工作目录，取消授权直接退出。实现依据：[微软 ShellExecuteExW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecuteexw)。
- 验收启动接受/取消、已提权启动、自动采集、网络停止重开、退出清理和开发/生产构建。标准账户输入另一管理员凭据时应用以该管理员身份运行，设置使用该身份目录，不共享原账户配置。

- 主窗口标题栏一体化遵循 [main-window 的标题栏方案](../v0.1.0-main-window/design.md#标题栏一体化)：Windows/Linux 使用应用绘制的窗口控件，macOS 隐藏标题栏背景和文字并保留原生红绿灯。宿主在创建窗口前调整配置，不能全平台统一关闭原生装饰；前端读取已确认主题并完成首个布局后显示窗口。Windows 已实测，其他平台待实机验收。
- 宿主在首次显示前及已确认设置变化时同步原生窗口主题，system 交还系统；启动背景与已确认主题协调，前端顶部和内容共用 Sakani 变量。Tauri 窗口操作由前端客户端适配器封装，按主窗口范围配置最小化、最大化/还原、关闭和拖动所需权限；窗口状态以系统实际结果为准。实现依据见 [Tauri 窗口定制](https://v2.tauri.app/learn/window-customization/)。
- 自绘关闭按钮、Alt+F4 和系统关闭入口沿用同一关闭选择流程；最小化/恢复仍沿用订阅及低内存策略。关闭选择使用 Sakani Modal 和 Checkbox，HTML 预览仅作为主窗口结构参考。Windows 特有行为若需原生补充，按架构放入平台适配，由宿主桥接，不将系统类型传入核心。

- S0 使用 React 19、Vite 8、Tauri 2；npm 与 Cargo 锁文件固定实际解析版本。开发脚本显式定位工程；CLI 由前端开发依赖锁定，通过脚本在 host 目录调用，避免全局 CLI 漂移。
- 宿主 contracts 通过 ts-rs 导出受控 TypeScript；协议 v1 保留逐项状态、来源、语义、有效时间、设备与配置代次。先交付明确不可用的工程入口，再在 S1 加入仅开发模式的演示客户端。
- S3 使用官方 single-instance 2.4.4 与 window-state 2.4.1：只记忆位置、尺寸及最大化，不恢复隐藏/最小化状态。启动与显示环境变化时校验窗口和当前工作区交集，必要时移回可见范围；手动恢复入口复用宿主桥接。
- 调试验证可显式设置 WebView2 的本机调试端口；发布配置不打开开发工具、不内置调试端口。安装包使用当前用户 NSIS，不需要管理员权限；签名和对外发布另行决定。
- 协议导出及采样探针二进制要求显式 `dev-tools` feature；开发检查启用，正式打包不启用，避免 Tauri 自动把 `src/bin` 诊断程序作为附加二进制分发。[固定 CLI 的二进制筛选](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-cli/src/interface/rust.rs#L952)
- Windows 工具链采用 Rust MSVC、既有 Visual Studio C++ 与 WebView2；依赖依据 [Tauri 前提](https://v2.tauri.app/start/prerequisites/)、[Vite 环境要求](https://vite.dev/guide/) 和包注册表核对，实际版本及运行结果写入执行进度。

- 使用三个 Rust crate 与独立前端工程；core 保持纯逻辑，platform 隔离系统依赖，host 仅装配、执行任务并桥接 Tauri 窗口和通信。
- 单实例运行；用户重复启动时激活现有主窗口。采样刷新及睡眠恢复不主动抢焦点。
- 保存主窗口尺寸、位置及显示器信息；显示器断开或工作区变化后移回可见区域，保留手动恢复入口。
- Windows 应用最小化入口收起到托盘，保留有限的基础采集历史，解除 UI 订阅并停止绘图；恢复时重新订阅快照和有限历史，按既有协议补齐或标注缺口。
- Windows 最小化时通过宿主 WebView 框架桥接请求低内存模式，恢复时切回正常模式；使用 `ICoreWebView2_19::SetMemoryUsageTargetLevel`，运行时查询接口，旧版本失败时保留原行为并记录诊断。仅作为浏览器资源提示，不停止 Rust 采样、不丢弃历史；需实测内存与恢复延迟。[微软接口说明](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_19?view=webview2-1.0.3800.47)
- 选择明确退出时停止采集与执行器、释放订阅及平台资源并退出，不留下后台常驻进程。
- 退出采用两步处理：先请求采样停止，保持事件循环运转，在另一线程等待采样结束后再退出。窗口查询会等待主线程回执，不能在主线程直接 join 采样线程而形成互等。
- 从 M0 开始建立 Windows、macOS、Linux 核心测试和基础构建检查；真实基础 provider 与 `basic-monitoring` 联合接入，缺失能力明确报告，不能用无条件成功的空壳通过验收。
- 仅 Windows 纳入首发实机发行验收。当前参考系统为 Windows 11 专业版、10.0.26200、64 位；已执行的应用验证见 execution.md，不据此宣称其他 Windows build 或 CPU 架构受支持。
- S4 同步首页启动说明与文档状态，使用发布程序进行资源测量，并在独立当前用户目录验证 NSIS 安装、启动、退出和卸载；保留原始测量数据，未完成的长时及设备测试继续列明。
- S0 先固定参考机器、系统、构建版本、依赖、测量工具和全进程聚合口径，测出基线后冻结预算及适用条件；差异与未验证环境记录在执行进度。

## 验收

- 标题栏平台配置与 main-window 约定一致，Windows 标题栏一体化后的拖动、双击、最大化后拖动还原、边缘缩放、贴靠和最大化按钮悬停布局逐项实测；记录框架差异，不默认视为保留原生行为。macOS/Linux 未运行的窗口行为保持未验证。
- 自绘控件与系统入口均能最小化、最大化/还原及关闭，窗口状态、尺寸位置记忆、启动主题和退出资源释放回归通过；视觉截图与控件状态由 main-window 记录。
- 工程按约定目录可安装依赖、检查与构建，三平台核心测试和基础装配通过；平台依赖隔离成立，产物不散落仓库根。
- Windows 普通启动经一次系统授权后可打开主窗口；单实例、关闭选择、记住及重设选择、最小化恢复、明确退出及再次启动正常，关闭后无应用相关残留进程。
- 完成尺寸位置记忆、可见区域恢复、100% / 150% / 200% 与混合缩放、多屏断开重连及睡眠恢复检查，记录每个已测组合。
- 最小化后没有 UI 推送或绘图，基础历史仍有上限；恢复后读数与历史状态正确，不增加第二套采样任务。
- 发布构建关闭开发工具，预热 2 分钟后每种状态测量 10 分钟；统计应用、WebView 等全部相关进程，避免共享内存重复计数，记录平均 CPU、分位数、峰值及稳定内存。
- 参考机预算固定为：主窗口可见时平均整机 CPU ≤ 1%、稳定内存 ≤ 180 MiB；最小化时 ≤ 0.5%、≤ 120 MiB。CPU 使用整个应用进程树的 CPU 秒差分除以实际时间和逻辑处理器数；内存使用进程树私有工作集之和，不包含共享页或换出的提交内存。基线在 S4 补测，保留首次超预算结果，不因结果调高阈值；不外推到其他机器或其他内存口径。
- 至少连续运行 8 小时，确认历史、内存、句柄及线程不随时间持续增长；验证 Windows 安装、启动及卸载，记录包体和运行时依赖。
- 其他质量与记录口径引用 [产品设计第 11 节](../../design/product-design.md#11-质量目标与验收)；跨平台构建通过不等于对应桌面环境或安装包已获实机验证。
