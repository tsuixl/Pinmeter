# Pinmeter 文档目录

[返回项目首页](../README.md)

文档按产品设计、技术架构、方案调研和开发记录分类。v0.1.0 Windows 主窗口与真实基础监控已实现，完整发行验收仍在进行；实际结果和未验证项以各功能执行记录为准。

**界面依据**：[Sakani 官方 Storybook](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/sakani-design-system--docs) 是强制视觉与组件标准；[HTML 预览](development/v0.1.0-main-window/assets/main-window-preview.html) 仅为布局参考。当前主窗口与标题栏已实现；项目首页保留总览和任务栏展示图。历史测试截图已归档，完整视觉与原生环境验收仍有未完成项，见 [主窗口执行记录](development/v0.1.0-main-window/execution.md)。

## 目录结构

```text
docs/
├─ README.md                       # 分类索引与阅读导航
├─ design/
│  └─ product-design.md             # 产品设计
├─ architecture/
│  └─ overview.md                   # 跨平台架构与代码组织
├─ research/
│  └─ windows-monitoring.md         # Windows 性能采集方案调研
└─ development/
   ├─ README.md                    # 开发约定与功能索引
   ├─ v0.1.0-development-workflow/  # 开发规范
   ├─ v0.1.0-desktop-runtime/       # 工程、窗口生命周期与交付
   ├─ v0.1.0-main-window/           # 主窗口页面
   ├─ v0.1.0-basic-monitoring/      # 基础指标与历史
   ├─ v0.1.0-app-network-ranking/   # 应用网络排行
   ├─ v0.1.0-app-network-control/   # 应用限速与禁用网络（待实机验收）
   ├─ v0.1.0-preferences/           # 基础设置
   ├─ v0.1.0-cpu-temperature/       # CPU 温度
   ├─ v0.1.0-gpu-monitoring/        # GPU 采集与详情页
   ├─ v0.1.0-ip-inspection/         # IP 检测与资料
   ├─ v0.1.0-taskbar-display/       # 任务栏显示与常驻入口
   └─ v0.1.0-hardware-info/         # 启动时一次查询的硬件信息
```

每个功能目录只包含 `design.md`（设计）与 `execution.md`（任务计划和进度）。

## 产品设计 · design

回答“做什么、用户如何使用、达到什么标准”。

- [项目设计文档](design/product-design.md)：定位与原则、功能阶段、悬浮窗与任务栏交互、Sakani 强制视觉标准与 HTML 布局边界、指标口径、质量目标和验收标准。包含技术方案概要，便于理解完整产品。

## 技术架构 · architecture

回答“如何划分代码职责、跨平台如何实现、数据如何流动”。

- [跨平台架构与代码组织](architecture/overview.md)：模块化单体、共享核心与平台适配、MVC/MVVM 职责、接口契约、状态归属、通信协议、工程目录规范及验证边界。第 7 节明确根目录允许项和源码、配置、测试、资源的归属。

## 方案调研 · research

回答“有哪些可参考方案、选择依据是什么、哪些结论还需验证”。

- [Windows 性能采集方案调研](research/windows-monitoring.md)：成熟 GitHub 项目比较、源码与版本依据、Windows API 选择、TrafficMonitor 技术参考和最小验证范围。

## 开发记录 · development

- 硬件信息：[设计](development/v0.1.0-hardware-info/design.md) · [任务计划与进度](development/v0.1.0-hardware-info/execution.md)。启动时一次查询，沿用 Sakani 展示本机硬件清单。

回答“本次功能如何设计、准备做什么、实际完成了什么”。

- 任务栏显示：[设计](development/v0.1.0-taskbar-display/design.md) · [任务计划与进度](development/v0.1.0-taskbar-display/execution.md)。Windows 原生双行读数、Sakani 绘制、设置与托盘入口已实现；完整兼容性、管理员正式宿主和长期验收以原记录为准。
- [首版计划、开发约定与功能索引](development/README.md)：v0.1.0 主窗口范围、S0–S4 交付顺序与各项功能文档；同时包含模板、版本约定及自动提交流程。
- 应用网络排行：[设计](development/v0.1.0-app-network-ranking/design.md) · [任务计划与进度](development/v0.1.0-app-network-ranking/execution.md)。Windows 只读排行已实现，实机与长期验收仍有待办，不含限速。
- 应用网络控制：[设计](development/v0.1.0-app-network-control/design.md) · [任务计划与进度](development/v0.1.0-app-network-control/execution.md)。已接入上下行限速、禁用/恢复网络适配与规则管理界面；Windows 基础下载限速与新连接禁用/恢复已实测，其他场景及发行验收未完成。
- GPU 采集与详情页：[设计](development/v0.1.0-gpu-monitoring/design.md) · [任务计划与进度](development/v0.1.0-gpu-monitoring/execution.md)。Windows 双卡采集与深浅主题已验证，其他硬件、休眠恢复与长期性能仍待验收。
- IP 检测与资料：[设计](development/v0.1.0-ip-inspection/design.md) · [任务计划与进度](development/v0.1.0-ip-inspection/execution.md)。已按 one-ip 源码移植方案实现出口检测与资料适配，使用 Pinmeter/Sakani 界面；代理能力与发行未验证项见执行进度。

## 建议阅读顺序

开发前先阅读 [仓库规则](../AGENTS.md) 与 [开发约定](development/README.md)，再按任务查阅以下文档。

1. 阅读产品设计，了解范围、交互与验收目标；UI 任务实际查阅 Sakani 对应组件与状态，HTML 只用于布局。
2. 阅读技术架构，确定代码边界与实现约束。
3. 按具体实现任务查阅方案调研，核对来源与待验证事项。
4. 查找或建立对应功能目录，先明确设计与任务计划，再实现并更新进度。

## 维护约定

- 产品行为与验收要求维护在 `design/`；代码分层、依赖和协议维护在 `architecture/`；外部方案、版本证据和比较结论维护在 `research/`。
- 具体功能的设计与执行维护在 `development/`，同一功能复用原两份文档，引用公共设计与架构。
- 产品设计中的技术概要通过链接指向架构文档；详细代码约束以架构文档为准，调研建议被采纳后同步到相应设计或架构文档。
- 新文档使用含义明确的小写英文文件名，以连字符分词，并加入本索引。新增类别时再创建目录。
- 调研保留检查日期与来源，区分源码推断、设计建议和实机验证结果。
- 移动或重命名文档时，同步更新项目首页、本索引、文档间链接及目录示意。
