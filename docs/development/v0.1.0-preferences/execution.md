## 任务计划

- [x] 扩展可搜索 Windows 字体目录及真实字体样式，保留旧配置和默认鸿蒙。
- [x] 接入前端与原生同一字面选择、缺失回退、刷新及保存验证。
- [x] 完成真实目录/字面探针、搜索与样式交互、相关回归、Windows 构建和本地提交。

- [x] 接入三项全局字体、默认鸿蒙、旧配置兼容、即时保存和前端加载失败处理。
- [x] 同步原生任务栏字体与测量缓存，保留固定读数布局。
- [x] 完成字体许可、相关回归、Sakani 深浅主题对照、Windows 运行包核对和本地提交。

- [x] v0.1.3：实现启动到托盘、旧配置兼容、显式恢复优先与托盘失败回退，验证设置及启动协议。

- [x] 接入设置原子保存、即时生效、失败回滚、版本冲突及统一保存顺序。
- [x] 实现默认关闭的 Windows 开机自启、构建标识和便携迁移说明。
- [ ] 补测正式主窗口的配置损坏、保存失败、重启恢复、换网卡及真实登录触发。
- [x] 实现卸载时按安装路径清理自启，隔离验证仅删除匹配任务及失败保护。

## 进度

- 字体样式与系统目录扩展已实现并交付：用户体验后要求取代原三项限制，增加搜索和真实样式。`font_family` 保留原三项标识并支持 `installed:` 家族名，`font_style` 缺字段默认 `auto`；设置保存前检查实际目录，原生摘要包含样式并同步重建读数与 Tooltip。缺失选择保持在配置中，Web 与原生显示回退并提示。
- 本机只读探针通过：DirectWrite 枚举 **207 个系统字体家族**（另有三项内置/默认选项），包括用户安装的 HarmonyOS Sans SC 六字重与 0xProto、Arial 的常规/粗体/斜体/窄体；目录不进入采样流，不复制已安装字体到包。真实目录测试及原生字面/权重/缺失回退、六种字体/样式 × 100%/150%/200% 透明和数值对齐回归通过；原有三个管理员/Explorer 测试仍跳过。
- 界面夹具通过 `PINMETER_FONT_PROBE=1` 显式启用只读开发端点，读取同一个已编译的 Windows 探针；`tests/font-settings.html?systemFonts=1` 使用真实目录与真实本机字体，保存仍隔离为演示。已验证[鸿蒙 Medium](assets/system-font-harmony-medium.jpg)、[系统鸿蒙六种样式](assets/system-font-harmony-styles.jpg)、[Arial Narrow Italic](assets/system-font-arial-italic.jpg)、Segoe UI 的搜索 Enter 选择和样式随家族重置。14px 数字预览从 Regular 79.802px 变为 Medium 81.208px，确认实际字面发生变化。
- 已核对[深色搜索](assets/system-font-search-dark.jpg)、[搜索空结果](assets/system-font-search-empty.jpg)、[样式保存失败回滚](assets/system-font-save-failure.jpg)和[缺失选择保留/回退](assets/system-font-missing.jpg)。来源为官方 [Combobox](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/composite-combobox--docs)、[Input](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/forms-input--docs)、Select；Input 实际输入文字 14px、内容高 22px 与官方一致。固定包的 Combobox 没有搜索实现，按设计组合官方 Input 过滤，未改库样式。字体家族/字面是用户例外，其余 Sakani 规范继续生效，HTML 仅为布局参考。
- 已知边界：[0xProto 预览加载拒绝](assets/system-font-load-rejected.jpg)。Windows 可枚举且本机字库可解析，但当前浏览器 `FontFace.local` 拒绝该字面，直接 CSS 家族匹配也回退；界面明确报错并保持旧偏好，不假报应用成功。正式 WebView2、管理员主窗口与任务栏切换仍待用户退出旧实例后实测；不宣称全部系统字库均已完成渲染验收。
- 工程检查：`tools/dev.ps1 check` 完整通过 Rust 工作区测试/Clippy、helper/打包锁、契约、前端构建与格式；后续字体加载缓存和搜索封装的类型检查及 **38 项前端测试**通过，包含真实字面来源转义/粗斜宽度映射、无效样式拒绝和未成功保存时的缓存上限。下文保留上一批三项字体交付证据。
- 本轮 Windows release 从干净实现提交 `e384ed2ff75a4fbfe43867a2fe3bbabc1448ac46` 执行 `PINMETER_HOLD_DELIVERY=1 node tools/desktop.mjs build --no-bundle -- --locked`。前端、全部 helper 与 Rust release 构建成功，入口为 `E:\dev\github\Pinmeter\src\backend\target\test4\Pinmeter.exe`，版本 0.1.3，56,507,392 字节。test1/test2 占用路径无法确认，test3 正在运行，入口均安全跳过；保留旧实例，本轮未启动新的管理员主程序，也未生成安装器。
- 本轮交付核对：340 个源码指纹构建前后及交付前一致，42 个完整运行文件来源/复制件 SHA-256 一致；三份前端 TTF 与共享原文件及下载来源哈希一致。EXE SHA-256：`d502210ab422e7a97c57274425f5c25fcd09202af31f14c808fa0f8d364800d8`。最终实时探针为 207 个系统家族、476 个去重真实字面。证据在 test4 的 `build.log`、`build-source.json`、`source-manifest.json`、`package-hashes.json` 与 `delivery-verification.json`；构建及目录锁保留到核对结束。
- 切换版本须完全退出旧版再运行 test4，保留整个目录及伴随资源。此功能与验收记录仅本地提交；未推送、上传或发布。正式 WebView2/管理员任务栏、跨平台与完整视觉验收仍受上述边界限制。

