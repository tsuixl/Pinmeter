# hardware-info

> 当前工作版本：v0.1.0

## 目标与范围

- 2026-09-22：按用户截图新增硬件信息页，优先交付可用界面与已有系统查询结果；静态信息启动后异步查询一次并缓存，不定时扫描。Windows 首先实现，其他平台明确显示未支持。
- 总览之后增加“硬件信息”。三张摘要卡显示设备、系统和系统启动时间推算的运行时长；下方按主板、处理器、内存、显卡、显示器、硬盘、音频、网卡列出设备。提供复制信息；首版不做截图导出、热插拔刷新或独立硬件详情页。
- 参考图只提供内容与信息层级，视觉遵守 [产品设计](../../design/product-design.md) 5.4；[HTML](../v0.1.0-main-window/assets/main-window-preview.html) 只参考布局。复用固定 @sakaniui/react 0.3.1 的 Card、Button、SidebarItem、Badge/Alert 与官方变量。参考 [StatCard](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-stat-card--grid)、[SidebarItem 状态](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-sidebar-item--all-states)；长设备名使用 Card 内正常标题排版，避免强行套大数字。
- 实际采用 [Card](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-card--default) / [Dark Mode](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-card--dark-mode) 与 [Button](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/core-button--all-variants) / [Dark Mode](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/core-button--dark-mode)，共享控件不重写样式。硬件分类使用自适应描述列表，默认同类合为摘要行，详情中多设备逐项换行；详情和复制按钮使用官方 outline/sm 与 leftIcon。

## 方案

- 2026-09-22 简洁布局修正：用户要求像原参考图一样一眼看清整机配置。默认保留三张紧凑摘要卡与八行硬件概览，每类一行，同类设备以斜线连接；长内容最多两行，完整内容可从“详细信息”展开。内存合并品牌、安装容量、类型/速率和插槽摘要，显卡保留型号及容量，磁盘保留型号及容量，显示器保留型号及明确标注的首选分辨率。驱动、BIOS、逐条内存、虚拟适配器等移入详细模式；只有虚拟设备时仍如实显示。详情切换不重新查询，复制默认复制当前模式。摘要卡默认隐藏系统构建号、启动日期等次要字段，说明文字和采集时间标记放入详细模式。保持 Sakani 字号与控件规格，通过减少默认内容和间距改善密度，不缩小字体。

- 遵循 [架构](../../architecture/overview.md) 分层及目录规则：core/hardware 保存项目清单模型和一次查询状态；platform/hardware 封装 Windows 查询；host 装配独立启动任务并提供只读 DTO；frontend/features/hardware 通过客户端与 ViewModel 显示数据。
- 第一版复用已实查的 Windows PowerShell/CIM 方法，固定内嵌脚本、隐藏窗口、限定查询与整体超时；不接受用户脚本或命令，不依赖图吧工具运行。慢查询不持有基础采样状态锁。查询失败按分类保留原因，不用 0 填充缺失值。系统信息只在内存缓存，不上传。
- GPU 型号清单使用系统枚举；显存只复用已有 GPU 权威读数，禁止使用截断的 Win32_VideoController.AdapterRAM。CPU 型号优先复用已有来源；内存展示已安装容量及插槽，不混用系统可用总量；硬盘使用物理设备容量。
- 显示器首版显示 WMI 型号、物理尺寸估算和明确标注的首选分辨率，不将其当作当前模式，不在未验证映射时标记主屏。设备厂商占位型号以主板型号补充；虚拟设备按来源可辨认信息标记，无法可靠分类时不假装物理设备。
- 查询在应用启动时启动一次；页面读取缓存，加载期间短暂重读状态，完成后停止；切页不重新采集。运行时长仅基于启动时间更新显示。部分失败继续显示成功项，复制内容反映当前展示状态。

## 验收

- 本机八类设备有真实数据；空字段、部分失败、整体超时和未支持状态明确；查询不阻塞 CPU/GPU/网络采样。
- 切页不重新启动查询；长名称、多设备、窄窗口、深浅主题与键盘操作可用，保存截图并在执行文档区分功能与视觉验收。
- 运行相关 Rust/前端检查、生成协议检查，构建本次 Windows release；完整资源及哈希核对通过，实际启动与未验证项单独记录。
