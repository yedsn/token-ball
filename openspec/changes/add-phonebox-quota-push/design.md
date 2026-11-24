# Design

## Context

动机与范围见 `proposal.md`。以下是已核对的实现约束，不代表功能已经存在。

| 来源 | 现状与约束 |
| --- | --- |
| `src-tauri/src/commands/quota.rs` | 全量和单实例刷新分别加载汇总并发布桌面事件；平台失败时仍可读取旧快照 |
| `src-tauri/src/storage/repository.rs` | 有独立 settings 读写入口；汇总包含未按启用实例过滤的账号，且摘要平台/状态不适合作为多实例统一状态 |
| `src-ui/src/store.ts` | 总体余量是启用实例账号关键窗口百分比的平均值；等效账号数是已知百分比除以 100 后求和 |
| `src-ui/src/utils/quotaPeriod.ts` | 已有 5 小时、周、月的周期识别及恢复时间规则，不能借用其他周期的时间 |
| PhoneBox `Program.cs` / `PushStore.cs` | 同机 `POST /api/push/{key}` 无需密钥；JSON 原样保存，单次请求最大 64 KiB，可设置 TTL |
| PhoneBox `widgets/pc.js` | 自定义 HTML 通过 `phonebox.on('push', ...)` 收到按 key 索引的 `{ data, at, expiresAt }`；运行于沙箱 |
| PhoneBox `docs/copilot-cli-page.json` | 页面导入使用 `{ phonebox: 'pages', version: 1, pages: [...] }`，12×6 网格 |

## Goals / Non-Goals

**Goals:** 后端常驻投递与桌面窗口可见性解耦；提供有版本的最小展示协议；重启可恢复、错误可诊断；模板可从源码重复生成。

**Non-Goals:** 不提供通用 Webhook 插件平台，不修改 Provider 采集逻辑，不读写 PhoneBox 的设置，不加入跨机器 HTTPS/证书/密钥配置，不通过手机执行桌面动作。

## Decisions

### 1. 使用已有推送接口，不开服务端

TokenBall 使用已有 HTTP 客户端依赖发送 `POST <baseUrl>/api/push/tokenball`。固定 key 为 `tokenball`，第一版只支持一个 TokenBall 向一个同机 PhoneBox 投递。PhoneBox 页面使用该 key，不需要任何上游源码补丁。

替代方案：TokenBall 对外 API + PhoneBox Agent 拉取需要双仓开发与路由兼容；数据库桥接需要额外进程且耦合存储结构，均不作为本次实现。

### 2. 集成设置独立保存，缺失即关闭

计划在 `settings` 中新增 `integration.phonebox`，与额度显示设置隔离，避免影响旧配置加载。管理窗口“通用”设置增加集成区。

| 项目 | 默认/约束 |
| --- | --- |
| `enabled` | `false`；开启后后台生效，关闭取消未发任务/在途请求 |
| `baseUrl` | `http://127.0.0.1:8765`；仅 HTTP，主机为字面量 `127.0.0.1` 或 `[::1]`；允许合法自定义端口 |
| URL 限制 | 拒绝远程地址、localhost 等域名、userinfo、路径（根路径除外）、query、fragment |
| 推送周期 | 后台心跳固定 60 秒；不增加可调参数 |
| 请求超时 | 3 秒；禁用环境代理与自动重定向 |
| TTL | 请求 `?ttl=180`，心跳停止后旧数据会从 PhoneBox 消失 |
| 最近结果 | 运行时保存状态、最后尝试/成功时间、简短错误码；重启显示尚未投递 |

“立即推送”只在已保存且启用时可用，发送当前额度缓存，不调用平台刷新。保存地址变化时取消旧目标任务，在新目标立即投递；旧目标数据按 TTL 自然过期。

限制字面量回环地址是为了避免 DNS、代理和重定向把展示数据发送到其他机器。暂不支持 `localhost` 是明确的输入限制，不要求用户处理鉴权或证书。

### 3. 单后台发送者，事件触发与心跳互补

计划新增 `src-tauri/src/integrations/phonebox/`，统一负责设置校验、快照映射与发送。启动后创建一个发送者；事件使用合并信号唤醒，连续变化不积累历史快照。

```text
启动 / 开启集成 / 立即推送 / 实例变更 / 额度刷新完成
                         ↓ 合并信号
                  后台发送者（单请求）
                         ↑
                     60 秒心跳
                         ↓
          读取最新缓存 → 构造白名单快照 → PhoneBox
```

