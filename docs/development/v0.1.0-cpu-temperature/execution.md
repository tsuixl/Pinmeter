## 任务计划

- [x] 将驱动安装改为固定官网入口和重新检测，移除安装命令及随包安装器，检查深浅主题、失败反馈和打包拒绝回归。

- [x] 接入独立温度采集、逐项状态、总览与 CPU 双轴趋势。
- [x] 保留只读驱动检测及温度重试，打开官网仅接受固定网址，无安装执行入口。
- [ ] 验证正式主窗口打开官网、用户手动安装/重启后的真实采集，以及其他硬件和异常退出场景。

## 进度

- Windows 宿主启动统一请求权限，温度辅助程序继承权限；缺驱动、权限不足、超时、失败与不支持独立呈现。有效温度复用五分钟历史，无效区间断开。
- 官网入口、打开失败/重试、重复点击保护、显式重新检测、切页后清除旧反馈和温度恢复的浏览器回归通过；模拟状态不代表实际安装驱动。类型、23 项前端测试及完整项目检查通过；7 项打包回归包含拒绝历史缓存和产物中的 PawnIO 安装器。既有温度选择与平台测试继续通过。
- Sakani 0.3.1 官方 [Button](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/core-button--primary) 的默认/loading/disabled，以及 [Alert](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/story/composite-alert--info) 的 info/danger/dark 已实际打开对照；[官方提示](assets/driver-reference-alert.png)、[官方深色错误](assets/driver-reference-alert-danger-dark.png)与应用[浅色](assets/driver-download-light.png)、[深色错误](assets/driver-download-error-dark.png)、[窄窗](assets/driver-download-narrow-light.png)已复核。直接复用标准控件，HTML 仅作布局参考；原生 DPI 和正式宿主交互仍未验收。
- 运行包和安装包来源、资源核对及未验证项统一见 [desktop-runtime](../v0.1.0-desktop-runtime/execution.md)。本次不安装、卸载或更改用户已有驱动。
