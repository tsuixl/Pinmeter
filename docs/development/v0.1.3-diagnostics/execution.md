## 任务计划

- [x] 2026-10-04：增加用户主动的限时排障、自动停止和独立预览导出；验证隐藏期限、取消、上限及无效读数语义。

- [x] 实现白名单诊断模型、主窗口预览及准确内容导出。
- [x] 验证敏感字段排除、文件冲突/失败和前端反馈。
- [x] 完成 Sakani 深浅截图，统一构建并本地提交。

## 进度

- 2026-10-04 本批交付：源码 `96b8c4677384bf32f09f7aec0920698832492bfd`，实际入口 `E:\dev\github\Pinmeter\src\backend\target\test6\Pinmeter.exe`。统一检查通过（Rust 167 项、前端 49 项、格式/Clippy/契约/打包工具）；完整 release/NSIS 构建、42 个运行文件及 376 个源码指纹核对通过。原生、管理员、真实升级及长期未验证项保留；[交付详情](../v0.1.0-desktop-runtime/execution.md)。

- 2026-10-04：限时排障复用唯一进程服务与基础历史，支持 30/60/120 秒、手动停止、自动结束及独立 JSON 预览导出。进程服务重建时结束旧会话，旧线程不能取消新记录；旧进程服务的租约由原会话持有并撤销。原诊断白名单不变，不混入进程名单。核心上限、无效样本及诊断租约回归已执行；最终统一检查与本次运行包见 desktop-runtime。
- 浏览器演示已验证 30 秒自动结束、15 个观测点、60 秒自动结束、结束前不能导出及明确进程名/PID 告知，[浅色预览](assets/improvements-trace-preview-light.jpg)、[深色预览](assets/improvements-trace-preview-dark.jpg)。演示导出如实显示“未导出文件”，未写用户下载目录。沿用已核对的 Sakani Modal/Card/Select/Button，HTML 仅供布局；真实管理员采样、收起期间计时和下载目录写入仍未实测，不以演示替代原生验收。

- 已对照 Sakani Modal/Button/Alert，沿用 0.3.1 组件及 tokens；[浅色](assets/p1-preview-light.jpg)、[深色](assets/p1-preview-dark.jpg)与[420×400 弹窗](assets/p1-narrow-dark.jpg)检查通过，内容区独立滚动且关闭/导出按钮可见。演示导出明确提示“未写入文件”，不伪装成功；真实文件写入的准确性与冲突由平台测试覆盖。HTML 仅参考布局，真实主窗口和系统下载目录交互仍未实测。统一运行包见 [desktop-runtime](../v0.1.0-desktop-runtime/execution.md)。

- 当前：本轮实现与本地交付完成，诊断提供本地快照，不自动上报。已包含在 `test2/Pinmeter.exe`，来源与完整资源哈希见 desktop-runtime 交付记录。
- 已接入设置页预览、同一快照令牌导出、UTF-8 JSON 与独占文件创建。核心白名单测试和平台文件冲突/失败测试、Rust 工作区测试、前端类型及最终 33 项测试通过；原有 3 项外部环境测试仍跳过。真实主窗口与系统下载目录交互尚未实测，不能用演示导出代替。