- 全局字体已实现并交付：默认内置 HarmonyOS Sans SC，可切换 Geist / 系统默认；已确认配置贯通主窗口、原生读数与自绘提示。保存前加载目标字体，失败保留原值；旧配置缺字段与恢复默认使用鸿蒙。Windows 系统字体读取非客户区消息字体，字体变化重建绘制对象并失效旧测量。
- 代码验证：`tools/dev.ps1 check` 通过 helper/打包锁回归、314 个本地文档链接及分层检查、Rust 格式/工作区测试/Clippy、契约一致性、前端 production 构建、35 项测试和格式检查。原有 3 项管理员 ETW/真实 Explorer 测试保持跳过。追加的设置间距与隔离 HTML 验证夹具已通过类型检查和格式化；release 会重新构建前端。
- 字体资源：Regular、Medium、Bold 和 LICENSE 与 `E:\Download\HarmonyOS_Sans` 原文件 SHA-256 一致；未安装、转换或裁剪。生产前端构建包含三份原始 TTF；原生 DirectWrite 三项字体创建、无效标识拒绝、鸿蒙数字等宽以及三项字体在 100%/150%/200% 的透明和百分比/温度像素对齐测试通过。
- 浏览器验证：实际切换三项字体、键盘 End/Enter 选择和恢复默认通过；[鸿蒙默认](assets/font-harmony-light.jpg)、[Geist](assets/font-geist-light.jpg)、[系统默认](assets/font-system-light.jpg) 均已核对，1280×720 下全页无水平溢出。隔离夹具 `src/frontend/tests/font-settings.html?save=slow` 验证[保存中禁用](assets/font-saving.jpg)、回执前保留旧字体及成功后应用；`?save=fail` 验证[失败回滚](assets/font-save-failure.jpg)，不读写实际用户设置。主页面控制台无 error。
- 视觉来源：对照官方 [Select](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/forms-select--docs) 及[深色原控件](assets/font-sakani-select-dark.jpg)，应用[浅色列表](assets/font-options-light.jpg)和[深色列表](assets/font-options-dark.jpg)沿用官方控件。实际文字 14px/400、控件高 40px、圆角 8px、深色表面 RGB(20,20,20) 与来源一致；家族替换是用户明确要求的例外。HTML 仅供布局。以上是浏览器演示与原生位图验证，旧版实例仍在运行，正式管理员主窗口冷启动、任务栏实时切换/Explorer 恢复、混合 DPI 与其他平台未验收，整体视觉验收仍未完成。
- Windows 交付：从干净实现提交 `22a41106e484d53b7b5ecb44a1d3a3ba9bf194b3` 执行 `PINMETER_HOLD_DELIVERY=1 node tools/desktop.mjs build --no-bundle -- --locked`，前端、全部 helper 和 Rust release 构建成功。test1 无法确认现有进程路径、test2 正在运行，均由入口跳过；本次实际路径为 `E:\dev\github\Pinmeter\src\backend\target\test3\Pinmeter.exe`，产品版本 0.1.3，40,018,944 字节。本次提供完整便携运行包，不生成安装器。
- 交付复核：42 个主程序/伴随文件的来源与复制件 SHA-256 一致；334 个源码文件构建前后及交付前指纹一致；生产前端的三份 TTF 与下载原文件、共享原文件逐字节哈希一致。主 EXE SHA-256 为 `3c3055b69788e2c4e8c92b1c9712b1590507bfffbd7b4a69921e30f3d3178910`。证据保存在 test3 的 `build.log`、`build-source.json`、`source-manifest.json`、`package-hashes.json`、`font-verification.json` 和 `delivery-verification.json`，构建锁与目录独占保留至交付核对结束。
- 运行边界：保留既有两个 Pinmeter 实例，未将打开旧窗口当作新版冷启动或管理员任务栏验证。切换时完全退出旧版后再启动上述入口；移动或保留新版时须保留整个 test3 目录及 sensors、network、network-control、licenses。实现和验收记录仅本地提交，未推送或发布。

