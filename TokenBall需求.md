下面是一份可以直接作为项目立项、技术设计和开发排期基础的完整方案。

# AI 流量余量球完整产品与技术实施方案

## 一、项目概述

### 1.1 项目名称

暂定名称：**AI 流量余量球**

可选英文名称：

* Quota Orb
* Token Orb
* AI Quota Ball
* Usage Orb

### 1.2 产品定位

AI 流量余量球是一款运行在 Windows 和 macOS 桌面上的轻量级悬浮应用，用于实时展示多个 AI 编程服务渠道的当前可用额度、账号状态和额度恢复时间。

V1.0 首期接入：

1. **CLIProxyAPI**
2. **火山引擎方舟**

其中：

* CLIProxyAPI 用于展示其管理的多个 Codex Plus 账号额度。
* 火山引擎用于展示 Coding Plan 和 Agent Plan 的套餐用量。

### 1.3 核心产品价值

用户无需反复进入不同平台的控制台，即可通过桌面余量球快速回答：

* 当前还有多少额度可用？
* 哪个渠道最紧张？
* 哪个账号已经进入冷却？
* 下一个额度窗口什么时候恢复？
* CLIProxyAPI 中还有多少账号可以使用？
* 火山 Coding Plan 和 Agent Plan 各自剩余多少？

---

# 二、项目范围

## 2.1 V1.0 支持范围

### CLIProxyAPI

支持：

* 配置一个或多个 CLIProxyAPI 实例
* 读取实例内的 Codex Plus 账号
* 展示各账号当前额度
* 展示账号可用、冷却、耗尽、失效和禁用状态
* 展示多个额度窗口
* 展示额度重置时间
* 获取逐请求 Token 使用记录
* 展示实例连接状态

### 火山引擎

支持：

* Coding Plan
* Agent Plan
* 查询个人套餐
* 查询套餐使用情况
* 查询 Agent Plan AFP 使用情况
* 展示多时间窗口
* 展示额度重置时间
* 展示套餐状态和有效期

## 2.2 V1.0 不支持

暂不支持：

* 千问 Token Plan
* Claude Pro 或 Claude Max 直接接入
* OpenAI 官方账单 API
* 自动购买套餐
* 自动续费
* 自动修改 CLIProxyAPI 路由
* 云端账号体系
* 多设备同步
* 团队协作
* 移动端
* Web 管理后台

---

# 三、技术栈总览

## 3.1 推荐技术组合

```text
桌面框架：Tauri 2
前端框架：Vue 3
前端语言：TypeScript
构建工具：Vite
状态管理：Pinia
样式方案：CSS Variables + UnoCSS
后端语言：Rust
异步运行时：Tokio
HTTP 客户端：Reqwest
序列化：Serde
数据库：SQLite
数据库访问：SQLx
日志：Tracing
错误处理：Thiserror
时间处理：Chrono
```

Tauri 2 支持 Windows 和 macOS，并提供透明窗口、窗口置顶、系统托盘、自启动、通知、窗口状态恢复和自动更新等能力，适合长期运行的桌面悬浮工具。([Tauri][1])

Vue 官方推荐 Vue 3 配合 TypeScript 和 Vite，适合作为该项目的界面技术栈。([Vue.js][2])

SQLx 支持 SQLite 和 Tokio 异步运行时，适合 Rust 后端存储额度快照、同步记录和调用历史。([Docs.rs][3])

---

# 四、系统总体架构

```text
┌──────────────────────────────────────────┐
│                Vue 前端                  │
│                                          │
│  余量球    悬停面板    管理窗口    设置   │
│                                          │
│  Pinia：界面状态、最新额度、用户偏好      │
└────────────────────┬─────────────────────┘
                     │
          Tauri Commands / Events
                     │
┌────────────────────▼─────────────────────┐
│                 Rust Core                │
│                                          │
│  Provider Adapter                        │
│  ├── CLIProxyAPI Adapter                 │
│  └── Volcengine Adapter                  │
│                                          │
│  Quota Aggregator                        │
│  Sync Scheduler                          │
│  Alert Engine                            │
│  Credential Service                      │
│  SQLite Repository                       │
│  Application Event Bus                   │
└────────────────────┬─────────────────────┘
                     │
          ┌──────────┴──────────┐
          │                     │
┌─────────▼─────────┐ ┌─────────▼──────────┐
│   CLIProxyAPI     │ │   火山引擎方舟 API │
└───────────────────┘ └────────────────────┘
```

