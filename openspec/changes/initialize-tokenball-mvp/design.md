## Context

TokenBall 当前只有产品与技术方案文档，尚无应用工程。MVP 的目标是先做一个本地桌面工具，接入 CLIProxyAPI 并通过余量球展示 Codex Plus 账号可用额度。首版需要覆盖跨端桌面窗口、Rust 后端同步、Vue 前端展示、本地缓存和统一额度模型，因此需要在实现前固定模块边界。

约束：

- 首版只接入 CLIProxyAPI，不接入火山引擎。
- 首版只保存最新状态缓存，不实现历史趋势。
- 首版可以暂不接系统 Keychain 或 Credential Manager，但日志与用户可见错误必须脱敏。
- 业务计算放在 Rust 后端，前端只展示后端给出的标准化状态。

## Goals / Non-Goals

**Goals:**

- 建立 Tauri 2 + Vue 3 + TypeScript + Rust 的桌面应用骨架。
- 提供余量球、悬停详情面板和简化管理窗口。
- 建立统一额度模型，支持账号、多个额度窗口、当前关键窗口和多账号汇总。
- 接入 CLIProxyAPI Management API，完成连接测试、账号读取和额度同步。
- 将最新连接、账号和额度状态写入本地缓存，同步失败时保留上次成功数据。
- 提供手动刷新和后台定时同步。

**Non-Goals:**

- 不接入火山引擎、千问、Claude、OpenAI 官方账单或其他 Provider。
- 不实现完整历史趋势、长期快照压缩、Usage Queue 深度分析或复杂报表。
- 不实现完整告警系统、自动更新、开机启动、多设备同步、团队协作或 Web 管理后台。
- 不在首版实现系统级安全凭证迁移；仅保证本机保存和日志脱敏。

## Decisions

### Tauri Rust 后端负责业务与外部请求

前端不得直接请求 CLIProxyAPI 或读取本地数据库。Vue 通过 Tauri Commands 获取连接、额度和设置，通过 Events 接收同步更新。

选择原因：Management Key 和业务聚合逻辑留在 Rust 侧，后续接入更多 Provider 时 UI 不需要理解不同 API 的细节。

替代方案：前端直接 fetch CLIProxyAPI。该方案实现更快，但会让凭证暴露给前端环境，也会把标准化与错误处理分散到 UI 层。

### 首版 SQLite 只承担最新状态缓存

数据库保存 provider_connections、provider_accounts、quota_snapshots、quota_windows 和 settings。MVP 不做长期历史趋势，只保留每个账号的最新快照和必要同步状态。

选择原因：余量球需要离线或同步失败时仍展示上次成功数据，但长期历史会显著扩大首版范围。

替代方案：完全不落库，只用内存状态。该方案无法在重启后恢复配置和缓存，不满足桌面常驻工具的基本预期。

### 统一额度模型先于 Provider 适配

CLIProxyAPI 返回内容必须先映射为 QuotaAccount、QuotaWindow 和 QuotaSummary，再推送给 UI。当前关键窗口由 Rust 侧统一计算。

选择原因：避免 UI 绑定 CLIProxyAPI 的原始字段，也为后续火山 Provider 复用同一展示和汇总逻辑。

替代方案：为 CLIProxyAPI 做专用 UI 数据结构。该方案首版代码更短，但会阻碍后续多 Provider 扩展。

### 凭证首版本地保存但必须脱敏

CLIProxyAPI base_url 和 management_key 可以先保存到本地 SQLite 或配置文件。日志、错误提示、调试输出和 UI 列表不得展示完整 key。

选择原因：本项目定位为本地工具，系统凭证库可以后置；但日志泄露风险很容易发生，必须第一版控制。

替代方案：立即接入 Credential Manager 和 Keychain。该方案更安全，但会增加跨平台实现和调试复杂度。

### 同步失败使用缓存降级

任何网络、鉴权、超时或解析失败都不得把未知额度显示为 0%。系统必须保留上次成功快照，标记数据时间和连接异常状态。

选择原因：0% 表示额度耗尽，未知表示无法确认，两者对用户决策完全不同。

替代方案：失败时清空数据或显示 0%。该方案简单但会产生误导。

## Risks / Trade-offs

- [Risk] CLIProxyAPI 版本或部署差异导致字段不稳定。 -> Mitigation: Provider 层隔离 mapper 和 error，未知字段降级为 Unknown，不影响 UI 主流程。
- [Risk] 首版未接系统凭证库存储，配置文件泄露会暴露 key。 -> Mitigation: 明确仅本机保存，日志脱敏，后续单独增加安全存储 change。
- [Risk] 透明置顶窗口在 Windows/macOS 多显示器和 DPI 场景表现不一致。 -> Mitigation: MVP 先实现基础位置保存和主屏幕兜底，多显示器精细吸附后置。
- [Risk] 额度窗口单位和周期不一致导致汇总误判。 -> Mitigation: 当前可用额度只基于 active 且有 remaining_percent 的窗口选择，无法计算时展示未知而不是推断。
- [Risk] 后台定时同步与手动刷新并发请求同一连接。 -> Mitigation: 使用连接级互斥锁，同一连接同时只允许一个同步任务运行。

## Migration Plan

这是新项目初始化，无需兼容既有应用数据。首次启动时创建数据库和默认设置；后续 schema 变更通过 SQLx migration 管理。

如果 MVP 实现失败或需要回滚，可删除本地应用数据目录中的数据库与设置文件，应用重新创建默认状态。

## Open Questions

- CLIProxyAPI 的额度数据首版优先使用哪个稳定接口或策略，需要在实现时根据实际实例响应确认。
- 简化管理窗口首版是否只提供连接配置与状态列表，还是同时展示基础账号表格。
- 默认同步频率使用 3 分钟是否足够，还是需要提供设置项允许用户调整。
