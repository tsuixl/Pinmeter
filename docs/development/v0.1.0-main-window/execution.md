## 任务计划

- [x] 2026-10-04 界面精简：实现四张资源卡并核对独立指标状态、深浅主题、宽窄布局。
- [x] 2026-10-04 独立设置页面与返回上下文交付：完成本次 Windows 构建和资源核对，原生验证边界单列。

- [x] 移除侧栏“监控 / 排查 / 工具”分组标题，核对平铺导航深浅主题与折叠状态，构建并交付完整 Windows 运行包。

- [x] 第 5 项：实现 CPU/内存直达进程与上下文返回；浏览器检查指标排序及回看锚点保留。

- [x] 将侧栏品牌标识与 favicon 同步为已选应用图标，检查深浅主题及折叠侧栏。

- [x] 接入总览、硬件信息、CPU、内存、GPU、网络、IP、设置及主题一体化标题栏。
- [x] 完成指标状态、趋势分组、帮助入口和窗口控件的既有检查。
- [x] 更新 README 总览图片，包含当前硬件信息和 IP 入口。
- [ ] 补齐原生 100%/150% 与混合 DPI、多屏、系统运行中主题切换和完整可访问性验收。
- [ ] 完成 macOS/Linux 窗口实机验证；最大化按钮原生悬停贴靠面板尚未提供。

## 进度

- 2026-10-04 界面精简交付：稳定源码 `6e06977d2062f56fa0c0ba7b4cfb228466b93b72`（总览提交 `f969074`、独立设置提交 `6e06977`）通过 `node tools/desktop.mjs build --no-bundle -- --locked` 生成完整前端、Windows release 与辅助组件；本次不生成安装器。入口 `E:\dev\github\Pinmeter\src\backend\target\test11\Pinmeter.exe`，v0.1.3，57,430,016 字节，SHA-256 `fea5ef00ea107950a6b1cae29d0876d72c34477833f93c610ce16ec7c7e1b2f1`。构建前后源码清单一致，交付时再次核对 383 个源码指纹、42 个运行文件、配置资源路径及哈希通过；详情随包 `build-source.json`、`source-manifest.json`、`package-hashes.json`、`delivery-verification.json`。两把锁覆盖构建与交付核对，跳过被占用/无法确认的 test1–test10；保留正在运行的 test10 及无法读取路径的提权实例，没有启动新版，故实际原生启动/管理员行为/混合 DPI 未验证。构建仅保留既有前端块大于 500 kB 与 MSVC 创建库的非阻塞消息。切换时完全退出旧版并保留整个 test11 目录。

- 2026-10-04 四卡实现：CPU/GPU 使用率与温度独立状态合卡，内存/网速保持对应单位和口径，支持 GPU 核心/VR SoC 测温点，型号只出现一次。依照官方 Sakani Card 与 StatCard 读数规格，保留官方悬停/焦点与主题；HTML 只提供结构。[880×600 浅色](assets/overview-four-light.jpg)、[1440×900 深色四列](assets/overview-four-dark.jpg)、[420×700 单列及 VR SoC](assets/overview-four-narrow.jpg)已检查。8 项总览单测覆盖正常零值、温度失败/权限/不支持、上下行独立状态、多卡与历史断档；整合 TypeScript 和前端 79 项单测通过。Windows 构建证据见本节交付记录；演示数据不证明真实采集或原生 DPI。

- 2026-10-04 侧栏简化交付：稳定源码 `70db5527187a23b54488e395a42bb4f16e46813e` 经 `node tools/desktop.mjs build -- --locked` 完整生成前端、Windows release、辅助组件与 NSIS；实际入口为 `E:\dev\github\Pinmeter\src\backend\target\test10\Pinmeter.exe`（v0.1.3）。379 个源码指纹、42 个运行文件的配置路径/哈希及安装器版本/签名复核通过，EXE SHA-256 为 `bb9bd4cf76605690fbaf86ef409bb140a30b0b31bc65cd72e1d733bf8b609d1a`；安装器与签名已复制至该目录 `update-artifacts/` 并核对一致。构建锁与目录独占保留至交付核对结束；已有 `test9` 实例及无法确定路径的提升权限进程未停止，因此没有启动新版主程序，原生/管理员/混合 DPI 交互仍未实测。构建保留已有前端大于 500 kB 提示及 MSVC 创建库的非阻塞消息；无构建错误。切换时完全退出旧版并保留完整 `test10` 目录。

