# Provider 配置

TokenBall 当前支持 CLIProxyAPI、火山引擎和千问 Token Plan。所有凭证都只应在可信本机环境配置，不要提交本地数据库、日志或配置文件。

## CLIProxyAPI

适用于已经使用 CLIProxyAPI 管理多个 Codex 账号的场景。

| 字段 | 说明 |
| --- | --- |
| 显示名称 | 当前实例在 TokenBall 中显示的名称 |
| 服务地址 | CLIProxyAPI Management API 地址，例如 `http://127.0.0.1:8317` |
| 管理 Key | CLIProxyAPI Management Key |

保存后点击测试。测试成功后，TokenBall 会读取实例账号、额度窗口、账号状态和请求统计。

## 火山引擎

火山引擎支持两种渠道。

### 官方渠道

通过火山 OpenAPI 查询套餐和用量。

| 字段 | 说明 |
| --- | --- |
| OpenAPI Host | 默认 `https://open.volcengineapi.com` |
| Access Key ID | 火山引擎 AK |
| Secret Access Key | 火山引擎 SK |
| Region | 默认 `cn-beijing` |
| Service | 默认 `ark` |
| Coding ProjectName | Coding Plan 查询参数，可先保留 `default` |
| Coding SeatIDs | 高级席位查询参数，多个用逗号分隔 |

### 页面渠道

通过控制台登录态 Cookie 查询页面接口，适合官方 OpenAPI 暂时无法覆盖的场景。

| 字段 | 说明 |
| --- | --- |
| 控制台 API Host | 默认 `https://console.volcengine.com/api/top` |
| Coding ProjectName | 默认 `default` |
| 控制台 Cookie | 从 `console.volcengine.com` 登录态请求复制 |

## 千问 Token Plan

千问当前使用控制台登录态 Cookie 查询个人版 Token Plan 用量。

| 字段 | 说明 |
| --- | --- |
| ProductCode | 默认 `token-plan` |
| 控制台网关 | 默认 `https://platform-home.qianwenai.com` |
| 控制台 Cookie | 从 `platform.qianwenai.com` 登录态请求复制 |

千问接口细节见 [`/qianwen-token-plan-api`](/qianwen-token-plan-api)。

## 常见问题

### 测试连接成功但没有额度

先点击刷新，确认上游账号本身有用量数据。千问场景下还需要确认 Cookie 可以获取 `secToken`，并且个人版接口返回了 5 小时或每周字段。

### Cookie 保存后不回显

这是预期行为。界面只展示是否已保存，避免把敏感内容直接显示在输入框里。

### 额度显示为未知

通常表示上游没有返回可计算的 `remainingPercent`，或者当前窗口只有已用量没有总量。TokenBall 会尽量展示已用量作为补充信息。
