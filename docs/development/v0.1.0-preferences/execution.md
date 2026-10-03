## 任务计划

- [x] 接入三项全局字体、默认鸿蒙、旧配置兼容、即时保存和前端加载失败处理。
- [x] 同步原生任务栏字体与测量缓存，保留固定读数布局。
- [ ] 完成字体许可、相关回归、Sakani 深浅主题对照、Windows 运行包核对和本地提交。

- [x] v0.1.3：实现启动到托盘、旧配置兼容、显式恢复优先与托盘失败回退，验证设置及启动协议。

- [x] 接入设置原子保存、即时生效、失败回滚、版本冲突及统一保存顺序。
- [x] 实现默认关闭的 Windows 开机自启、构建标识和便携迁移说明。
- [ ] 补测正式主窗口的配置损坏、保存失败、重启恢复、换网卡及真实登录触发。
- [x] 实现卸载时按安装路径清理自启，隔离验证仅删除匹配任务及失败保护。

## 进度

- 全局字体已实现：默认内置 HarmonyOS Sans SC，可切换 Geist / 系统默认；已确认配置贯通主窗口、原生读数与自绘提示。保存前加载目标字体，失败保留原值；旧配置缺字段与恢复默认使用鸿蒙。Windows 系统字体读取非客户区消息字体，字体变化重建绘制对象并失效旧测量。下一步从本批冻结源码构建 Windows release。
- 代码验证：`tools/dev.ps1 check` 通过 helper/打包锁回归、314 个本地文档链接及分层检查、Rust 格式/工作区测试/Clippy、契约一致性、前端 production 构建、35 项测试和格式检查。原有 3 项管理员 ETW/真实 Explorer 测试保持跳过。追加的设置间距与隔离 HTML 验证夹具已通过类型检查和格式化；release 会重新构建前端。
- 字体资源：Regular、Medium、Bold 和 LICENSE 与 `E:\Download\HarmonyOS_Sans` 原文件 SHA-256 一致；未安装、转换或裁剪。生产前端构建包含三份原始 TTF；原生 DirectWrite 三项字体创建、无效标识拒绝、鸿蒙数字等宽以及三项字体在 100%/150%/200% 的透明和百分比/温度像素对齐测试通过。
- 浏览器验证：实际切换三项字体、键盘 End/Enter 选择和恢复默认通过；[鸿蒙默认](assets/font-harmony-light.jpg)、[Geist](assets/font-geist-light.jpg)、[系统默认](assets/font-system-light.jpg) 均已核对，全页无水平溢出。隔离夹具 `src/frontend/tests/font-settings.html?save=slow` 验证[保存中禁用](assets/font-saving.jpg)、回执前保留旧字体及成功后应用；`?save=fail` 验证[失败回滚](assets/font-save-failure.jpg)，不读写实际用户设置。主页面控制台无 error。
- 视觉来源：对照官方 [Select](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/forms-select--docs) 及[深色原控件](assets/font-sakani-select-dark.jpg)，应用[浅色列表](assets/font-options-light.jpg)和[深色列表](assets/font-options-dark.jpg)沿用官方控件。实际文字 14px/400、控件高 40px、圆角 8px、深色表面 RGB(20,20,20) 与来源一致；家族替换是用户明确要求的例外。HTML 仅供布局。以上是浏览器演示与原生位图验证，旧版实例仍在运行，正式管理员主窗口冷启动、任务栏实时切换/Explorer 恢复、混合 DPI 与其他平台未验收，整体视觉验收仍未完成。

- P1 浏览器：对照 Sakani Switch、Radio 的受控 0.3.1 组件，核对[浅色启动开关](assets/p1-startup-light.jpg)与[深色保存状态](assets/p1-startup-dark.jpg)，点击标签可即时保存启动偏好。HTML 仅供布局；管理员真实冷启动、自启登录仍未验证，不能由浏览器演示推断。统一构建结果见 [desktop-runtime](../v0.1.0-desktop-runtime/execution.md)。

- v0.1.3：新增默认关闭的启动到托盘开关，前端就绪后交由后端一次性决策；旧配置保持显示，托盘失败回退窗口，重复打开和退出确认优先显示，界面重连不再隐藏窗口。核心状态回归、旧配置原子保存、Rust 工作区及最终前端 33 项测试通过；已包含在本轮 `test2/Pinmeter.exe`，完整来源与哈希见 desktop-runtime 交付记录。管理员实际冷启动仍未验收。

- v0.1.2：版本卡片接入统一更新状态、自动检查偏好及公告入口；旧监控配置不迁移，更新偏好使用单独原子文件。检查与 UI 证据见 [app-update](../v0.1.2-app-update/execution.md)。

- 慢系统注册和存储在监控锁外执行；多个入口共用串行保存，退出先拒绝新写入并等待已接纳保存，再使用最终偏好处理网络规则。
- 既有用例覆盖保存/注册失败补偿、旧配置迁移、版本竞态及采样不被慢保存阻塞；管理员自启探针的注册、读取、删除曾通过，未注销或重启验证登录触发。
- 设置页和侧栏共用应用版本及构建信息；迁移便携目录后需更新自启路径。卸载命令在完成网络恢复后，使用嵌入的同一平台脚本清理指向本安装的自启任务；其他目录、非标准名称和含额外参数/动作的任务保留。正常退出和同目录升级不清理自启。
- 隔离任务适配测试覆盖大小写及规范化路径、其他安装、模糊动作、重复清理、删除失败及删除后残留；完整项目检查通过。正式交付 helper 已包含该脚本；本轮管理员验证授权取消，没有创建系统测试任务，真实卸载和登录触发仍未验证。运行包与安装包见 [desktop-runtime](../v0.1.0-desktop-runtime/execution.md)。
