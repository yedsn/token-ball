# Spec Delta

## Purpose

让 TokenBall 在用户明确启用后，把已有的多平台额度快照安全推送到同机 PhoneBox，提供统一、可解释的展示数据及可恢复的投递行为，同时确保外部面板故障不会影响额度采集、账号状态或桌面使用体验。

## ADDED Requirements

### Requirement: Explicit opt-in and local destination

系统 SHALL 默认关闭 PhoneBox 集成，并独立持久化启用状态和 Agent 地址；默认地址为 `http://127.0.0.1:8765`。第一版 MUST 仅接受主机为字面量 `127.0.0.1` 或 `[::1]` 的 HTTP 根地址及合法端口，拒绝域名、远程地址、用户信息、非根路径、query 与 fragment，不使用环境代理或跟随重定向。

#### Scenario: Upgrade without integration settings
- **WHEN** 用户升级或首次启动，尚未保存 PhoneBox 设置
- **THEN** 集成保持关闭，不发送请求，已有额度与显示设置不受影响

#### Scenario: Invalid or redirected destination
- **WHEN** 用户保存远程/域名地址，或 Agent 响应重定向
- **THEN** 系统拒绝无效配置或报告投递失败，不向重定向目标发送展示数据

### Requirement: Versioned safe quota payload

系统 SHALL 使用 `POST /api/push/tokenball?ttl=180` 发送 `schema=tokenball.phonebox.v1` 的 JSON，兼容 PhoneBox 普通推送组件的 value/title/unit/sub/progress/level 字段，并提供账号详情。系统 MUST 采用允许列表，只输出额度展示必需字段、脱敏通用别名、不透明 ID、平台/套餐/状态、独立周期值及时间；MUST NOT 输出原账号或实例名称、平台凭据、原始账号标识、连接地址、原始响应和请求历史。

#### Scenario: Credentials and identifiers exist in source data
- **WHEN** 本地连接保存 Cookie、AK/SK、Management Key，账号含邮箱、externalId 或 ChatGPT ID
- **THEN** 出站 JSON 及投递日志均不包含这些信息，也不通过原显示名称间接泄露这些标识

#### Scenario: Standard and custom widgets consume one key
- **WHEN** PhoneBox 接收到 v1 负载
- **THEN** 普通 push 组件可显示总体余量，自定义 HTML 可从同一个 key 读取详细账号数据

### Requirement: Enabled account scope and consistent quota arithmetic

系统 SHALL 仅推送启用实例所属账号，独立于悬浮球选中账号；以账号关键窗口的已知百分比计算平均余量及已知额度等效账号数，并输出已知额度账号数。全部未知时平均余量与等效账号数 MUST 为 null。各账号平台及连接状态 SHALL 来自各自连接，不能套用全局单一 provider/status。

#### Scenario: Mixed providers and unknown quota
- **WHEN** 启用实例中两个账号的关键窗口余量分别为 80%、20%，第三个未知，另有停用实例账号
- **THEN** 账号数为 3、已知额度账号数为 2、平均余量为 50%、已知额度等效账号数为 1.0，停用账号不参与，平台分别映射

#### Scenario: No known quotas
- **WHEN** 所有启用账号关键窗口缺失或没有可用百分比
- **THEN** 平均余量与等效账号数为 null，不输出代表耗尽的 0%

### Requirement: Independent period values and collection freshness

系统 SHALL 为 5 小时、周、月分别输出自身窗口的额度及 resetAt，缺失/无效值为 null；订阅到期时间独立表达。每个账号 MUST 保留真实采集时间并给出独立 staleAt，不能以推送时间或其他账号的时间替代。staleAt 使用采集时间加 `max(300, 同步间隔 + 已启用的最大随机延迟 + 300)` 秒；采集时间缺失、连接异常或超过期限时 MUST 标为缓存/过期。

#### Scenario: Weekly reset missing but subscription exists
- **WHEN** 周额度没有 resetAt，但 5 小时窗口及订阅到期时间存在
- **THEN** 周恢复时间仍为 null，不借用其他时间

#### Scenario: Heartbeat republishes old cache
- **WHEN** 心跳重新投递旧缓存，某账号已超过 staleAt 或连接异常
- **THEN** pushedAt 可以更新，collectedAt 不变，缓存/过期状态保留，各账号独立判断

#### Scenario: Stored collection timestamp missing
- **WHEN** 数据库存有账号但缺少有效快照采集时间
- **THEN** 出站 collectedAt 为 null 并标记缓存/未知，不兜底为当前时间

### Requirement: Recoverable background delivery without refresh coupling

启用时系统 SHALL 在启动、启用/地址变更、额度全量或单实例刷新完成（含失败）、实例范围变化后投递当前缓存，并每 60 秒心跳重发。立即推送 SHALL 只投递已保存且启用的配置对应缓存，不触发 Provider 请求。投递 MUST 与窗口显隐无关、最多一个并发请求、只保留待发的最新状态；失败只在下次事件或心跳重试，不积累历史任务。

#### Scenario: PhoneBox starts after TokenBall
- **WHEN** 投递因 Agent 未启动而失败，随后 Agent 启动且没有新的平台采集
- **THEN** 下次心跳自动投递当前缓存，无需打开 TokenBall 窗口或手动刷新平台

#### Scenario: Burst updates during slow delivery
- **WHEN** 一个请求正在执行，发生多次实例及额度变化
- **THEN** 不并发发送旧快照，后续投递使用最新范围和最新数据

#### Scenario: Integration disabled
- **WHEN** 用户关闭集成
- **THEN** 系统取消在途及待发任务，不再投递，不清除 PhoneBox 其他数据；已接收的 tokenball 值最多 180 秒后按 TTL 过期

### Requirement: Isolated failure reporting and bounded payload

单次投递 SHALL 在 3 秒内超时，并在管理页提供最近尝试/成功时间及简短错误原因；只有预期成功响应才算投递成功。失败 MUST NOT 改变额度刷新结果、Provider 状态或阻塞桌面界面。发送体 MUST 不超过 60 KiB，超量时按稳定顺序只保留完整账号，报告遗漏数量且保留全量汇总，不发送不完整 JSON 或记录敏感响应体。

#### Scenario: HTTP failure or invalid success response
- **WHEN** 请求连接失败、超时、收到非 2xx，或 2xx 不符合 PhoneBox 成功确认
- **THEN** 管理页显示投递失败及简短原因，额度刷新保持原结果，后续可自动恢复

#### Scenario: Account detail exceeds request budget
- **WHEN** 完整账号详情序列化后超过 60 KiB
- **THEN** 发送体保持合法且不超过限制，汇总仍覆盖所有启用账号，includedAccountCount 与 omittedAccountCount 准确
