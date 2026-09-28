# gpu-monitoring

> 当前工作版本：v0.1.0

## 目标与范围

- 根据用户要求，在既有实测基础上将 GPU 采集与独立详情页纳入 v0.1.0。支持多显卡选择、使用率、专用与共享内存、显存容量、GPU 核心温度、核心及显存频率，以及最近五分钟趋势。
- Windows 先实现；其他平台保留明确不支持状态。不调整驱动、频率或风扇，不运行压力测试。网络排行授权复用与缩进由另一任务维护，本功能只在公共装配点接入，不改变其行为。

## 方案

- Windows 按 [desktop-runtime](../v0.1.0-desktop-runtime/design.md) 每次启动统一授权，GPU helper 继承宿主权限，无单独授权入口；普通权限探针仍可独立运行。

- 遵循[技术架构](../../architecture/overview.md)和[Windows 调研](../../research/windows-monitoring.md)。C# 样例归属 `src/backend/platform/sensors/probes/`，运行脚本归属 `tools/`；编译产物与原始诊断输出放已忽略的 `src/backend/target/gpu-probe/`，精选验证数据放本功能 `assets/`。
- 使用现有且经 SHA-256 核验的 LHM 0.9.6 发布包，独立进程只开启 GPU 分组，常驻复用 Computer；记录版本、是否提权、设备及传感器标识、类型、原始单位、每轮耗时、空值与异常。使用默认空设置，不调用控制接口。
- 默认普通权限连续采样 20 轮、轮间等待 1 秒；清除动态传感器的 LHM 缓存后更新，明确区分“本轮库返回数值”与“底层驱动保证新样本”。AMD `GPU Memory Total` 在固定版本构造函数中读取，作为静态设备信息保留并标识，不将其误清空；初次发现的差分负载不作为有效基线。
- 同期用 `nvidia-smi` 获取独立进程的 NVIDIA 参考值，记录查询起止时间。两者可能共用厂商接口且时间窗不同，只验证可读性、量级和语义，不据此宣称独立硬件校准或逐帧一致。
- 采样轮数和等待时间有上限；外部诊断进程设置总超时并回收。GPU 探测不沿用现有 CPU helper 强制 PawnIO/UAC 前置条件；仅报告实际令牌权限，不主动提权。
- 初步验证明确 LHM/NVAPI 与 NVML 的显存已用量不直接等同，正式实现选择下述 D3D 字段。AMD 仅有 `GPU VR SoC` 温度时，GPU 核心温度能力保持缺失。详细实测边界见[执行记录](execution.md)。

### 正式采集与页面

- 2026-09-17 按用户要求补充 `GPU VR SoC` 温度：独立保存 `vr_soc_temperature`，不覆盖核心温度字段。未提供核心温度且有 VR SoC 指标时，总览和 GPU 详情选择后者，标题、图例和提示统一标为“GPU VR SoC 温度”，说明“未提供核心温度，显示 VR SoC 传感器读数”。核心温度暂时读取失败时不切换来源；两者都缺失仍显示“—”。历史使用各自字段，不拼接不同测温点。

- 独立 GPU helper（继承启动时统一取得的权限） 复用固定版本 LHM；GPU 使用率取 LHM 暴露的 D3D 引擎最大值，专用/共享内存取 D3D Used 字段，不回退成厂商负载或 NVAPI 已用显存。驱动报告的 `GPU Memory Total` 单独作为容量展示，不与 D3D 内存计算百分比。仅 `GPU Core` 温度可作为核心温度，频率分别取 Core/Memory。采集失败、未暴露与预热按指标保留。
- helper 位于 platform/sensors，GPU 选择与缓存清理独立于 CPU 代码，默认空控制设置且无写操作入口。每次 sample 返回有界 JSON，最多 16 张卡；设备使用系统 DeviceId，缺少稳定身份的设备不静默合并。正常采集持续复用 Computer 与引擎基线；每 30 秒只通过 [CM_Get_Device_ID_List](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_get_device_id_listw) 比较当前显示适配器身份，身份不变不重建。设备集合改变、采样中断超过 3 秒或 helper 异常重启后才重新建立基线；身份查询失败不打断仍正常的采集。GPU 不单独请求 UAC，不因 CPU 温度读取失败阻止 GPU。
- platform 使用独立 worker、单一在途请求、3 秒读取超时与 30 秒失败退避，退出回收 helper；core 保存唯一 GPU 快照，拒绝重复/倒序样本，校验各项范围并在 3 秒后标为过期。历史随既有基础帧冻结，复用五分钟/301 帧上限，不复制一套前端历史。
- 新增 `gpu` View 与 ViewModel，普通下拉选择设备并维持稳定选择；消失后回退到仍存在的设备并清空历史锚点。主导航增加 GPU，页面使用官方 StatCard、Select、Card 与 Area Chart 视觉；仅增加页面布局 CSS，复用现有图表的断档、提示卡和历史导航。
- 视觉来源：固定 `@sakaniui/react` 0.3.1，官方 [StatCard Grid](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-stat-card--grid)、[Select](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/forms-select--default)、[Area Chart](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/charts-area-chart--default) 及深色状态。HTML 仅提供页面结构参考，不作为视觉标准。

## 验收

- 两张物理 GPU 分别列出四类指标的实际字段、有效范围、缺失项及设备匹配情况；虚拟显示适配器不当作物理 GPU。
- 能复现普通权限启动、连续采集、正常退出；保存有限的原始证据及明确结论。
- 对零值、底层缓存、核显共享内存与负载口径保持谨慎；未做受控负载、提权对照、休眠恢复或其他硬件验证时明确保留未验证项。
- GPU 页真实采集、设备切换、逐项缺失、断线/过期、历史断档、深浅主题和窄窗通过；保存浏览器与本机真实读数截图。网络排行原任务的改动保持完整，相关格式、类型、核心/协议与构建检查通过后精确提交本功能。
