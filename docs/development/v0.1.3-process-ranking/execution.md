## 任务计划

- [x] 2026-10-04 内存自动单位：统一三类行显示，覆盖 1024 阈值、舍入晋位、正常零值/无效值及原字节排序；检查界面并构建核对 Windows 运行包。

- [x] 2026-10-04 搜索布局：摘要移到资源占用下方，搜索与视图切换/恢复列贴近表格并对齐；检查宽窄、深浅、搜索/排序/恢复列，构建本次 Windows 运行包并本地提交。

- [x] 2026-10-04：将长说明及大面积提示改为信息浮层和紧凑状态，检查正常/预热/暂停/读取不全、键盘与深浅主题，随本次 Windows 交付。
- [x] 修正父子名称相对缩进，接入可拖动重排/调宽/点击排序表头，验证方向、无效值、分组及拖动不误排序；核对深浅主题、窄窗，构建本次完整 Windows 包并提交。
- [x] 修正折叠按钮与名称换行，检查普通/长名称、展开/搜索禁用、深浅主题及窄窗；完成前端检查、本次 Windows 构建和本地提交。
- [x] 第 5 项：接受指标页排序与返回上下文，明确当前读数语义。
- [x] 第 7 项：实现可靠应用归组、搜索、固定、展开、暂停/恢复和复制；补充身份/不完整/排序行为测试并完成浏览器深浅窄窗检查。
- [ ] 第 7 项：通过本批统一类型/行为检查及完整 Windows 构建，记录原生验证边界。
- [x] 实现只读采样、进程身份与 CPU 差值、Top 10。
- [x] 接入界面，检查权限/空态与排序，完成测试及截图。
- [x] 构建、核对完整 Windows 运行包并本地提交。

## 进度

- 2026-10-04 内存单位交付：源码 `ce9a85bc5c0fb6cba9bc8c48be98e97ead18fa07`，经 `node tools/desktop.mjs build --no-bundle -- --locked` 生成最新前端、Windows release 和辅助组件。实际入口 `E:\dev\github\Pinmeter\src\backend\target\test15\Pinmeter.exe`，v0.1.3，SHA-256 `5792607edc65f1149d2e37c47ada6398dafc6287556275e54bd07815cc41b70c`。385 个源码指纹、42 个运行文件及配置路径/源与副本哈希核对通过；生产 JavaScript 不包含单位演示参数/数据标识。构建与目录独占保持至交付核对完成，构建来源与各清单、独立复核结果随包保留。现有 test14 和提权实例未停止，未启动新版；原生主程序/管理员/混合 DPI 未实测，不将浏览器演示视为真实采集验收。仅有既有前端大块和 MSVC 创建库的非阻塞提示；切换时完全退出旧版并保留整个 test15 目录。

- 2026-10-04 单位换算整合检查：全部前端 18 个测试文件、101 项测试通过；TypeScript、修改文件 Prettier、文档/目录和差异检查通过。样例开发客户端的默认数据不变，仅显式单位演示参数启用混合量级数据。

- 2026-10-04 内存自动单位实现：进程展示模型新增二进制格式函数，应用/单进程/子进程共用；B 显示整数，KiB 至 EiB 显示一位小数，舍入到 1024 时晋位。5148.7/3652.5/1951.5 MiB 分别显示 5.0/3.6/1.9 GiB，正常零值为 0 B；缺失、负值或非有限数为“—”，非正常读取优先保留状态。后端字节值与数字排序未变，Sakani Table 与数字排版未改，HTML 仍只供布局。
- 单位阈值/舍入、B 至 EiB、零值/无效值与同显示文本的原字节排序回归通过，进程模型测试共 43 项。开发入口 `?demo=1&processMemory=units` 仅为混合单位样本，保留明显演示标识，生产不使用此客户端；浏览器核对 [应用汇总浅色](assets/memory-units-light.jpg)、[单进程深色](assets/memory-units-dark.jpg)，展开子进程同样显示 GiB/0 B，升序为 512 B → 768.0 KiB → 1.9/2.4/3.6/5.0 GiB，普通演示仍显示 581.6 等 MiB。控制台无错误，未把演示读数当作本机实测；完整构建及原生验证边界见本节内存单位交付记录。

