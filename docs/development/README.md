# 开发约定

[文档目录](../README.md) / 开发记录

仓库通用规则见 [AGENTS.md](../../AGENTS.md)。本目录记录每个功能的设计与执行，内容以当前有效方案和实际进度为准。

UI 开发与交接必须明确：[Sakani 官方 Storybook](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/sakani-design-system--docs) 是强制视觉与组件标准，[HTML 预览](v0.1.0-main-window/assets/main-window-preview.html) 仅作布局参考。具体规范与对照验收见 [产品设计第 5.4 节](../design/product-design.md#54-视觉规范)，不能交接为“按 HTML 样式实现”。

## v0.1.0 首版计划

当前工作版本为 **v0.1.2**，在线更新正在实现；最近已发布版本仍为 [v0.1.1 Windows x64 预览版](https://github.com/tsuixl/Pinmeter/releases/tag/v0.1.1)。Windows 预览发行沿用原 [desktop-runtime](v0.1.0-desktop-runtime/design.md) 功能目录，在线更新维护在 [app-update](v0.1.2-app-update/design.md)。下列 v0.1.0 表格和目录保留首版计划含义，发布与验收以各功能执行记录为准。

[preferences](v0.1.0-preferences/design.md) 包含默认关闭的开机自启；任务注册、即时保存和验证由原设置功能维护。

[任务栏显示](v0.1.0-taskbar-display/design.md)采用双行紧凑默认布局，已接入原生显示、设置及常驻入口。关闭选择由 desktop-runtime 统一维护；管理员、兼容性及完整视觉验收按原功能记录。

**当前实现**：主窗口包含总览、硬件信息、CPU、内存、GPU、网络、IP 与设置；基础指标保留最近五分钟趋势。已接入托盘、任务栏直显、开机自启和应用网络控制。关闭默认询问最小化或退出，最小化隐藏任务栏按钮并保留托盘。用户开启的应用流量监控在切页、托盘及界面重连期间连续累计；明确停止后再次开始建立新统计。完整发行验收尚未完成，各能力以原执行记录为准。

已发布 [v0.1.0 Windows x64 预览版](https://github.com/tsuixl/Pinmeter/releases/tag/v0.1.0)。首发验证 Windows；当前参考环境为 Windows 11 专业版 `10.0.26200`、64 位，已实际运行桌面界面与生命周期测试。发布源码的 Windows、macOS 和 Linux [远端 CI](https://github.com/tsuixl/Pinmeter/actions/runs/36585620873) 已全部通过；通过构建不等于已完成对应平台发行验收。

悬浮窗、磁盘实时读写、通用进程页、风扇、告警与长期历史仍后置；CPU 温度、[GPU 采集及详情页](v0.1.0-gpu-monitoring/design.md)、[应用网络排行](v0.1.0-app-network-ranking/design.md)与[应用网络控制](v0.1.0-app-network-control/design.md)已作为本版能力接入。基础下载限速与新连接禁用/恢复已实测；代理/VPN、多传输场景、升级卸载及长期验收仍按原功能逐项记录。

[IP 检测与资料](v0.1.0-ip-inspection/design.md)采用本地源码移植和后端适配，不依赖原作者部署的服务，仍需访问上游数据源。来源、许可和修改告知随模块保留，界面遵循 Sakani；服务条件及发行状态见执行记录。

| 顺序 | 可检查的交付 | 主要功能 |
| --- | --- | --- |
| S0 工程与契约 | 可启动的主窗口、明确的 DTO/能力/设置契约、三平台构建入口；记录参考环境与资源基线 | desktop-runtime、main-window |
| S1 主界面 | 按 HTML 布局组织五页，遵循 Sakani 视觉与组件标准；演示数据清楚标识，预热/失效/断档可检查 | main-window |
| S2 真实监控 | 接入 CPU、内存和网速，统一采样与五分钟历史；完成来源比对并固定精度容差 | basic-monitoring |
| S3 设置与恢复 | 偏好持久化，托盘最小化/恢复、单实例、关闭选择与退出清理、网卡变化和休眠处理 | preferences、desktop-runtime |
| S4 首版交付 | 发布构建接入真实数据，Sakani 视觉对照、普通权限、安装/卸载、缩放、多屏、性能与 8 小时稳定性验收通过 | 全部功能 |

S1 界面与 S2 采集可在 S0 契约确定后并行。主窗口/最小化的资源预算在 S0 测量后固定；精度容差在 S2 首轮对照后固定，未固定或未通过均不能完成 S4。具体方案和进度仅在下面各功能的两份文档维护；本表只说明交付顺序。

S0–S3 的功能主体已实现；S1 曾完成浏览器深浅主题/控件状态及 Windows 200% DPI 检查，历史素材已归档；当前完整视觉验收状态见 main-window 执行记录。S4 仍按功能执行记录逐项验收；原生 100%/150% DPI、混合缩放及未实际运行的休眠、多屏、长期稳定性和其他平台测试不标为通过。

标题栏一体化已实现，视觉继续遵循 Sakani。Windows 200% DPI 下的深浅主题、控件状态与主要窗口操作已检查；跨平台、其他 DPI 和原生悬停贴靠面板限制继续记录在 main-window 与 desktop-runtime，不能视为完整验收通过。

## 本地运行与构建

在仓库根目录运行，前端依赖先安装一次：

```powershell
npm.cmd --prefix src/frontend ci
powershell -ExecutionPolicy Bypass -File tools/dev.ps1 dev
# 格式、类型、行为、契约及工程检查
powershell -ExecutionPolicy Bypass -File tools/dev.ps1 check
# Windows release 与 NSIS 安装器
powershell -ExecutionPolicy Bypass -File tools/dev.ps1 build
```

Windows 构建入口自动预留空闲的 `src/backend/target/testN/`，输出完整运行目录；默认编译缓存和安装器在 `src/backend/target/desktop-build/`。仅需便携运行包时使用 `node tools/desktop.mjs build --no-bundle -- --locked`，打包与构建锁回归使用 `node tools/desktop.mjs check-tools`。`--package-dir=src/backend/target/testN` 指定的目录仍须通过独占与占用检查；自定义 `CARGO_TARGET_DIR` 必须为绝对路径。直接 npm/Cargo 不取得项目构建锁，不能与完整构建同时写共享产物。

Windows 本地检查及 CI 会先通过 `node tools/desktop.mjs prepare` 编译辅助组件并准备资源，再检查 Rust 宿主；该操作与完整构建共用构建互斥。`check-tools` 包含隔离自启清理与 NSIS 卸载回归；首次尚未下载 NSIS 工具时会明确跳过后者，应在首次安装包构建后重跑。

配置保存于 `%APPDATA%\io.pinmeter.desktop\settings.json`。便携版迁移后，已启用自启的用户在新位置关闭再开启自启，以更新路径。仅调试界面可运行 `npm.cmd --prefix src/frontend run dev`，访问 `http://127.0.0.1:1420/?demo=1`；演示入口仅在开发模式存在，普通浏览器不连接本机采集。

详细构建与管理员/跨平台验证边界见 [desktop-runtime](v0.1.0-desktop-runtime/execution.md)。[Sakani 官方标准](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/sakani-design-system--docs) 决定视觉与组件，[HTML 预览](v0.1.0-main-window/assets/main-window-preview.html) 只提供结构参考，界面验收见 [main-window](v0.1.0-main-window/execution.md)。

## 目录与版本

```text
development/
├─ README.md
└─ v0.1.0-development-workflow/
   ├─ design.md
   └─ execution.md
```

- 功能目录命名为 `vX.Y.Z-short-name`：版本采用三段数字，功能名使用简短的小写英文，以连字符分词。
- 目录中的版本记录功能首次建立时的计划交付版本，用于标识该功能，后续保持目录名稳定；它不表示已经发布。
- `design.md` 用“当前工作版本”记录本次工作的目标版本；普通文档补充和进度更新不自动升版，目标版本明确变化时仅更新原文档。产品设计等公共文档自身的修订版本独立编号。
- 新功能以当前明确的目标版本建立目录并填写当前工作版本；已有功能沿用设计中的当前工作版本。尚无目标版本时，先在功能设计中说明采用的版本。本仓库首次文档与规则基线使用 `v0.1.0`。
- 同一功能的继续开发、修复和优化复用原目录与原两份文档，不因版本变化重命名或复制目录；旧状态通过 Git 历史查看。
- 只有能独立说明目标与验收的新功能才建立新目录。公共规则或跨功能维护也使用一个明确主题的目录，避免挂在无关功能下。
- 每个功能目录仅有 `design.md`、`execution.md` 两份 Markdown；相关图片等素材需要时放 `assets/`。不另建同功能的计划、总结、问题或测试报告文档。

## 文档内容

`design.md` 记录当前设计：目标与范围、相关公共文档、涉及模块和接口、必要方案及验收标准。只写该功能的差异，简单功能几条即可，复杂功能补充必要细节。

```markdown
# short-name

> 当前工作版本：vX.Y.Z

## 目标与范围
- 要解决的问题、纳入范围。

## 方案
- 相关设计/架构链接；涉及模块、职责和关键行为。

## 验收
- 可以实际检查的完成条件。
```

`execution.md` 只设“任务计划”和“进度”两节。任务使用复选框；进度写当前状态、验证结果和阻塞原因。持续更新对应记录，避免逐次追加聊天过程或长日志。

```markdown
## 任务计划
- [ ] 可检查的任务一
- [ ] 可检查的任务二

## 进度
- 当前：进行中；已完成……；下一步……。
- 验证：已运行的检查与结果，或明确标注未验证及原因。
```

完成的功能保留设计和最终进度。修复时在原计划中增加必要任务并更新进度，不清空仍有效的验收条件。Git 记录历史，无需在文档中重复维护提交日志或写入本次提交自身的哈希。

## 工作顺序与提交

1. 读取相关产品设计、架构及已有功能文档，明确本次范围。
2. 更新 `design.md` 和 `execution.md` 的计划，再实现与当前任务相关的修改。
3. 运行适用检查，将结果写入执行进度；未完成、失败和缺少环境分别如实记录。
   应用改动同时按 [可运行版本交付规则](../../AGENTS.md#可运行版本交付) 构建本次 Windows release，核对 EXE 与伴随资源，在原执行文档记录来源、路径和验证范围；纯文档任务可明确复用现有产物。
   多任务按 [并行构建与产物清理规则](../../AGENTS.md#并行构建与产物清理) 使用稳定源码和独占的 `target/testN` 运行目录。项目入口已自动实现构建锁、目录独占、占用检查及安全复用；从 test1 起尝试，无法确认空闲时跳过，不强停程序。共享输出的完整构建仍需串行，边界见 desktop-runtime 原执行记录。
4. 检查差异与暂存范围，按一批完成且可独立检查的修改自动本地提交；每次交付前执行，不以每次保存文件为提交单位。
5. 最终回复默认先提供可直接启动的 EXE 链接，再说明变化、验证与提交结果。无变更不提交，提交失败不标为成功；交付边界与例外遵循 [仓库规则](../../AGENTS.md)。

提交标题固定为 `type: [vX.Y.Z] short-name - 简短说明`。`type` 按内容选用 `feat`、`fix`、`refactor`、`docs`、`test` 或 `chore`；版本取自该功能设计中的当前工作版本，功能名沿用目录中的功能名。

例如某功能目录为 `v0.1.0-floating-window/`，后续目标改为 `v0.2.0` 时，只更新原设计中的当前工作版本及执行进度，目录保持不变；提交使用 `[v0.2.0] floating-window`。

```text
docs: [v0.1.0] development-workflow - 建立项目文档与开发规则
```

## 功能索引

- [local-history 设计](v0.1.3-local-history/design.md)与[执行记录](v0.1.3-local-history/execution.md)：本地 24 小时分钟趋势与今日有效流量。

- [process-ranking 设计](v0.1.3-process-ranking/design.md)与[执行记录](v0.1.3-process-ranking/execution.md)：按需只读 CPU/工作集 Top 10。

- [disk-monitoring 设计](v0.1.3-disk-monitoring/design.md)与[执行记录](v0.1.3-disk-monitoring/execution.md)：物理磁盘读写、活动时间和五分钟趋势。

- [diagnostics 设计](v0.1.3-diagnostics/design.md)与[执行记录](v0.1.3-diagnostics/execution.md)：v0.1.3 本地诊断快照、预览与导出。

- [app-update 设计](v0.1.2-app-update/design.md)与[执行记录](v0.1.2-app-update/execution.md)：v0.1.2 在线更新、轻提示与更新公告；实现中，线上发布和实机验证按执行记录。

[hardware-info 设计](v0.1.0-hardware-info/design.md)与[执行记录](v0.1.0-hardware-info/execution.md)：先交付 Windows 硬件信息页，启动时异步查询一次并缓存，沿用现有 Sakani 样式。

| 初始版本 | 功能 | 文档 |
| --- | --- | --- |
| v0.1.0 | development-workflow：文档与开发规则 | [设计](v0.1.0-development-workflow/design.md) · [执行](v0.1.0-development-workflow/execution.md) |
| v0.1.0 | desktop-runtime：工程、窗口生命周期与交付 | [设计](v0.1.0-desktop-runtime/design.md) · [执行](v0.1.0-desktop-runtime/execution.md) |
| v0.1.0 | main-window：主窗口与监控页面 | [设计](v0.1.0-main-window/design.md) · [执行](v0.1.0-main-window/execution.md) |
| v0.1.0 | basic-monitoring：真实指标与短期历史 | [设计](v0.1.0-basic-monitoring/design.md) · [执行](v0.1.0-basic-monitoring/execution.md) |
| v0.1.0 | preferences：基础设置与持久化 | [设计](v0.1.0-preferences/design.md) · [执行](v0.1.0-preferences/execution.md) |
| v0.1.0 | cpu-temperature：可选 CPU 温度采集与趋势显示 | [设计](v0.1.0-cpu-temperature/design.md) · [执行](v0.1.0-cpu-temperature/execution.md) |
| v0.1.0 | app-network-ranking：应用网络排行（已实现，部分验收待完成） | [设计](v0.1.0-app-network-ranking/design.md) · [执行](v0.1.0-app-network-ranking/execution.md) |
| v0.1.0 | app-network-control：上下行限速与禁用网络（已实现，完整验收待完成） | [设计](v0.1.0-app-network-control/design.md) · [执行](v0.1.0-app-network-control/execution.md) |
| v0.1.0 | gpu-monitoring：GPU 采集、多卡详情与趋势 | [设计](v0.1.0-gpu-monitoring/design.md) · [执行](v0.1.0-gpu-monitoring/execution.md) |
| v0.1.0 | ip-inspection：公网出口检测与 IP 资料 | [设计](v0.1.0-ip-inspection/design.md) · [执行](v0.1.0-ip-inspection/execution.md) |
| v0.1.0 | taskbar-display：Windows 任务栏读数与常驻入口（首批实现，完整验收待完成） | [设计](v0.1.0-taskbar-display/design.md) · [执行](v0.1.0-taskbar-display/execution.md) |