- 2026-10-04 侧栏简化：移除“监控 / 排查 / 工具”标题及专用容器/样式，十项导航顺序与底部设置保留。已核对官方 [SidebarItem / All States](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-sidebar-item--all-states) 和 [Dark Mode](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-sidebar-item--dark-mode)，继续直接复用组件；HTML 仅供结构参考。开发演示环境中 [1280×720 浅色](assets/sidebar-flat-light.jpg)、[深色](assets/sidebar-flat-dark.jpg)与 [880×600 折叠侧栏](assets/sidebar-flat-compact.jpg)检查通过，标题消失、入口齐全，CPU/进程/硬件信息切换、键盘 Enter、选中态与折叠可访问名称正常。TypeScript、修改文件 Prettier、项目文档/目录及 `git diff --check` 通过；Windows 构建和资源核对见上条，原生交互及混合 DPI 未实测。

- 2026-10-04 本批交付：源码 `96b8c4677384bf32f09f7aec0920698832492bfd`，实际入口 `E:\dev\github\Pinmeter\src\backend\target\test6\Pinmeter.exe`。统一检查通过（Rust 167 项、前端 49 项、格式/Clippy/契约/打包工具）；完整 release/NSIS 构建、42 个运行文件及 376 个源码指纹核对通过。原生、管理员、真实升级及长期未验证项保留；[交付详情](../v0.1.0-desktop-runtime/execution.md)。

- 第 5 项浏览器验收：CPU 一分钟曲线键盘回看后进入进程，再返回仍保留一分钟范围与“返回实时”历史锚点；内存入口选中内存排行，均提示当前进程不代表历史归因。[返回 CPU 的深色证据](assets/metric-return-dark.jpg)已保存；进程深浅及 420px 窄窗结果见 process-ranking。采用开发模式演示数据，不替代原生主窗口验证。

- 2026-10-04 第 5 项实现：CPU/内存可直接进入对应排序的进程排行；返回恢复原页、趋势范围、时间锚点和滚动位置。入口明确当前进程不能解释过去时刻。使用官方 Sakani Button；已通过浏览器核对对应文档的 32/40/48px、ghost/secondary 及禁用规范，HTML 仅供结构参考。源码格式检查通过；浏览器截图与交互结果见本节证据，统一构建及原生交互仍待本批验收。

- 2026-10-03 全局字体默认改为 HarmonyOS Sans SC，并可在设置选择 Geist / 系统默认。主窗口启动预加载和确认后切换已接入；Sakani 控件、主题与间距继续沿用，HTML 只供布局。字体浏览器深浅截图、失败回滚及本次构建证据统一见 [preferences](../v0.1.0-preferences/execution.md)；原有正式宿主和完整视觉未验收项保持。

- 2026-10-02 品牌图标：侧栏和 favicon 复用 128×128 导出 PNG，侧栏保持 24×24 布局与拖动排除。浏览器演示环境检查 [浅色](assets/icon-light.jpg)、[深色](assets/icon-dark.jpg)（1280×720）及[折叠侧栏](assets/icon-compact.jpg)（880×600）；图片完整加载且无拉伸、溢出，主题与控件继续复用 Sakani，HTML 仅供结构参考。本次仅验收新标识在这些浏览器状态中的呈现，原生托盘、Shell 图标缓存和混合 DPI 未验证。
- `npm --prefix src/frontend run check`、修改文件 Prettier 检查与 `git diff --check` 通过；最初在 npm 安装尚未完成时的类型检查未启动，依赖安装完成后重跑通过。生产前端随 desktop-runtime 的完整构建验证。

- 界面复用后端数据与已确认设置，ViewModel 组织展示状态；六张总览卡、资源趋势、逐逻辑处理器与状态提示已实现。
- 标题栏使用平台适配、固定窗口按钮和拖动排除，Windows 200% DPI 的主要窗口操作曾通过检查；其他平台和缩放不沿用这一结论。
- Sakani 是视觉与组件标准，[HTML 预览](assets/main-window-preview.html)仅为布局参考。旧测试截图不随公开源码提供，完整视觉验收保持未完成；后续 UI 修改须按原设计重新保存可复核证据。
- [当前深色总览](assets/readme-overview-native-dark.png)为 Windows 实机窗口截图，已检查侧栏入口、六项指标和趋势，图片不包含其他应用。
