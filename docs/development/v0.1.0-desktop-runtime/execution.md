## 任务计划

- [x] 2026-10-04 v0.1.3 发布：准备公告和材料，推送并等待发布提交三平台 CI，通过完整 Windows 构建及资源/签名/源码/压缩包检查；草稿上传核对后公开预览 Release，验证匿名下载并更新发布记录。

- [x] 2026-10-04：检查并减少已证实的常驻重复工作，复核现有全进程测量的身份、权限及结果有效性；执行针对性检查和本次完整 Windows 构建。
- [ ] 在可读取本次管理员宿主且保持窗口状态的环境完成两种状态各 120 秒预热 + 600 秒测量及 8 小时稳定性；现有实例不强停，无法完成时保留明确未验证项。

- [x] v0.1.3：完成 P1 六项集成、隐藏停止刷新、统一公告、项目检查和本地 Windows 完整运行包。

- [x] 同步 v0.1.1 版本、发布入口和说明，推送准备提交并确认三平台 CI。
- [x] 从该干净提交重建 Windows 包，生成并核对安装器、便携 ZIP、源码 ZIP 与校验文件。
- [x] 创建并发布 v0.1.1 预览 Release，核对标签、附件摘要与匿名下载。

- [x] 将已选设计稿导出为宿主多尺寸图标，接入应用、托盘、NSIS 与 GitHub README 展示。
- [x] 检查图标格式、深浅背景和小尺寸，构建并核对本次 Windows 运行包与安装器。

- [x] 根据名称和产品定位生成应用/安装共用图标设计稿，保存原图与生成说明，检查透明通道及视觉轮廓。

- [x] 修复远端 CI 的 PowerShell 模块路径和非 Windows 导入问题，推送并确认检查结果。
- [x] 从通过检查的提交重建、核对发布附件，并上传首个 Windows 预览版草稿。
- [x] 按用户确认公开现有仓库，完成预览版发布和中英文首页下载入口。

- [x] 完成卸载失败保护、按安装路径清理自启、共享资源准备及 UNIC 声明补正，执行针对性回归和完整项目检查。
- [x] 从已提交源码构建 Windows release/NSIS，核对全部资源、驱动安装器排除及适用辅助组件启动行为。

- [x] 建立共享核心、平台适配与宿主工程，接入窗口、托盘、单实例和统一启动授权。
- [x] 实现 Windows 构建互斥、编号运行包独占、安全复用及资源路径/哈希核对。
- [x] 接入 AGPL 和第三方声明的安装及便携资源映射，完成已有 release 构建和打包检查。
- [x] 修正 Windows 资源准备顺序，以无辅助组件产物的独立源码快照验证 prepare 和宿主编译检查。
- [x] 修复卸载缺少网络辅助程序时跳过规则恢复，以及卸载未清理自启任务的问题。
- [x] 补齐 UNIC 版权行，取消分发 PawnIO 安装器并保留实际依赖声明。
- [x] 整理对应源码 ZIP、完整便携 ZIP、安装器、中英文说明及 SHA-256 清单。
- [ ] 验证实际主程序安装/升级/卸载及管理员网络恢复。
- [ ] 从最终干净提交构建正式产物，验证真实安装、升级、卸载、驱动缺失和管理员网络恢复。
- [ ] 补测正式宿主托盘/关闭/Explorer 恢复、最小化资源基线、VPN、100%/150%/混合 DPI、多屏、睡眠、冷启动及八小时稳定性。
- [ ] 完成 macOS/Linux 宿主及实机验证；按实际需要拆分职责集中的模块。

## 进度

