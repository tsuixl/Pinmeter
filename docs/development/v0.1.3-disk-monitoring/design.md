# disk-monitoring

> 当前工作版本：v0.1.3

## 目标与范围
- P1：磁盘页展示各物理盘读写速率、活动时间与最近五分钟趋势，不包含逐进程磁盘归属或写盘压测。

## 方案
- 遵循[产品设计](../../design/product-design.md)与[架构](../../architecture/overview.md)。core/disk 维护项目模型、有效性和有界历史；platform/disk 用 Windows PDH PhysicalDisk 通配计数器；host/disk 管理一个工作线程和主窗口授权命令，前端 features/disk 按 MVVM 接入。
- 每两秒采集一次；页面轮询续租五秒，切页五秒内释放计数器，窗口隐藏立即停采，恢复重新预热。不使用卷容量推算吞吐，不将 `_Total` 当作物理盘。活动时间为 `100 - % Idle Time`，限制有效范围 0–100%。
- 最多 32 块盘、151 帧；缺盘、失败与采样间断均断开曲线。其他平台显示不支持，不返回演示数据。
- 通用控件复用 Sakani [Select](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/forms-select--docs)、[Stat Card](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/composite-stat-card--docs)、[Line Chart](https://main--6a5a658b3681fcc010430db5.chromatic.com/?path=/docs/charts-line-chart--docs)；沿用当前受控 Sakani 源码与 tokens。HTML 仅参考结构，业务曲线沿用 Sakani 变量。

## 验收
- Windows 真实读数、切换设备、首次预热、无盘/失效、隐藏停止和历史有界可验证。
- 行为测试、类型/构建检查通过；深浅主题、窄窗和对应状态截图可复核。原生权限、设备热插拔与长期运行未实测时单独记录。