## 4.1 架构原则

### Vue 前端负责

* 余量球展示
* 动画
* 用户交互
* 设置表单
* 数据列表
* 错误提示
* 接收 Rust 推送事件

### Rust 后端负责

* HTTP 请求
* 凭证管理
* 定时同步
* 数据缓存
* SQLite 存储
* 额度标准化
* 当前关键窗口计算
* 告警判断
* 网络重试
* 数据清理
* 日志记录

### 明确禁止

前端不直接：

* 保存 CLIProxyAPI Management Key
* 保存火山 AK/SK
* 请求火山接口
* 请求 CLIProxyAPI 管理接口
* 直接操作 SQLite
* 计算最终业务额度状态

---

# 五、桌面窗口设计

建议拆成三个独立窗口。

## 5.1 余量球窗口

### 用途

常驻桌面，展示最重要的额度信息。

### 推荐尺寸

默认：

```text
84 × 84 像素
```

支持用户选择：

* 64 × 64
* 72 × 72
* 84 × 84
* 96 × 96
* 120 × 120

### 窗口属性

```json
{
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "resizable": false,
  "shadow": false
}
```

Tauri 支持透明窗口、自定义窗口外观和 `alwaysOnTop` 等配置。([Tauri][1])

### 功能

* 拖动
* 屏幕边缘吸附
* 始终置顶
* 多显示器适配
* 显示或隐藏
* 自动轮播
* 鼠标悬停
* 单击打开详情
* 右键菜单
* 记住上次位置

---

## 5.2 悬停详情窗口

悬停时在余量球旁边显示。

### 展示内容

```text
AI 额度状态

CLIProxyAPI
账号 01    剩余 72%    可用
账号 02    剩余 46%    可用
账号 03    剩余 18%    冷却中

火山 Coding Plan
当前可用 42%
1 小时 26 分后重置

火山 Agent Plan
当前关键窗口剩余 61%

最后同步：刚刚
```

### 设计原则

* 独立窗口
* 自动根据屏幕边缘调整展开方向
* 鼠标离开后延迟关闭
* 鼠标移动到详情面板时不关闭
* 悬停期间暂停余量球自动轮播
* 面板最大高度限制
* 超出内容支持滚动

---

## 5.3 管理窗口

普通桌面窗口，用于完整管理。

### 页面

* 总览
* CLIProxyAPI
* 火山引擎
* 账号列表
* 历史趋势
* 告警记录
* 同步日志
* 设置
* 关于和更新

---

# 六、余量球展示方案

## 6.1 默认轮播

### 第一屏：总体当前可用额度

```text
68%
当前可用
```

### 第二屏：CLIProxyAPI 可用账号

```text
2 / 3
Codex 可用
```

### 第三屏：火山 Coding Plan

```text
Coding
剩余 42%
```

### 第四屏：火山 Agent Plan

```text
Agent
剩余 61%
```

### 第五屏：最近恢复时间

```text
1小时后
额度恢复
```

## 6.2 轮播规则

* 每屏停留 4 秒
* 切换动画 300 毫秒
* 鼠标悬停暂停
* 用户可以滚轮手动切换
* 异常状态优先展示
* 所有渠道正常时按用户配置顺序展示
* 用户可关闭自动轮播

## 6.3 球体表现

可使用：

* SVG 圆环
* SVG 遮罩
* CSS 动画
* CSS Transform
* 数字滚动动画
* 呼吸动画

不建议 V1.0 使用：

* Three.js
* WebGL
* Canvas 粒子系统
* 复杂液体物理模拟

### 推荐视觉方案

球体内部使用波浪水位表达剩余比例：

```text
剩余 70%：水位较高
剩余 30%：水位较低
剩余 10%：水位接近底部
```

同时显示明确数字，不能只依赖颜色和水位。

---

# 七、额度业务模型

## 7.1 核心概念

### Provider

数据来源，例如：

* CLIProxyAPI
* Volcengine

### Connection