- 2026-10-04 搜索布局交付：源码 `a040e2570f3763cd8f9545fac8e95e5a0e8bfe60`，经 `node tools/desktop.mjs build --no-bundle -- --locked` 完整构建最新前端、Windows release 和辅助组件。入口 `E:\dev\github\Pinmeter\src\backend\target\test14\Pinmeter.exe`，v0.1.3，SHA-256 `df94835d9b5e7d963422da7e7ad3dc98bc1306d6c512d4585150bd6785ff602b`。385 个源码指纹、42 个运行文件及配置路径/源与副本哈希复核通过；随包保留构建来源、源码清单、文件哈希和独立复核结果。构建与目录独占保持至交付核对完成，保留已运行的 test13 及提权实例，未启动新主程序，因此原生字体/缩放、管理员与实际鼠标环境仍未验证。仅有既有前端大块和 MSVC 创建库提示，无构建错误。切换前完全退出旧版并保留完整 test14 目录。
- 已实际对照官方 [Segmented Control Two](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-segmented-control--two) 的默认/选中结构及固定包 Input/Button 小号规格，沿用原主题与字体；本轮局部布局检查通过，完整原生视觉验收保持原边界。

- 2026-10-04 搜索布局实现：将资源标题、短状态/匹配/时间、固定与复制反馈收为紧凑摘要区（内部 8px）；视图切换、无外置标签的搜索框和恢复默认列统一到表格上方工具栏，距表格 8px。搜索保留“查找应用或进程”可访问名称和搜索图标/占位文字，复用 Sakani 小号 Input/Button 与默认 SegmentedControl，按控件中心对齐，不覆盖官方高度；HTML 仅供结构。ProcessTable 只增加 ReactNode 工具栏插槽，保持同一实例及原列状态/操作。
- 浏览器检查：[1200×800 浅色](assets/search-toolbar-light.jpg)、[深色](assets/search-toolbar-dark.jpg)、[420×700 搜索与换行](assets/search-toolbar-narrow.jpg)。1200 与 880 宽度的切换/搜索/恢复列中心差小于 0.01px；窄窗视图切换独立一行，搜索和恢复列同排，主内容 clientWidth/scrollWidth 均为 341，仅表格横向滚动。名称搜索、清空、按应用/进程切换、CPU 升序、暂停、键盘调宽和恢复列通过；调整后的列宽在搜索变更时保持，控制台无错误。类型、修改文件格式及差异检查通过；本次是布局调整，未新增或重复运行采集/业务单测。Windows 完整构建与原生验证边界见本节搜索布局交付记录。

- 2026-10-04 本批最终交付：源码 `4d4d6bf`，实际入口 `E:\dev\github\Pinmeter\src\backend\target\test13\Pinmeter.exe`。完整 Windows release 与辅助组件构建成功，385 个源码指纹及 42 个运行文件路径/哈希复核通过；[构建与验证边界](../v0.1.0-main-window/execution.md)。本次未启动新主程序，管理员/原生鼠标和混合 DPI 保持未验证。

- 2026-10-04 浏览器验收：[浅色主视图](assets/info-compact-light.jpg)、[深色暂停与读取不全](assets/info-compact-dark.jpg)、[420×700 预热说明浮层](assets/info-open-narrow.jpg)。长蓝色说明区已移除；采样中、已暂停、读取不全数量等仍可见，表格提前显示。信息图标 Enter 打开、说明区键盘 End 滚至末尾、Esc 关闭并返回图标焦点通过，静态口径及动态原因完整保留。对照用户截图仅调整信息层级，控件继续使用 Sakani，HTML 仅供结构；默认表头排序、列恢复、搜索和暂停逻辑未修改。统一类型/格式/差异检查及控制台检查通过，未以浏览器验证冒充原生管理员/混合 DPI；Windows 交付待补充。

- 2026-10-04 说明收起已实现：资源占用标题旁复用共享 InfoPopover，迁移 CPU 基线、归组与工作集口径、搜索/固定/展开、暂停及表头排序/拖拽/调宽操作说明。采样中、权限不足、失败、读取不全数量、截断和暂停继续常驻为紧凑 Sakani Badge；浮层保留查询错误、采集原因和缺失/截断解释。匹配数与空态精简，移除表格顶部长提示并保留恢复默认列；应用行仅显示进程数，内部身份、真实无效值及采集/排序逻辑未改。全前端类型、局部格式与差异检查通过；深浅/窄窗/状态截图与本次 Windows 包待本批统一核对，不以静态修改视为视觉或原生验收。

