## Why

TokenBall 需要先从完整产品设想收敛为可开发、可验证的桌面 MVP。首版优先接入 CLIProxyAPI，跑通“配置渠道 -> 同步额度 -> 标准化窗口 -> 桌面余量球展示 -> 失败时展示缓存”的最小可用闭环。

## What Changes

- 创建 Tauri 2 + Vue 3 桌面应用 MVP，包含余量球窗口、悬停详情面板和简化管理窗口。
- 引入统一额度领域模型，用于表达 Provider、连接、账号、额度窗口、当前关键窗口和汇总状态。
- 接入 CLIProxyAPI，支持配置连接、测试连接、读取账号、同步额度状态和处理基础错误。
- 保存本地最新状态缓存，同步失败时继续展示上次成功数据并标记数据时间。
- 首版暂不接入火山引擎、历史趋势、自动更新、完整告警系统和系统级凭证安全存储。
- 首版凭证可保存在本机配置或数据库中，但日志与 UI 错误必须避免泄露完整密钥。

## Capabilities

### New Capabilities

- `desktop-shell`: 桌面应用外壳，包括 Tauri 工程、窗口、托盘入口、基础设置和本地缓存初始化。
- `quota-domain`: 统一额度模型、当前关键窗口选择、多账号汇总和缓存降级行为。
- `cliproxy-provider`: CLIProxyAPI 连接配置、连接测试、账号读取、额度同步和错误映射。
- `quota-orb-ui`: 余量球窗口、轮播展示、悬停详情面板和刷新交互。

### Modified Capabilities

- None.

## Impact

- 新增 Tauri 2、Vue 3、TypeScript、Vite、Pinia、Rust、Tokio、Reqwest、Serde、SQLite/SQLx 和日志依赖。
- 新增 Rust 后端模块：commands、providers、quota、storage、scheduler、events 和 windows。
- 新增 Vue 前端视图、组件、stores、composables、services 和类型定义。
- 新增本地 SQLite 表，用于保存连接配置、账号、最新额度快照、额度窗口和基础设置。
- CLIProxyAPI Management API 成为 MVP 的唯一外部数据源。