用户配置的具体连接。

例如：

* 本机 CLIProxyAPI
* 远程 CLIProxyAPI
* 火山引擎主账号

### Account

具体使用额度的账号或席位。

### Plan

具体套餐：

* Codex Plus
* Coding Plan
* Agent Plan

### Quota Window

额度统计窗口：

* 5 小时
* 每日
* 每周
* 每月
* 滚动窗口
* 自定义周期

---

## 7.2 当前可用额度定义

当前可用额度是：

> 当前时刻真正限制继续使用服务的剩余额度。

例如：

```text
5 小时窗口：剩余 18%
每周窗口：剩余 63%
月度窗口：剩余 81%
```

当前可用额度应显示：

```text
18%
```

因为 5 小时窗口最先构成限制。

## 7.3 当前关键窗口

算法：

1. 获取所有有效额度窗口。
2. 排除无法计算的窗口。
3. 优先使用上游明确标记的限制窗口。
4. 如果上游没有标记，选剩余百分比最低的窗口。
5. 如果比例相同，优先选择重置时间更晚的窗口。
6. 将该窗口标记为当前关键窗口。

```rust
pub fn select_critical_window(
    windows: &[QuotaWindow],
) -> Option<&QuotaWindow> {
    windows
        .iter()
        .filter(|window| window.is_active)
        .filter(|window| window.remaining_percent.is_some())
        .min_by(|a, b| {
            let percent_order = a
                .remaining_percent
                .unwrap_or(100.0)
                .partial_cmp(
                    &b.remaining_percent.unwrap_or(100.0)
                )
                .unwrap_or(std::cmp::Ordering::Equal);

            if percent_order == std::cmp::Ordering::Equal {
                b.reset_at.cmp(&a.reset_at)
            } else {
                percent_order
            }
        })
}
```

## 7.4 多账号汇总

对于 CLIProxyAPI：

优先展示：

```text
可用账号：2 / 3
最低余量：18%
最近恢复：2 小时后
```

不建议直接平均各账号剩余比例，因为不同账号的统计窗口和绝对额度可能不同。

---

