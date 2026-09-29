## 任务计划

- [x] 修复远端 CI 的 PowerShell 模块路径和非 Windows 导入问题，推送并确认检查结果。
- [x] 从通过检查的提交重建、核对发布附件，并上传首个 Windows 预览版草稿。
- [ ] 确认私有仓库的公开范围，完成预览版发布和首页下载入口。

- [x] 完成卸载失败保护、按安装路径清理自启、共享资源准备及 UNIC 声明补正，执行针对性回归和完整项目检查。
- [x] 从已提交源码构建 Windows release/NSIS，核对全部资源、驱动安装器排除及适用辅助组件启动行为。

- [x] 建立共享核心、平台适配与宿主工程，接入窗口、托盘、单实例和统一启动授权。
- [x] 实现 Windows 构建互斥、编号运行包独占、安全复用及资源路径/哈希核对。
- [x] 接入 AGPL 和第三方声明的安装及便携资源映射，完成已有 release 构建和打包检查。
- [x] 修正 Windows 资源准备顺序，以无辅助组件产物的独立源码快照验证 prepare 和宿主编译检查。
- [x] 修复卸载缺少网络辅助程序时跳过规则恢复，以及卸载未清理自启任务的问题。
- [x] 补齐 UNIC 版权行，取消分发 PawnIO 安装器并保留实际依赖声明。
- [x] 整理对应源码 ZIP、完整便携 ZIP、安装器、中英文说明及 SHA-256 清单。
- [ ] 验证远端 CI；实际主程序安装/升级/卸载及管理员网络恢复仍须验收。
- [ ] 从最终干净提交构建正式产物，验证真实安装、升级、卸载、驱动缺失和管理员网络恢复。
- [ ] 补测正式宿主托盘/关闭/Explorer 恢复、最小化资源基线、VPN、100%/150%/混合 DPI、多屏、睡眠、冷启动及八小时稳定性。
- [ ] 完成 macOS/Linux 宿主及实机验证；按实际需要拆分职责集中的模块。

## 进度