- 全量刷新、单实例刷新不论采集成功还是失败，都在状态落库后通知发送者；只发信号，不等待网络请求。
- 实例保存、启停、删除、导入完成后重新计算推送范围。桌面悬浮球选中账号不影响该范围。
- 启动及开启集成立即投递缓存；此后心跳解决 PhoneBox 晚启动、重启、投递失败的恢复问题，不触发 Provider 请求。
- 同时最多一个请求；在途期间的新信号合并为下一次最新快照投递。瞬时失败不紧密重试，下次事件或心跳再试。
- 错误不传播到额度刷新结果、账号状态和平台连接状态；失败日志只记分类、状态码，不记录响应体、凭据或完整 payload。成功须为 2xx，且限长读取的响应 JSON 含 `ok=true`、`key='tokenball'`；其他响应视为地址不匹配或协议失败。
- 禁用时不调用清空接口或删除其他 key；停止心跳，让本次集成的数据在最多 180 秒后自然过期。

替代方案：只在刷新后发送无法及时恢复 PhoneBox 重启（TokenBall 默认按小时采集）；直接从 Vue 推送依赖窗口生命周期，两者均不采用。

### 4. 白名单快照兼容两种 PhoneBox 组件

以下字段为计划的 v1 展示协议；不是直接序列化现有 `QuotaSummary`、连接对象或数据库行。

| 层级 | 字段与含义 |
| --- | --- |
| 兼容字段 | `title='TokenBall'`、`value`（平均关键窗口余量或 `--`）、`unit='%'`、`progress`（已知时 0–1）、`sub`、`level` |
| 协议 | `schema='tokenball.phonebox.v1'`、`pushedAt`（本次发送 UTC 时间）、`heartbeatTimeoutSeconds=180` |
| 汇总 | `summary.accountCount`、`availableAccountCount`、`knownQuotaAccountCount`、`averageRemainingPercent`、`equivalentAccounts`、`stale` |
| 账号 | `accounts[]` 含 `id`（不透明 ID）、`alias`（备用脱敏别名）、`displayName`（账号展示名称，移除控制字符、最多 160 字符）、`provider`、`instanceAlias`、`planName`（限长纯文本）、`status`、`connectionStatus`、实际 `collectedAt`、`staleAt`、`stale`、`subscriptionUntil` |
| 排序 | `accounts[].priorityResetAt` 复用托盘的优先恢复时间：月周期优先，否则选择带恢复时间的最高优先级窗口，最终回退账号 nextResetAt；仅用于排序 |
| 周期 | `accounts[].periods` 的 `fiveHour` / `weekly` / `monthly`，各项为 null 或含 `remainingPercent`、`total`、`used`、`remaining`、`unit`、`resetAt` |
| 关键窗口 | `accounts[].criticalRemainingPercent` 独立取原关键窗口，不强制属于三个展示周期 |
| 体积提示 | `includedAccountCount`、`omittedAccountCount`，截断不改变全量汇总 |

映射与口径：

- 仅包含启用实例的账号，账号状态保留原值；可用账号只计 available/warning。
- 先用每个账号原 `criticalWindowId` 取已知百分比，再计算平均值与等效量，不把最低余量混称为总体余量。未知值不进入平均分母，等效量是已知值之和除以 100；全部未知时平均值与等效量均为 null。显示“已知额度等效账号”，避免把未知解释成已耗尽；这一 null 约定仅用于新集成协议，不改桌面现有全未知时求和为 0 的行为。
- Provider 从所属连接映射，不使用全局摘要的单一 provider/status；连接状态按账号附带。
- 周期识别与已有工具规则一致，包含火山 5 小时 AFP；恢复时间只能来自当前周期窗口，订阅到期单独展示。为 Rust 映射建立与已有前端样例一致的回归数据，不改写现有桌面业务。
- 新输出 ID 用源 ID 的单向摘要，保留账号/实例通用别名。2026-11-24 按用户要求增加账号 displayName 展示；名称本身可能含邮箱，设置页和接入文档明确提示。除此展示字段外，不输出独立邮箱字段、原实例名称、externalId、ChatGPT ID、连接地址、原始响应或任何凭据；日志不包含展示负载。
- 不输出 recentRequests 等非展示必需历史，所有字符串限长，数值与时间校验，非有限数/无效时间映射为 null。
- JSON 使用 UTF-8 紧凑序列化，最多 60 KiB。后端先按订阅时间、优先恢复时间排序，缺失时间最后，名称和 ID 确保确定性；超量从末尾移除完整账号并报告遗漏数，优先保留紧急账号。页面对已包含账号按订阅时间、优先恢复时间、名称 zh-CN localeCompare 排序，与托盘一致；后端容量截断时的同时间名称并列使用 Rust 字符串排序，不引入 ICU 依赖。汇总始终使用全部启用账号计算，禁止静默截断或发送半个账号。