- P1 浏览器：对照 Sakani Switch、Radio 的受控 0.3.1 组件，核对[浅色启动开关](assets/p1-startup-light.jpg)与[深色保存状态](assets/p1-startup-dark.jpg)，点击标签可即时保存启动偏好。HTML 仅供布局；管理员真实冷启动、自启登录仍未验证，不能由浏览器演示推断。统一构建结果见 [desktop-runtime](../v0.1.0-desktop-runtime/execution.md)。

- v0.1.3：新增默认关闭的启动到托盘开关，前端就绪后交由后端一次性决策；旧配置保持显示，托盘失败回退窗口，重复打开和退出确认优先显示，界面重连不再隐藏窗口。核心状态回归、旧配置原子保存、Rust 工作区及最终前端 33 项测试通过；已包含在本轮 `test2/Pinmeter.exe`，完整来源与哈希见 desktop-runtime 交付记录。管理员实际冷启动仍未验收。

- v0.1.2：版本卡片接入统一更新状态、自动检查偏好及公告入口；旧监控配置不迁移，更新偏好使用单独原子文件。检查与 UI 证据见 [app-update](../v0.1.2-app-update/execution.md)。

- 慢系统注册和存储在监控锁外执行；多个入口共用串行保存，退出先拒绝新写入并等待已接纳保存，再使用最终偏好处理网络规则。
- 既有用例覆盖保存/注册失败补偿、旧配置迁移、版本竞态及采样不被慢保存阻塞；管理员自启探针的注册、读取、删除曾通过，未注销或重启验证登录触发。
- 设置页和侧栏共用应用版本及构建信息；迁移便携目录后需更新自启路径。卸载命令在完成网络恢复后，使用嵌入的同一平台脚本清理指向本安装的自启任务；其他目录、非标准名称和含额外参数/动作的任务保留。正常退出和同目录升级不清理自启。
- 隔离任务适配测试覆盖大小写及规范化路径、其他安装、模糊动作、重复清理、删除失败及删除后残留；完整项目检查通过。正式交付 helper 已包含该脚本；本轮管理员验证授权取消，没有创建系统测试任务，真实卸载和登录触发仍未验证。运行包与安装包见 [desktop-runtime](../v0.1.0-desktop-runtime/execution.md)。