- 层级/表头最终交付：从干净提交 `b1a132ab4370707a3ed9ec2146072125a9905f81`（含主窗口 HTML5 拖放配置）经 `PINMETER_HOLD_DELIVERY=1 node tools/desktop.mjs build -- --locked` 完整生成前端、宿主、辅助组件、release/NSIS。实际入口 `E:\dev\github\Pinmeter\src\backend\target\test9\Pinmeter.exe`，版本 0.1.3，57,430,016 字节；42 个运行文件源/副本哈希、379 个源码指纹及窗口配置复核通过。EXE SHA-256：`3d1245ac3af7c387d74439290a8ad3062429a7848b749de5f56a4a628a420cff`。安装器复制到 `test9/update-artifacts/` 后哈希、Tauri 签名与 PE 版本复核通过，SHA-256：`f7865263cc019b18ded7606e552dd3b344f094fdab066fe3d6add8845840189c`。此前 test8 为配置补齐前的中间构建，不作为本次最终交付；test7 现有实例保留。
- 本次检查包括 70 项前端测试、类型/局部格式、407 个本地文档链接、生产构建及上述浏览器交互；后端采样逻辑未改，不重复运行不相关后端单测。test9 CPU/GPU helper 启动、协议和正常退出通过（CPU permission_denied、GPU normal / 1 设备）；证据位于同目录 build-source、source-manifest、package-hashes、delivery-verification、helper-smoke JSON 与 build.log。构建及编号目录独占保持至交付核对完成；真实管理员主程序、WebView2 鼠标操作和用户字体/缩放仍未实测，不将浏览器检查当作原生验收。切换前完全退出旧版并保留完整运行目录，未推送或发布。

- 交付前核对固定 Tauri 2.11.5 的 WindowBuilder 文档及官方配置说明：Windows HTML5 拖动要求关闭原生 drag/drop handler。已给 main 窗口显式设置 `dragDropEnabled: false`；未发现依赖原生文件拖放的现有入口。已从包含该配置的提交重建上述 test9 完整运行包，未把此前仅浏览器通过当作原生支持证据。

- 2026-10-04 层级与表头已实现：父级折叠槽使用 Sakani IconButton sm（32px），子名称在父名称基础上再右移 24px；原指标快捷切换集中为可点击表头。名称/PID 默认升序、CPU/内存默认降序，再次点击反转；父子成组、固定项优先、异常值置后。应用 PID 只排序子进程并给出说明。列宽/顺序随刷新、暂停及空结果保持，离开页面后重新初始化，不冒充持久偏好。
- 本次验证：进程排序与列布局 24 项针对性测试、全部前端 70 项测试和类型检查通过。浏览器实际验证鼠标调宽增加 120px、拖 CPU 到名称之前、键盘移动/调宽/最小宽度、恢复默认列，表头与数据对应且拖动不改变排序；名称/PID/CPU/内存点击排序及 PID 反向通过，空筛选恢复后布局保留。深浅主题下子名称右移 24px；420px 窄窗主内容 clientWidth/scrollWidth 均 341，只有表格横向滚动，未压缩官方控件。证据：[浅色层级](assets/columns-tree-light.jpg)、[深色换列](assets/columns-reordered-dark.jpg)、[窄窗](assets/columns-narrow.jpg)。Sakani Table、Button、IconButton 规格及状态已核对，HTML 仅供布局；原生管理员主窗口、用户缩放/字体及鼠标设备仍待实机复核，完整包见上方最终交付。

- 名称行修复交付：从干净提交 `f9dd779318929a56a7eb527d34e7a49c1decbfcd` 通过 `PINMETER_HOLD_DELIVERY=1 node tools/desktop.mjs build -- --locked` 完整构建前端、宿主、辅助组件和 NSIS。实际入口 `E:\dev\github\Pinmeter\src\backend\target\test7\Pinmeter.exe`，版本 0.1.3，57,425,920 字节；42 个运行文件与 376 个源码指纹独立核对通过。EXE SHA-256：`2c7f0ebd5e13259e8fb07bca6115c601faf3a005809b9476abeeeb3a6d215a92`；同目录 update-artifacts 中安装器的复制哈希、Tauri 签名和 PE 版本均通过，SHA-256：`1e2d1b036f8261423a7b21e068801c3958a604f4e5fff2c516ec034005609c96`。构建锁和编号目录独占保持至交付复核结束，test6 运行实例保留。
- 本次验证包括前端类型/局部格式、404 个本地文档链接、生产构建与浏览器布局，未为纯布局改动增加镜像测试或重跑不相关后端单测。运行包 CPU/GPU helper 协议与退出通过（CPU permission_denied，GPU normal / 1 设备），详见 test7 的 build-source、source-manifest、package-hashes、delivery-verification 和 helper-smoke JSON。本次未启动新的管理员主窗口，不将浏览器或文件核对当作原生字体/缩放验收；先完全退出旧版再打开 test7，移动时保留整个运行目录。未推送或发布。

