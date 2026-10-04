## 任务计划
- [ ] 修正父子名称相对缩进，接入可拖动重排/调宽/点击排序表头，验证方向、无效值、分组及拖动不误排序；核对深浅主题、窄窗，构建本次完整 Windows 包并提交。
- [x] 修正折叠按钮与名称换行，检查普通/长名称、展开/搜索禁用、深浅主题及窄窗；完成前端检查、本次 Windows 构建和本地提交。
- [x] 第 5 项：接受指标页排序与返回上下文，明确当前读数语义。
- [x] 第 7 项：实现可靠应用归组、搜索、固定、展开、暂停/恢复和复制；补充身份/不完整/排序行为测试并完成浏览器深浅窄窗检查。
- [ ] 第 7 项：通过本批统一类型/行为检查及完整 Windows 构建，记录原生验证边界。
- [x] 实现只读采样、进程身份与 CPU 差值、Top 10。
- [x] 接入界面，检查权限/空态与排序，完成测试及截图。
- [x] 构建、核对完整 Windows 运行包并本地提交。

## 进度

- 2026-10-04 层级与表头已实现：父级折叠槽使用 Sakani IconButton sm（32px），子名称在父名称基础上再右移 24px；原指标快捷切换集中为可点击表头。名称/PID 默认升序、CPU/内存默认降序，再次点击反转；父子成组、固定项优先、异常值置后。应用 PID 只排序子进程并给出说明。列宽/顺序随刷新、暂停及空结果保持，离开页面后重新初始化，不冒充持久偏好。
- 本次验证：进程排序与列布局 24 项针对性测试、全部前端 70 项测试和类型检查通过。浏览器实际验证鼠标调宽增加 120px、拖 CPU 到名称之前、键盘移动/调宽/最小宽度、恢复默认列，表头与数据对应且拖动不改变排序；名称/PID/CPU/内存点击排序及 PID 反向通过，空筛选恢复后布局保留。深浅主题下子名称右移 24px；420px 窄窗主内容 clientWidth/scrollWidth 均 341，只有表格横向滚动，未压缩官方控件。证据：[浅色层级](assets/columns-tree-light.jpg)、[深色换列](assets/columns-reordered-dark.jpg)、[窄窗](assets/columns-narrow.jpg)。Sakani Table、Button、IconButton 规格及状态已核对，HTML 仅供布局；原生管理员主窗口、用户缩放/字体及鼠标设备仍待实机复核，完整包待本次构建。

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