- 2026-10-04 19:54（Asia/Shanghai）已公开发布 [v0.1.3 Windows 预览版](https://github.com/tsuixl/Pinmeter/releases/tag/v0.1.3)，保留旧 Release。发布提交 `0165ecfbd01ce17fc5120456deaedf647de09be8` 已推送，附注标签 `v0.1.3` 解引用、Release target、源码 ZIP 及二进制构建来源均对应此提交；该提交的 [Windows/macOS/Linux CI](https://github.com/tsuixl/Pinmeter/actions/runs/37198913197) 全部通过。后续首页与发布记录只改文档，不移动标签或重建已发布 EXE。
- 从该干净提交经 `PINMETER_HOLD_DELIVERY=1 node tools/desktop.mjs build -- --locked` 构建最新前端、宿主、辅助组件、NSIS 及 Tauri 更新签名，运行入口 `E:\dev\github\Pinmeter\src\backend\target\test16\Pinmeter.exe`。385 个源码指纹及 42 个运行文件路径/哈希复核通过；EXE SHA-256 `c9d4e54ceeda22929172966ea17eb91cfa1f34e86ae6a03eb61e3dfa14ad16a7`，安装器 SHA-256 `aa38d543c4ec715d8a096a72325a3a9d61c6619f7528962fae1f729aedd50344`。安装器 PE 产品版本 0.1.3，与同版本签名和清单核对通过；应用与安装器 Authenticode 状态均为 NotSigned，发布说明已区分其与 Tauri 签名。
- 发行附件共七项：安装器、`.sig`、完整便携 ZIP、对应源码 ZIP、`SHA256SUMS.txt`、`latest.json`、`release-notes.json`。便携 ZIP 按运行清单白名单生成，包含 42 个运行文件和一份使用说明，逐条哈希一致；源码 ZIP 来自 `git archive`，583 个跟踪文件及构建所需 385 个指纹均匹配。发布前草稿附件大小/服务器摘要与本地一致；发布后七个附件均可匿名 HTTP 200 下载，大小与 SHA-256 再次匹配。预览标记保持，不推进稳定 latest 入口（匿名稳定入口仍为 HTTP 404）。本地证据集中在 test16 的 build-source/source-manifest/package-hashes/delivery-verification/release-files/release-upload-verification/release-verification JSON；公开 ZIP 不含本机配置、日志及这些带本机路径的元数据。
- 辅助组件只读冒烟：CPU 一次 sample 返回 permission_denied，GPU 三帧 normal / 1 设备，二者 stderr 为空且 exit 0；仅验证协议、可用读取和退出，未安装驱动或操作网络规则，证据为 test16/helper-smoke.json。现有 test15 与提权实例未停止，未启动新主程序；真实管理员、安装/升级/卸载、VPN/网络恢复、多屏混合 DPI、休眠与长期资源占用继续保持未验收。构建/目录独占保留至发布与下载核对结束；切换前完全退出旧版，并保留完整运行目录。

- 2026-10-04 优化交付完成：从干净源码 `96b8c4677384bf32f09f7aec0920698832492bfd` 经 `PINMETER_HOLD_DELIVERY=1 node tools/desktop.mjs build -- --locked` 生成 release 和 NSIS。实际运行入口 `E:\dev\github\Pinmeter\src\backend\target\test6\Pinmeter.exe`，产品版本 0.1.3，57,425,920 字节。test1–5 因现有进程占用或管理员子进程路径无法确认跳过；构建锁及 test6 独占保持至交付核对结束，未停止用户旧版。
- 构建与复核：42 个运行文件按当前资源配置逐项核对源/副本路径及 SHA-256；376 个源码文件构建前后与交付复核一致。主 EXE SHA-256 为 `14e8b78c3b3cbb16a6497fc3662a979ffd2bd98ccda08939db7ff40b923551bc`；安装器与签名已复制到 `test6/update-artifacts/` 并再次验签/核对 PE 产品版本，安装器 SHA-256 为 `d7e97455839d1a864572a915ca8ff16e43174eb9ac33c78e86b2999ba0055aa3`。完整 sensors、network、network-control、licenses 目录须随 EXE 保留。来源与证据为同目录 `build-source.json`、`source-manifest.json`、`package-hashes.json`、`delivery-verification.json`、`helper-smoke.json`、`final-check.log`。
- 最终统一 `tools/dev.ps1 check` 全部通过：Rust 167 项通过、3 项原有外部环境测试跳过，前端 49 项/16 个文件通过，Clippy `-D warnings`、Rust/前端格式、受控契约、前端生产构建、打包/构建互斥/隔离卸载回归通过。MSVC 信息性 linker 输出及前端主 chunk 530.63 kB 的体积提示保留，未调高告警阈值；不据此宣称已满足常驻资源预算。
- 运行边界：本包 CPU/GPU helper 启动、协议及正常退出通过；CPU 如实返回 `permission_denied`，GPU `normal` 且枚举 1 张设备，stderr 均为空。初次冒烟脚本将 UTF-8 BOM 写入 stdin 导致无回复，改为无 BOM 的既有 `sample` 协议后复验通过，应用代码未改变。保留旧实例，未将其激活当作新版启动；本次管理员主程序、真实安装升级/下载目录导出、Explorer/休眠/多屏混合 DPI、10 分钟预算和 8 小时稳定性仍未实测。各新增界面的浏览器证据见各原功能 execution，不能代替原生验收。网络控制与故障恢复仅登记问题，源代码和真实规则未修改；未推送、打标签或发布。

- 2026-10-04：减少 GPU 重复快照克隆，磁盘/进程空闲改为请求/显隐/退出通知唤醒，保持原采样频率、租约和有效性。限时排障是明确操作后的有期限后台进程需求；详见 diagnostics 原功能。现有测量脚本未被改写；当前旧版 test4 的管理员子进程路径不可读，保留实例，未把旧启动器的占用当作新版基线。
- 本批修复库单元测试缺少 Common Controls v6 清单及重复嵌入资源的问题：MSVC 由链接器统一嵌入同一清单，Tauri 资源继续提供图标/版本但不重复携带 RT_MANIFEST。宿主最终 35 项测试、全部二进制目标及 release 构建通过，未绕过测试。链接参数依据 [Cargo 构建脚本](https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-arg)。

- v0.1.3：P1 六项实现、浏览器检查与完整本地运行包已交付，各功能分别本地提交。原生隐藏事件同时暂停磁盘/进程/历史页轮询。已有安装版 `pinmeter-host` 与辅助组件正在运行，本轮保留实例，不进行冷启动替换、登录或管理员交互验收。
- [v0.1.3 公告弹窗](assets/p1-release-notes-light.jpg)已核对，六项变化与统计边界完整展示，浏览器控制台无错误。Sakani 决定组件视觉，HTML 只供结构参考。
- 统一入口 `tools/dev.ps1 check` 已完整通过：helper/安装清理与构建互斥回归、310 个本地 Markdown 链接与分层检查、Rust 格式/工作区测试/Clippy、生成契约一致性、前端 production 构建、33 项测试与格式检查。原有 3 项需要管理员 ETW/Explorer 环境的测试保持跳过。更新演示的目标版本随应用升为 0.1.3 后改为 0.1.4，相关测试按目标版本断言，修复初次检查中的旧固定版本断言后已重跑全部检查成功。
- Windows v0.1.3 交付：从干净提交 `4d2b1143e7d280345cc64f3339674b1395661a5b` 执行 `PINMETER_HOLD_DELIVERY=1 node tools/desktop.mjs build -- --locked`，release 与 NSIS 构建成功；实际运行入口为 `E:\dev\github\Pinmeter\src\backend\target\test2\Pinmeter.exe`，文件版本 0.1.3、16,344,064 字节。test1 因已有进程路径无法确认而跳过；项目构建锁与 test2 预留保持至交付，未覆盖既有实例。
- 资源/来源：按当前 Tauri Windows 资源配置核对全部 40 个运行文件与来源 SHA-256，325 个源码文件指纹构建前后及交付前一致。记录在同目录 `build-source.json`、`source-manifest.json`、`package-hashes.json`、`delivery-verification.json`。主 EXE SHA-256 为 `1cae1eb8bafb3876c8e14830f834b972afe7ea2f04ffe9b45c0e2b2fa01d867c`。完整 sensors、network、network-control、licenses 目录必须随 EXE 保留。
- 更新材料：本次安装器和 `.sig` 已另存于 `test2/update-artifacts/`，与构建源哈希一致，并用应用内嵌公钥独立验证 Tauri/minisign 签名；安装器 SHA-256 为 `aad3fb2ea6c59b4cc619e5355c0e68089185ec9f05e3cb672cb52e6f25cd9b49`。同目录提供 v0.1.3 待发布公告与清单；未推送、打标签、上传或发布，Tauri 更新签名不代表 Windows Authenticode 签名。
- 运行验证边界：随包 CPU/GPU helper 在当前权限下均输出有效协议并以 0 退出；CPU 温度为 `permission_denied`，GPU 枚举 1 个设备，记录见 `test2/helper-smoke.json`。P1 原生磁盘/进程只读探针和历史退出/重载测试通过；保留正在运行的旧实例，未将启动旧窗口当作新版冷启动成功。管理员托盘冷启动、真实下载目录导出、实际安装升级、Explorer/休眠/混合 DPI 和真实 24 小时长期运行仍未验收。切换时先完全退出旧版，再启动新入口；移动时保留整个 test2 运行目录。

- v0.1.2：更新安装与普通退出复用设置等待及网络清理；安装器启动失败重建采集和托盘。NSIS 增加与实际 EXE 路径绑定的安装标记，签名及公告材料随构建生成。当前项目检查通过，真实管理员更新/安装与长时场景仍待验收，证据见 [app-update](../v0.1.2-app-update/execution.md)。
- v0.1.2 本次运行包为 `E:\dev\github\Pinmeter\src\backend\target\test1\Pinmeter.exe`，构建来源为干净提交 `6986f39`，完整资源 40 项及源码 287 项复核通过。签名安装器与待发布清单位于同目录的 `update-artifacts/`；helper 启动结果及准确哈希集中记录在上述 app-update 执行文档。正式主窗口、实际安装升级、休眠/Explorer/多屏与八小时常驻仍未验收，不能将本次构建成功标记为 S4 完成。

- v0.1.1 已于 2026-10-02 20:04（Asia/Shanghai）[公开发布为 Windows 预览版](https://github.com/tsuixl/Pinmeter/releases/tag/v0.1.1)，保留 v0.1.0。发布源码为 `368e1adad6694c488c201680a74e9a0f7a27973c`，其 [Windows/macOS/Linux CI](https://github.com/tsuixl/Pinmeter/actions/runs/37002886261) 全部通过；已推送的附注标签 `v0.1.1` 解引用到同一提交，Release target、源码 ZIP 和 Windows 构建来源一致。发布后的本段记录不改变标签或构建源码。
- Windows 从上述干净提交经 `PINMETER_HOLD_DELIVERY=1 node tools/desktop.mjs build -- --locked` 重建，复用已确认空闲的 `src/backend/target/test1/` 并保持构建锁与目录独占至交付结束。运行入口 `src/backend/target/test1/Pinmeter.exe` 的产品版本为 `0.1.1`，SHA-256 为 `465B32276B428AF6E13E3591F54AC6B428C888D266A6808BC0BF09B0C7BE0C07`；40 个完整运行文件路径与哈希通过核对。
- 四个发行附件保存在 `src/backend/target/test1/release-assets/`：`Pinmeter_0.1.1_x64-setup.exe`、`Pinmeter-0.1.1-preview-windows-x64.zip`、`Pinmeter-0.1.1-source.zip`、`SHA256SUMS.txt`。便携包包含 40 个运行文件及两份去除本机绝对路径的构建来源资料，42 个 ZIP 条目逐项哈希核对通过；源码包 331 个文件逐一核对 Git blob ID。安装器复制哈希一致，上传后 GitHub 的四个附件摘要/长度均与本地一致；公开后再匿名下载四个附件，长度和 SHA-256 均通过。公开校验值见 [SHA256SUMS.txt](https://github.com/tsuixl/Pinmeter/releases/download/v0.1.1/SHA256SUMS.txt)，详细本地证据为运行目录内 `release-verification.json` 与 `public-release-verification.json`。
- 本次运行包的 CPU/GPU helper 启动并正常退出：GPU 返回 normal 与真实指标，CPU 返回缺少 PawnIO 的 unsupported；未安装驱动、未启动需 UAC 的主程序、未执行真实安装/升级/卸载。发布说明保留管理员、VPN、多 DPI、多屏、休眠和长时验收边界；CI 与打包通过不替代这些实机验证。

- v0.1.1 发布准备（2026-10-02）：已核对用户推送的 `bafa356` 与远端 main 一致，选择补丁版本并保留预览标记。同步前后端清单、Cargo/npm 锁文件中的项目版本、宿主及请求 User-Agent；中英文首页更新新版本下载入口，历史截图与功能目录名保持原版本。类型、清单格式、Rust 格式、锁文件元数据及工程检查通过；从发布准备提交重建 0.1.1 二进制，未复用 0.1.0 包。发布流程及最终核对结果见上文。

- 2026-10-02 图标接入：采用用户确认的设计稿，经锁定 Tauri CLI 2.11.4 `icon` 命令导出到临时目录后替换已有宿主图标集合；移除无引用旧 SVG。Windows ICO 包含 16/24/32/48/64/256 像素，PNG 尺寸与文件名一致且角点透明；NSIS 安装和卸载图标显式引用 `icons/icon.ico`，窗口与托盘仍复用默认宿主图标。中英文 README 展示同一 PNG，未修改 GitHub 账户头像、未推送或发布。前端标识和深浅/折叠截图见 [main-window 执行记录](../v0.1.0-main-window/execution.md)。
- 本机原缺少 npm 依赖与 Rust，已使用锁文件安装前端依赖，并从 Rust 官方安装 1.98.1 minimal 工具链（未修改全局 PATH）；项目 `prepare` 辅助组件构建及其既有检查通过。首次 npm 下载停滞后，使用缓存、关闭 audit 并设置有界下载重试重新执行 `npm ci` 成功，未改变锁文件。
- 本次交付来源为干净提交 `ec7c30cba9d57711f29601c728957355d35cb26d`，通过 `PINMETER_HOLD_DELIVERY=1 node tools/desktop.mjs build -- --locked` 完成前端、helper、Rust release 与 NSIS 构建；构建锁与 test1 独占保留至交付核对结束。运行入口为 `src/backend/target/test1/Pinmeter.exe`，完整 40 个运行文件路径与源文件 SHA-256 一致；安装器由构建缓存复制到同目录 `Pinmeter_0.1.0_x64-setup.exe`，复制哈希一致。EXE SHA-256：`5B32BE66C03DEDBEE93E10ABD2D9E0C428A44FE0BED27DCC42DF6AE3B80F6884`；安装器 SHA-256：`14D1474B7DD401A868E83AA9B92D185FE8B08D1AFC46C382FF4E8E5EA5D632A9`。构建只有 MSVC 输出“创建库和对象”的 linker_messages 提示，无编译错误。
- 已将两个 EXE 作为数据读取 Windows PE 图标资源，应用与安装器的 16/24/32/48/64/256 六个图标帧均与新 ICO 逐字节一致；核对结果保留在运行目录 `icon-verification.json`。从运行包启动 CPU/GPU helper 均正常退出：GPU 两帧由 warming 转 normal 并返回真实指标；CPU 明确返回缺少 PawnIO 驱动的 unsupported，未安装驱动。未启动需要 UAC 的主程序，未执行真实安装/卸载、原生托盘、多 DPI 或 Shell 缓存刷新验收；图标资源验证不替代这些实机检查。
- 本批最终 `node tools/check-project.mjs` 通过（236 个本地 Markdown 链接及工程约束），类型、修改文件格式与差异检查通过。前端深浅主题/折叠截图已保存；完整 UI 视觉验收状态仍见 main-window。GitHub README 改动仅在本地提交，线上须推送后生效；未上传或替换已有 Release。

- 图标设计（2026-10-02）：交付 [透明 PNG 设计稿](assets/pinmeter-icon-concept-v1.png)，内置 imagegen 生成，完整提示词见 design.md。实际尺寸 1254×1254、32 位 ARGB，角点 alpha=0，中心 alpha=254；SHA-256 为 `E7F540626D1D7B397C7576DD38B0729FDC983DCC8E5A36FE096AF20098C98D7F`。已目视确认单一图标、橙色固定针/仪表轮廓、白色指针及无文字；色值存在生成偏差，像素级多尺寸适配与深浅背景视觉验收未完成。
- 本轮仅增加设计文档与预览素材，运行图标和安装配置保持现状，EXE 未重新构建；本工作区不存在 `src/backend/target/`，没有可核对并复用的 Windows EXE。未执行应用、安装器或管理员场景测试。
- 设计稿文档检查：`node tools/check-project.mjs` 通过（231 个本地 Markdown 链接、功能文档与工程约束），`git diff --check` 通过；原图复制后 SHA-256 与生成文件一致。

- 预览发布准备完成：源码 `cd6f3b1` 的 [GitHub CI](https://github.com/tsuixl/Pinmeter/actions/runs/36585620873) 在 Windows、macOS 和 Linux 全部通过。Windows 运行包与 NSIS 已由同一干净提交重新构建，入口为 `src/backend/target/test31/Pinmeter.exe`，40 个运行文件、270 个构建输入及便携 ZIP 的 41 个条目核对通过；主 EXE SHA-256 为 `31DD32643F086D027BE917FDCF70472F17DA5C1DE6DA8263824D6CC3E84C8604`。真实官方下载与签名验证也在继承 Core 模块路径的环境下通过，未执行安装器。
- 2026-09-29 按用户确认将现有仓库改为公开并发布 [v0.1.0 Windows 预览版](https://github.com/tsuixl/Pinmeter/releases/tag/v0.1.0)，保留预发布标记。未登录 API 已核对仓库公开、Release 已发布、标签指向构建源码 `cd6f3b1ea194c2cfd3b8b3773253df0e5360191c`，以及安装器、完整便携 ZIP、对应源码 ZIP、SHA256SUMS.txt 四个附件的大小和 SHA-256。中英文 README 已提供下载入口；实际安装/卸载、缺驱动完整安装、管理员网络恢复、VPN 及长时环境仍待实测。发布后的首页更新仅涉及文档，复用已核对的 `test31`，未重新构建 EXE。
- 发布复核：四个附件均可匿名下载，HTTP 200 且长度与发布记录一致；本地 `test31/Pinmeter.exe` 哈希仍与构建记录一致。`node tools/check-project.mjs` 通过（229 个本地 Markdown 链接及工程约束），`git diff --check` 通过。

- 首次发布前已取得远端 CI 日志：Windows 资源准备因 Windows PowerShell 继承 Core 模块搜索路径而找不到 Get-FileHash；macOS/Linux Clippy 拒绝 Windows 专用 ProcessBytes 的无条件导入。已按运行 shell 恢复内置模块优先级，并限制该导入的编译平台；驱动签名及自启的 PowerShell 调用同时固定对应系统模块来源。修复后的远端 CI 和重新构建结果见上述发布记录。
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