# 八、统一数据模型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderType {
    CliProxyApi,
    Volcengine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountStatus {
    Available,
    Warning,
    Cooldown,
    Exhausted,
    Disabled,
    AuthExpired,
    Offline,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuotaUnit {
    Percent,
    Token,
    Credit,
    Afp,
    Request,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeriodType {
    Rolling,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConnection {
    pub id: String,
    pub provider_type: ProviderType,
    pub display_name: String,
    pub enabled: bool,
    pub status: ConnectionStatus,
    pub last_synced_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaAccount {
    pub id: String,
    pub connection_id: String,
    pub external_id: String,
    pub display_name: String,
    pub masked_identifier: Option<String>,
    pub plan_name: String,
    pub status: AccountStatus,
    pub windows: Vec<QuotaWindow>,
    pub critical_window_id: Option<String>,
    pub next_reset_at: Option<DateTime<Utc>>,
    pub synced_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaWindow {
    pub id: String,
    pub name: String,
    pub period_type: PeriodType,
    pub period_seconds: Option<i64>,
    pub total: Option<f64>,
    pub used: Option<f64>,
    pub remaining: Option<f64>,
    pub remaining_percent: Option<f64>,
    pub unit: QuotaUnit,
    pub reset_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub is_current_constraint: bool,
    pub data_source: DataSource,
}
```

---

# 九、Provider Adapter 设计

所有渠道实现统一接口。

```rust
#[async_trait::async_trait]
pub trait QuotaProvider: Send + Sync {
    fn provider_type(&self) -> ProviderType;

    async fn test_connection(
        &self,
    ) -> Result<ConnectionTestResult, ProviderError>;

    async fn list_accounts(
        &self,
    ) -> Result<Vec<ProviderAccount>, ProviderError>;

    async fn fetch_quota(
        &self,
        account: &ProviderAccount,
    ) -> Result<QuotaSnapshot, ProviderError>;

    async fn fetch_all(
        &self,
    ) -> Result<Vec<QuotaSnapshot>, ProviderError>;
}
```

## 9.1 CLIProxyAPI Adapter

职责：

* 连接 Management API
* 获取认证账号
* 读取账号状态
* 获取额度数据
* 获取使用记录
* 解析多个额度窗口
* 处理不同 CLIProxyAPI 版本差异

模块：

```text
providers/cliproxy/
├── mod.rs
├── client.rs
├── auth_files.rs
├── quota.rs
├── usage_queue.rs
├── version.rs
├── mapper.rs
└── error.rs
```

### 主要接口

```text
GET /v0/management/auth-files
GET /v0/management/usage-queue
GET /v0/management/usage-statistics-enabled
POST /v0/management/api-call
```

### 版本适配

```rust
pub trait CliProxyQuotaStrategy {
    async fn supports(
        &self,
        version: &str,
    ) -> bool;

    async fn fetch_quota(
        &self,
        client: &CliProxyClient,
        account: &CliProxyAccount,
    ) -> Result<QuotaSnapshot, ProviderError>;
}
```

可以实现：

```text
DirectQuotaStrategy
ApiCallQuotaStrategy
LegacyQuotaStrategy
```

---

## 9.2 火山引擎 Adapter

模块：

```text
providers/volcengine/
├── mod.rs
├── client.rs
├── signer.rs
├── personal_plan.rs
├── usage_details.rs
├── afp_usage.rs
├── mapper.rs
└── error.rs
```

### 主要接口

```text
GetPersonalPlan
GetUsageDetails
GetAFPUsage
```

### 处理逻辑

1. 查询用户套餐。
2. 判断套餐类型。
3. Coding Plan 调用 `GetUsageDetails`。
4. Agent Plan 调用 `GetUsageDetails` 和 `GetAFPUsage`。
5. 标准化不同时间窗口。
6. 计算当前关键窗口。
7. 写入数据库。
8. 推送前端更新事件。

---

# 十、Rust 后端模块设计

```text
src-tauri/src/
├── main.rs
├── lib.rs
├── app_state.rs
├── error.rs
│
├── commands/
│   ├── mod.rs
│   ├── connection.rs
│   ├── quota.rs
│   ├── settings.rs
│   ├── history.rs
│   └── window.rs
│
├── providers/
│   ├── mod.rs
│   ├── traits.rs
│   ├── cliproxy/
│   └── volcengine/
│
├── quota/
│   ├── mod.rs
│   ├── model.rs
│   ├── normalize.rs
│   ├── aggregate.rs
│   └── critical_window.rs
│
├── scheduler/
│   ├── mod.rs
│   ├── quota_sync.rs
│   ├── usage_consumer.rs
│   └── cleanup.rs
│
├── storage/
│   ├── mod.rs
│   ├── database.rs
│   ├── migrations.rs
│   ├── connection_repository.rs
│   ├── quota_repository.rs
│   ├── usage_repository.rs
│   └── settings_repository.rs
│
├── security/
│   ├── mod.rs
│   └── credential_store.rs
│
├── alerts/
│   ├── mod.rs
│   ├── engine.rs
│   └── rules.rs
│
├── events/
│   ├── mod.rs
│   └── emitter.rs
│
└── windows/
    ├── mod.rs
    ├── orb.rs
    ├── hover.rs
    └── positioning.rs
```

---

# 十一、前端工程结构

```text
src/
├── main.ts
├── App.vue
│
├── views/
│   ├── OrbView.vue
│   ├── HoverPanelView.vue
│   ├── DashboardView.vue
│   ├── ConnectionsView.vue
│   ├── HistoryView.vue
│   └── SettingsView.vue
│
├── components/
│   ├── orb/
│   │   ├── QuotaOrb.vue
│   │   ├── LiquidLevel.vue
│   │   ├── OrbCarousel.vue
│   │   └── OrbStatusIcon.vue
│   │
│   ├── quota/
│   │   ├── QuotaAccountRow.vue
│   │   ├── QuotaWindowBar.vue
│   │   ├── ResetCountdown.vue
│   │   └── ProviderCard.vue
│   │
│   └── common/
│       ├── StatusBadge.vue
│       ├── EmptyState.vue
│       └── ErrorBanner.vue
│
├── stores/
│   ├── quota.ts
│   ├── connection.ts
│   ├── settings.ts
│   └── ui.ts
│
├── composables/
│   ├── useQuotaEvents.ts
│   ├── useCountdown.ts
│   ├── useOrbCarousel.ts
│   └── useHoverPanel.ts
│
├── services/
│   └── tauri.ts
│
├── types/
│   ├── quota.ts
│   ├── connection.ts
│   └── settings.ts
│
└── styles/
    ├── variables.css
    ├── global.css
    └── animations.css
```

---

# 十二、Tauri 通信设计

## 12.1 Commands

Vue 主动调用 Rust：

```text
connection_test
connection_create
connection_update
connection_delete

quota_get_latest
quota_refresh_all
quota_refresh_connection

settings_get
settings_update

history_query
sync_logs_query
```

## 12.2 Events

Rust 主动通知 Vue：

```text
quota://updated
quota://refresh-started
quota://refresh-completed

provider://connected
provider://error

alert://triggered

settings://updated
```

### 示例

```rust
app.emit(
    "quota://updated",
    &quota_summary,
)?;
```

Vue：

```ts
import { listen } from '@tauri-apps/api/event'

await listen<QuotaSummary>(
  'quota://updated',
  event => {
    quotaStore.updateSummary(event.payload)
  }
)
```

---

# 十三、后台同步方案

## 13.1 同步任务

Rust 启动后运行以下后台任务：

```text
Quota Sync Scheduler
Usage Queue Consumer
Alert Evaluator
Database Cleanup
Connection Health Checker
```

## 13.2 推荐频率

### CLIProxyAPI 额度

默认每 3 分钟。

### CLIProxyAPI 账号状态

默认每 1 分钟。

### CLIProxyAPI Usage Queue

建议每 10～30 秒消费一次。

### 火山套餐额度

默认每 5 分钟。

### 火山套餐信息

默认每 60 分钟。

### 数据清理

每天一次。

## 13.3 后台任务示例

```rust
pub async fn start_quota_scheduler(
    state: Arc<AppState>,
) {
    let mut interval =
        tokio::time::interval(Duration::from_secs(300));

    loop {
        interval.tick().await;

        if let Err(error) =
            sync_all_providers(state.clone()).await
        {
            tracing::warn!(
                ?error,
                "quota synchronization failed"
            );
        }
    }
}
```

## 13.4 防止重复同步

使用连接级锁：

```rust
HashMap<ConnectionId, Arc<Mutex<()>>>
```

同一个渠道同时只允许一个同步任务运行。

## 13.5 重试策略

```text
第一次失败：5 秒后
第二次失败：15 秒后
第三次失败：60 秒后
之后：恢复正常同步周期
```

禁止无限快速重试。

---

# 十四、本地数据库设计

## 14.1 数据库选择

使用：

```text
SQLite + SQLx
```

建议启用：

```text
WAL 模式
Foreign Keys
Busy Timeout
```

## 14.2 数据表

### provider_connections

```sql
CREATE TABLE provider_connections (
    id TEXT PRIMARY KEY,
    provider_type TEXT NOT NULL,
    display_name TEXT NOT NULL,
    base_url TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### provider_accounts

```sql
CREATE TABLE provider_accounts (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL,
    external_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    masked_identifier TEXT,
    plan_name TEXT,
    status TEXT NOT NULL,
    last_synced_at TEXT,
    FOREIGN KEY(connection_id)
        REFERENCES provider_connections(id)
);
```

### quota_snapshots

```sql
CREATE TABLE quota_snapshots (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    status TEXT NOT NULL,
    critical_window_id TEXT,
    next_reset_at TEXT,
    collected_at TEXT NOT NULL,
    FOREIGN KEY(account_id)
        REFERENCES provider_accounts(id)
);
```

### quota_windows

```sql
CREATE TABLE quota_windows (
    id TEXT PRIMARY KEY,
    snapshot_id TEXT NOT NULL,
    name TEXT NOT NULL,
    period_type TEXT NOT NULL,
    total REAL,
    used REAL,
    remaining REAL,
    remaining_percent REAL,
    unit TEXT NOT NULL,
    reset_at TEXT,
    is_current_constraint INTEGER NOT NULL,
    data_source TEXT NOT NULL,
    FOREIGN KEY(snapshot_id)
        REFERENCES quota_snapshots(id)
);
```

### usage_records

```sql
CREATE TABLE usage_records (
    id TEXT PRIMARY KEY,
    account_id TEXT,
    provider TEXT,
    model TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    reasoning_tokens INTEGER,
    cached_tokens INTEGER,
    total_tokens INTEGER,
    failed INTEGER NOT NULL,
    occurred_at TEXT NOT NULL
);
```

### sync_logs

```sql
CREATE TABLE sync_logs (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL,
    status TEXT NOT NULL,
    message TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT
);
```

### alert_records

```sql
CREATE TABLE alert_records (
    id TEXT PRIMARY KEY,
    account_id TEXT,
    alert_type TEXT NOT NULL,
    threshold REAL,
    message TEXT NOT NULL,
    triggered_at TEXT NOT NULL,
    acknowledged_at TEXT
);
```

---

# 十五、数据保留策略

## 最新状态

永久保留每个账号最新快照。

## 历史快照

* 每 15 分钟保留一条
* 默认保留 90 天
* 用户可以选择 30、90、180 天

## Usage Queue 原始记录

默认保留 30 天。

## 同步日志

只保留最近：

```text
2,000 条
或 30 天
```

## 告警记录

默认保留 90 天。

---

# 十六、安全设计

## 16.1 敏感信息

包括：

* CLIProxyAPI Management Key
* 火山 Access Key
* 火山 Secret Key
* OAuth Token
* 上游账号认证信息

## 16.2 存储方案

优先使用：

* Windows Credential Manager
* macOS Keychain

也可以使用 Tauri Stronghold。Tauri Stronghold 提供面向敏感数据的安全存储能力。([Tauri][4])

普通设置使用 Tauri Store：

* 球体大小
* 窗口位置
* 轮播速度
* 告警阈值
* 自动刷新频率

Tauri Store 适合保存普通持久化键值设置。([Tauri][5])

## 16.3 日志脱敏

日志中禁止出现：

* 完整密钥
* 完整邮箱
* 完整 Authorization Header
* 完整火山请求签名
* OAuth Token

脱敏示例：

```text
sk-123456789 → sk-****6789
name@example.com → na***@example.com
```

## 16.4 Tauri 权限

采用最小权限原则：

* 前端只能调用明确开放的 Commands
* 不开放任意 HTTP 请求权限
* 不开放任意文件系统权限
* 不允许前端直接访问数据库路径
* 不允许前端执行 Shell 命令

Tauri 2 使用权限和能力配置控制前端可调用的原生能力。([Tauri][6])

---

# 十七、告警系统

## 17.1 默认规则

### 额度偏低

```text
剩余低于 20%
```

### 额度紧张

```text
剩余低于 10%
```

### 额度耗尽

```text
剩余为 0%
```

### 全部账号不可用

CLIProxyAPI 中所有 Codex 账号均不可用。

### 额度恢复

账号由：

```text
Cooldown / Exhausted
```

变为：

```text
Available
```

### 同步异常

连续三次同步失败。

## 17.2 通知去重

同一个账号、同一个窗口、同一个阈值，只通知一次。

额度恢复到阈值以上后，才允许再次触发。

## 17.3 系统通知

使用 Tauri Notification Plugin，支持 Windows 和 macOS 原生通知。([Tauri][7])

通知示例：

```text
Codex 账号 03 当前额度只剩 8%，
预计 2 小时后恢复。
```

---

# 十八、系统托盘设计

托盘菜单：

```text
显示余量球
隐藏余量球
立即刷新
打开管理面板
暂停自动同步
开机启动
设置
检查更新
退出
```

Tauri 支持创建和定制系统托盘。([Tauri][8])

## 托盘图标状态

可以根据整体状态变化：

* 正常
* 警告
* 耗尽
* 离线
* 同步中

---

# 十九、开机启动与单实例

## 19.1 开机启动

使用 Tauri Autostart Plugin，支持 Windows 和 macOS。([Tauri][9])

默认不强制开启，首次启动时询问用户。

## 19.2 单实例

使用 Single Instance Plugin，避免用户重复启动多个同步进程。官方文档要求 Single Instance 插件优先注册。([Tauri][10])

第二次启动应用时：

* 激活已有实例
* 打开管理窗口
* 不创建新的后台同步任务

---

# 二十、窗口位置和多显示器

## 20.1 位置保存

主窗口可以使用 Window State Plugin 恢复状态；官方插件可以在应用重启后恢复窗口状态。([Tauri][11])

余量球建议自己存储：

```text
显示器 ID
相对 X
相对 Y
吸附边缘
距边缘距离
```

## 20.2 屏幕变化处理

需要处理：

* 外接显示器拔出
* 分辨率变化
* DPI 缩放变化
* Windows 任务栏位置变化
* macOS Dock 位置变化

如果上次屏幕不存在，将余量球移动到主屏幕右侧中央。

---

# 二十一、自动更新

使用 Tauri Updater Plugin。

支持：

* GitHub Releases
* 静态 JSON 更新文件
* 自建更新服务

Tauri Updater 支持动态服务器和静态 JSON 更新源。([Tauri][12])

## 更新策略

* 每 24 小时检查一次
* 用户可手动检查
* 不在额度同步过程中强制重启
* 下载后由用户确认安装
* 正式版本必须签名

---

# 二十二、错误处理

## 22.1 错误分类

```rust
pub enum AppError {
    Network,
    Authentication,
    Permission,
    ApiRateLimit,
    InvalidResponse,
    Database,
    CredentialStore,
    Timeout,
    UnsupportedVersion,
    Unknown,
}
```

## 22.2 用户提示

不要直接展示 Rust 技术错误。

错误：

```text
reqwest error: connection refused
```

转化为：

```text
无法连接 CLIProxyAPI，请检查服务地址和端口。
```

## 22.3 缓存降级

同步失败时：

* 保留上次成功数据
* 显示数据时间
* 标记“可能已过期”
* 不把未知数据变成 0%
* 不清空账号列表

---

# 二十三、性能目标

## 23.1 内存

目标：

```text
空闲状态低于 100 MB
```

## 23.2 CPU

空闲状态：

```text
平均低于 1%
```

## 23.3 动画

* 正常目标 30 FPS
* 鼠标悬停时最高 60 FPS
* 屏幕锁定或窗口不可见时暂停动画
* 系统省电模式下降低动画频率

## 23.4 网络

* 不在鼠标悬停时立即重复请求
* 悬停展示缓存数据
* 所有请求设置超时
* 同步任务合并
* 避免重复查询相同套餐

---

# 二十四、开发阶段规划

## 阶段一：工程骨架

完成：

* Tauri 2 工程
* Vue 3 + TypeScript
* 三窗口结构
* 系统托盘
* 单实例
* SQLite
* 日志
* 基础设置

## 阶段二：余量球

完成：

* 透明窗口
* 拖动
* 吸附
* 始终置顶
* 轮播
* 悬停面板
* 状态动画
* 窗口位置保存

## 阶段三：CLIProxyAPI

完成：

* 添加连接
* 测试连接
* 获取账号
* 获取状态
* 获取额度
* 读取 Usage Queue
* 多账号展示
* 异常降级

## 阶段四：火山引擎

完成：

* 凭证配置
* 请求签名
* 获取套餐
* Coding Plan 用量
* Agent Plan AFP 用量
* 多窗口标准化

## 阶段五：告警和历史

完成：

* 本地通知
* 告警规则
* 额度快照
* 基础趋势
* 同步日志

## 阶段六：发布

完成：

* Windows 安装包
* macOS 安装包
* 代码签名
* 自动更新
* 崩溃和日志导出
* 用户文档

---

# 二十五、MVP 优先级

## P0：必须完成

* Windows 和 macOS
* 余量球
* 悬停面板
* 系统托盘
* CLIProxyAPI 接入
* 火山引擎接入
* 当前可用额度
* 多额度窗口
* 自动同步
* 凭证安全存储
* SQLite
* 缓存降级
* 本地通知

## P1：应完成

* 历史趋势
* 多 CLIProxyAPI 实例
* 自定义轮播
* 自定义告警阈值
* 连接诊断
* 自动更新
* 开机启动

## P2：后续版本

* 自动路由联动
* 团队额度
* 云同步
* Web 管理端
* 手机端
* 更多渠道
* 插件机制

---

# 二十六、测试方案

## 26.1 单元测试

重点测试：

* 关键窗口算法
* 百分比计算
* 多账号汇总
* 时间窗口转换
* 重置倒计时
* 告警去重
* 数据脱敏

## 26.2 Provider 测试

使用 Mock Server 测试：

* 正常响应
* 空数据
* 字段缺失
* 401
* 403
* 429
* 500
* 超时
* 非法 JSON
* API 版本变化

## 26.3 桌面测试

覆盖：

* Windows 10
* Windows 11
* macOS Intel
* macOS Apple Silicon
* 单屏
* 双屏
* DPI 100%
* DPI 150%
* DPI 200%

## 26.4 数据测试

确认：

* 未知值不显示为 0
* 缓存数据有时间标识
* 多窗口选择正确
* Usage Queue 不重复消费
* 数据库异常不会导致应用崩溃

---

# 二十七、验收标准

## 余量球

* 启动后可正常显示。
* 支持拖动和边缘吸附。
* 支持始终置顶。
* 支持自动轮播。
* 悬停时显示详情。
* 退出主窗口后仍在托盘运行。
* 重启后恢复位置。

## CLIProxyAPI

* 能连接用户配置的实例。
* 能读取多个 Codex 账号。
* 能展示账号额度。
* 能展示多个额度窗口。
* 能展示冷却和耗尽状态。
* 能展示恢复时间。
* 能消费并保存 Usage Queue。

## 火山引擎

* 能识别 Coding Plan。
* 能识别 Agent Plan。
* 能读取套餐用量。
* 能读取 AFP 多窗口额度。
* 能正确选择当前关键窗口。

## 数据安全

* 密钥不明文保存。
* 日志不包含敏感凭证。
* 前端无法直接读取密钥。
* 网络失败时保留缓存数据。

## 稳定性

* 后台运行 24 小时无明显内存增长。
* API 失败不会导致应用退出。
* 重复启动不会产生多个实例。
* 多显示器切换后余量球不会丢失。

---

# 二十八、最终推荐结论

本项目建议采用：

```text
Tauri 2
Vue 3
TypeScript
Vite
Pinia
Rust
Tokio
Reqwest
Serde
SQLite
SQLx
Tracing
Stronghold 或系统凭证库
```

整体采用：

```text
Vue 负责显示和交互
Rust 负责接口、业务、数据和安全
SQLite 负责本地历史
Tauri Events 负责数据推送
Provider Adapter 负责渠道扩展
```

首版应该优先把以下闭环做完整：

```text
添加渠道
→ 获取额度
→ 标准化窗口
→ 计算当前可用额度
→ 展示余量球
→ 额度不足通知
→ 接口失败展示缓存
```

完成这个闭环后，AI 流量余量球就已经具备可实际使用和发布的价值。

[1]: https://v2.tauri.app/reference/config/?utm_source=chatgpt.com "Configuration"
[2]: https://vuejs.org/guide/typescript/overview?utm_source=chatgpt.com "Using Vue with TypeScript"
[3]: https://docs.rs/sqlx/latest/sqlx/?utm_source=chatgpt.com "sqlx - Rust"
[4]: https://v2.tauri.app/reference/javascript/stronghold/?utm_source=chatgpt.com "tauri-apps/plugin-stronghold"
[5]: https://v2.tauri.app/plugin/store/?utm_source=chatgpt.com "Store"
[6]: https://v2.tauri.app/reference/acl/core-permissions/?utm_source=chatgpt.com "Core Permissions"
[7]: https://v2.tauri.app/plugin/notification/?utm_source=chatgpt.com "Notifications"
[8]: https://v2.tauri.app/learn/system-tray/?utm_source=chatgpt.com "System Tray"
[9]: https://v2.tauri.app/plugin/autostart/?utm_source=chatgpt.com "Autostart"
[10]: https://v2.tauri.app/plugin/single-instance/?utm_source=chatgpt.com "Single Instance"
[11]: https://v2.tauri.app/plugin/window-state/?utm_source=chatgpt.com "Window State"
[12]: https://v2.tauri.app/plugin/updater/?utm_source=chatgpt.com "Updater"