### 5. 采集新鲜度与传输在线状态分离

每个账号保留实际采集时间。`staleAt = collectedAt + max(300, syncIntervalSeconds + enabledRandomDelayMaxSeconds + 300)`；缺失采集时间或当前连接不是 healthy 时立即按缓存/异常处理。映射层直接检查存储中的时间是否缺失，不能沿用读取模型将缺失值兜底为当前时间的行为。

`summary.stale` 为存在过期/异常账号；模板还必须按自己的当前时间重新判断 `staleAt`，因为两次推送之间状态会随时间变化。`pushedAt` 与 PhoneBox 包装的 `at` 只用于传输在线提示，不能替代 `collectedAt`；多账号分别显示实际更新时间，不用一个最新时间覆盖旧账号。

TTL 解决彻底退出/停止推送后的旧值残留。心跳中断、数据消失或本机时间明显不合理时模板显示未连接/状态未知，不自动把剩余值改成 0，也不在恢复时间到达时臆造 100%。

### 6. 模板作为 TokenBall 交付资源

计划文件：

- `integrations/phonebox/tokenball-widget.html`：独立 HTML/CSS/JS 源模板，订阅 `push`，读取 `items.tokenball.data`。
- `integrations/phonebox/tokenball-page.json`：嵌入源模板的可导入页面，遵循已核对的导入 envelope；单个自定义 HTML 占 12×6，不重复添加普通 push 总览。
- 对应生成/一致性检查脚本：只负责源模板嵌入 JSON，不引入打包框架。
- `docs/guide/phonebox.md`：同机前置条件、保存启用、立即推送、导入页面、验证数据、停用及排障；模板随构建作为独立资源供下载。

横屏左侧为全高总览，右侧通过按钮、圆点、滑动或键盘切换账号；窄屏上下排列，必要时当前账号内容可滚动。账号装饰与文字分区，避免叠字。收到推送或排序变化时按不透明 ID 保留当前账号，不自动轮播。使用本地样式、无外部字体/脚本/图片，不联网、不执行快捷指令。账号和套餐等动态文字使用文本节点，不拼接为 HTML。定时器本地更新倒计时与过期标记，缺失协议、未知版本或数据被 TTL 移除时给出明确提示。

## Risks / Trade-offs

- [PhoneBox 不同版本行为不一致] → 以当前仓库协议为兼容基线，文档记录实际联调版本/提交，导入与沙箱展示都做验证，不宣称全版本支持。
- [账号过多超过单条容量] → 60 KiB 安全预算、全量汇总、遗漏数量提示；第一版不做多 key 分片协议。
- [账号展示名称可能含邮箱] → 按用户要求显示账号名称，并在设置/文档明确提示；不透明 ID、备用别名与实例脱敏仍保留，不发送凭据及其他原始标识，不在日志输出负载。
- [PhoneBox 通用组件只知道投递时间] → 通用组件副标题注明缓存/采集口径，详细模板逐账号显示真实时间；TTL 限制过期残留。
- [新鲜度规则与随机采集间隔不同步] → 每次映射读取当前采集设置，并覆盖随机延迟启停的测试。
- [关闭集成后不会立刻抹掉 PhoneBox 数据] → UI/文档说明最多 180 秒过期；用户也可在 PhoneBox Webhooks 手动删除 `tokenball`。

## Migration Plan

1. 增量增加集成设置，旧数据库无此项时保持关闭；不改动已有显示设置及凭据备份格式。
2. 升级 TokenBall 后，用户在同机启动 PhoneBox Agent，保存启用并立即推送；在 Webhooks 检查 `tokenball`。
3. 用户通过 Pages 导入随附 JSON 并保存，在预览及手机检查；不自动覆盖原页面。
4. 回滚时关闭集成或退回旧 TokenBall 版本，原 Provider 配置仍有效；PhoneBox 数据 TTL 过期后消失，模板可由用户删除。