- 2026-10-04 名称行布局：移除身份区继承的 flex 换行，改为按钮/名称同一网格行、辅助说明置于名称下方；名称可收缩省略，完整 title 保留，长说明可换行并去掉多余底边距。不改变列宽、Sakani 控件尺寸/颜色或采集行为。已实际核对官方 Table 默认、深色和 640px 卡片切换，HTML 仅作结构参考。
- 本次浏览器验证：1280/960/640/420 宽度下，所有可见应用的按钮与名称垂直中心差小于 0.001px，无页面横向溢出；960 宽度下长名称省略且 title 完整。展开子进程、搜索自动展开与折叠按钮禁用均可用。证据：[浅色展开](assets/name-alignment-light.jpg)、[深色桌面窄窗](assets/name-alignment-dark-960.jpg)、[420px 搜索状态](assets/name-alignment-narrow.jpg)。前端类型与修改文件格式检查通过；Windows 原生缩放与当前用户字体实机复验另列，本次完整包见上方交付记录。

- 2026-10-04 本批交付：源码 `96b8c4677384bf32f09f7aec0920698832492bfd`，实际入口 `E:\dev\github\Pinmeter\src\backend\target\test6\Pinmeter.exe`。统一检查通过（Rust 167 项、前端 49 项、格式/Clippy/契约/打包工具）；完整 release/NSIS 构建、42 个运行文件及 376 个源码指纹核对通过。原生、管理员、真实升级及长期未验证项保留；[交付详情](../v0.1.0-desktop-runtime/execution.md)。

- 第 7 项浏览器验收：内存入口自动选内存，搜索 PID 2108 可找到默认前十以外进程；两个同名浏览器按不同身份独立展示。复制 PID 显示成功反馈、固定与暂停/恢复状态可用；[浅色](assets/investigation-light.jpg)、[深色暂停](assets/investigation-dark.jpg)、[420px 窄窗](assets/investigation-narrow.jpg)已保存并对照 Sakani 核对，根 scrollWidth=420，无横向溢出。没有应用控制台错误，未操作真实进程或权限。
- 补充服务重建边界：宿主为进程及应用 DTO 增加每实例不复用前缀，失败更新后重新启动采集服务也不会将固定项误绑到新 app:1。新增回归验证同 PID/创建时间/路径的两个服务实例输出不同身份；不将这个身份用于持久化。

- 2026-10-04 第 5/7 项实现：按指标入口带入排序；新增应用/进程两种视图、覆盖本次有界全表的名称/PID 搜索、固定、展开、复制名称/PID与暂停/恢复。暂停停止前端定时器及页面租约续期；恢复重新读取。应用汇总由核心计算，界面只筛选排序；同名不同路径不合并，缺失指标不成为有效零值，路径不出后端且不持久化。新增核心路径/无权限聚合回归和前端搜索超出前十、固定身份、无效指标排序测试，等待统一执行。
- 对应 Sakani Table（44px 行、语义变量、640px 响应式）、Input（标签与默认/聚焦/错误/禁用）及 Button（32/40/48px 与 ghost/secondary）官方文档已经浏览核对；沿用受控包，不自行替换视觉，HTML 仅供结构参考。精确格式和差异检查通过；新增界面的深浅/窄窗证据见本节，类型/构建及真实权限场景仍待本批统一验收。

- 对照 Sakani 0.3.1 Table 的默认 44px 行、深色与窄窗堆叠示例，并复用其 Segmented Control/Alert；[浅色](assets/p1-light.jpg)、[深色](assets/p1-dark.jpg)、[窄窗](assets/p1-narrow-dark.jpg)及[权限空态](assets/p1-permission-light.jpg)已留存，CPU/内存切换、部分数据提醒和空态可用，420×400 页面无横向溢出。HTML 仅作结构参考；真实管理员主窗口仍未验收，统一交付见 [desktop-runtime](../v0.1.0-desktop-runtime/execution.md)。
- 已实现 Windows Toolhelp + 受限查询权限的进程时间/工作集采集，页面 CPU/内存 Top 10、时间戳与部分数据提示。无终止进程或提权入口，未读取完整命令行，完整路径不出适配器。
- 原生只读探针两轮成功：379 个进程、143 个部分不可读，单轮约 12 ms；自身 CPU 与工作集、两种前十排行均有效。此为当前权限的短探针，不等于管理员/所有受保护进程验收。
- CPU 多核归一、PID 复用、回退、间断、权限及稳定排序回归通过；全目标 Clippy、前端类型检查及最终 33 项测试通过。深浅/窄窗截图已完成；功能已包含在 `test2/Pinmeter.exe`，来源与完整资源哈希见 desktop-runtime 交付记录。