- 预览发布准备完成：源码 `cd6f3b1` 的 [GitHub CI](https://github.com/tsuixl/Pinmeter/actions/runs/36585620873) 在 Windows、macOS 和 Linux 全部通过。Windows 运行包与 NSIS 已由同一干净提交重新构建，入口为 `src/backend/target/test31/Pinmeter.exe`，40 个运行文件、270 个构建输入及便携 ZIP 的 41 个条目核对通过；主 EXE SHA-256 为 `31DD32643F086D027BE917FDCF70472F17DA5C1DE6DA8263824D6CC3E84C8604`。真实官方下载与签名验证也在继承 Core 模块路径的环境下通过，未执行安装器。
- GitHub `v0.1.0` 预览草稿已附安装器、完整便携 ZIP、对应源码 ZIP 和 SHA256SUMS.txt；服务器返回的每个附件大小和 SHA-256 与本地一致。草稿说明明确实际安装/卸载、缺驱动完整安装、管理员网络恢复、VPN 及长时环境仍待实测；仓库仍为私有，尚未公开发布。

- 首次发布前已取得远端 CI 日志：Windows 资源准备因 Windows PowerShell 继承 Core 模块搜索路径而找不到 Get-FileHash；macOS/Linux Clippy 拒绝 Windows 专用 ProcessBytes 的无条件导入。已按运行 shell 恢复内置模块优先级，并限制该导入的编译平台；驱动签名及自启的 PowerShell 调用同时固定对应系统模块来源。仍待推送后的 CI 验证，发布包将从最终通过的提交重新构建。
- 本地验证：将模块路径置为不兼容的 Core 目录后，资源准备及完整项目检查通过；新增子进程回归确认内置 Get-FileHash 可重新解析。Rust、前端 23 项测试、格式/类型/契约、打包、自启及隔离卸载检查通过。远端检查结果随后单独核对。
- 首轮远端复验已通过 Windows 资源准备，随后发现全新 Windows 检出将文本转换为 CRLF，导致设计变量逐字比较失败；新增根文本属性统一 LF，二进制资源不变。非 Windows 的协议解码测试也需要 ProcessBytes，导入条件补为 Windows 或 test；保持严格 Clippy 和生成文件检查，未绕过失败步骤。

- 后续 CPU 驱动已改为点击后自动从官方来源下载、校验并安装，仍不捆绑安装器；最新 `test29` 运行包、对应源码和验证范围见 [cpu-temperature](../v0.1.0-cpu-temperature/execution.md)。

- 主窗口最小化隐藏任务栏按钮并保留托盘；关闭默认询问最小化或退出。启动统一请求权限，辅助进程继承权限；正常退出复用网络清理用例。
- 项目构建入口覆盖前端、辅助组件和 Windows release；运行包包含完整伴随资源，检查名称、路径、清单和来源哈希。既有构建、打包损坏用例、锁与安全复用检查通过，不代表安装和管理员业务场景全部通过。
- 资源准备由同一项目入口和构建锁执行。独立源码快照起初没有 sensors/network/network-control 产物，仅复用已校验的下载缓存；prepare 生成全部组件且无 PawnIO 安装器，随后工作区 Cargo check 通过（复用编译依赖缓存）。这不是全新 Windows 或远端 CI 验收。
- 完整 tools/dev.ps1 check 通过，含 Rust 测试/Clippy/格式/契约及前端类型、生产构建、23 项测试和格式；3 项已有管理员/真实桌面测试保持忽略。7 项打包回归、自启隔离测试、构建锁回归通过；实际编译运行的 NSIS 夹具验证 helper 缺失/失败均保留文件、成功允许卸载、升级跳过清理。夹具不调用真实网络规则或用户启动项。
- [第三方声明](../../../src/backend/host/resources/legal/THIRD-PARTY-NOTICES.txt)已按锁定的 5 个 unic-* 源码版权行补齐；[传感器来源](../../../src/backend/platform/sensors/licenses/SOURCES.txt)明确驱动及安装器由用户从官网下载，包内仅保留实际传感器依赖。现有品牌图标与来源声明按用户选择保留。IP 服务条件及其他未验证范围继续见 [ip-inspection](../v0.1.0-ip-inspection/execution.md)；不因这次修正宣称全部第三方条件已获确认。
- 性能测量尚未完成完整常驻预算；最小化采集受窗口状态干预的结果不计为通过。旧测试原始记录和截图已移出公开基线，保留上述验证边界。
- 本轮交付由干净源码 `6e41a6c` 经 `node tools/desktop.mjs build -- --locked` 完整生成前端、辅助组件、Windows release 与 NSIS。运行入口为 `src/backend/target/test27/Pinmeter.exe`；构建锁及目录独占保持到交付完成，原运行实例保留。268 个构建输入前后及交付复核一致，40 个运行文件与清单、构建来源三方 SHA-256 一致；NSIS 生成脚本和运行包均不含 PawnIO 安装器。主程序 SHA-256 为 `AC887336F09119198E7E61B419A0BFBA50C8317240E20675412ED09E516AEA9A`。
- 同目录提供 `Pinmeter_0.1.0_x64-setup.exe`、`Pinmeter-0.1.0-preview-windows-x64.zip`、`Pinmeter-0.1.0-source.zip`、中英文 README.txt 和 SHA256SUMS.txt。便携 ZIP 仅含 40 个运行文件及说明，逐项路径和解压内容哈希通过；不包含本机构建清单、测试脚本或诊断日志。源码 ZIP 对应上述构建提交，安装器与原生成文件哈希相同，详细校验记录保留在忽略的运行目录。
- 从交付目录运行 CPU/GPU helper 后正常退出、stderr 为空；非管理员 CPU 明确返回 permission_denied，GPU 状态 normal、识别 2 张设备，PawnIO 状态检测返回 installed，旧安装命令返回拒绝码。交付网络控制 helper 自检通过。管理员隔离恢复请求由 Windows 返回授权取消，测试未开始、未创建测试规则或启动项，延迟执行已禁用；不将夹具通过当作真实管理员恢复通过。现有管理员实例继续运行，本轮未启动新主窗口或执行真实应用安装/卸载，官网入口原生交互及长期场景仍未验证。本轮未推送、公开仓库或发布版本。
