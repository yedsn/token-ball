# 千问 Token Plan 用量接口说明

**更新日期**：2026-07-26

本文记录 TokenBall 当前接入千问 Token Plan 个人版用量的接口来源、调用方式和字段映射，便于后续排查“连接成功但没有额度信息”等问题。

## 1. 接入结论

当前接入的是千问控制台内部 API，不是网页 DOM 抓取。

主路径使用 `platform.qianwenai.com` 登录态 Cookie 调用控制台网关：先取 `secToken`，再通过 `cs-data.qianwenai.com/data/api.json` 转发到 `zeldaHttp.apikeyMgr./tokenplan/personal/api/v2/usage`。

| 项目 | 内容 |
| --- | --- |
| 页面入口 | `https://platform.qianwenai.com/home/billing/subscription/token-plan-individual` |
| secToken 接口 | `GET https://platform-home.qianwenai.com/tool/user/info.json` |
| 用量网关 | `POST https://cs-data.qianwenai.com/data/api.json` |
| Product | `sfm_bailian` |
| Action | `BroadScopeAspnGateway` |
| 业务 API | `zeldaHttp.apikeyMgr./tokenplan/personal/api/v2/usage` |
| 本地实现 | `src-tauri/src/providers/qianwen.rs` |

## 2. 调用流程

### 2.1 获取 secToken

**用途**：千问控制台网关调用需要 `sec_token`，该值来自登录态用户信息接口。

| 项目 | 内容 |
| --- | --- |
| 方法 | `GET` |
| 地址 | `https://platform-home.qianwenai.com/tool/user/info.json` |
| 鉴权 | 请求头 `Cookie: <千问控制台登录态 Cookie>` |
| 响应字段 | `data.secToken` |

### 2.2 查询个人版用量

**用途**：查询 Token Plan 个人版 5 小时窗口和每周窗口的用量百分比。

| 项目 | 内容 |
| --- | --- |
| 方法 | `POST` |
| 地址 | `https://cs-data.qianwenai.com/data/api.json` |
| Content-Type | `application/x-www-form-urlencoded` |
| 鉴权 | 请求头 `Cookie` + 表单字段 `sec_token` |

Query 参数：

| 字段 | 示例 | 说明 |
| --- | --- | --- |
| `product` | `sfm_bailian` | 控制台产品标识 |
| `action` | `BroadScopeAspnGateway` | 控制台网关动作 |
| `api` | `zeldaHttp.apikeyMgr./tokenplan/personal/api/v2/usage` | 被转发的业务 API |

Form 字段：

| 字段 | 示例 | 说明 |
| --- | --- | --- |
| `product` | `sfm_bailian` | 与 Query 保持一致 |
| `action` | `BroadScopeAspnGateway` | 与 Query 保持一致 |
| `sec_token` | `<secToken>` | 来自用户信息接口 |
| `region` | `cn-beijing` | 控制台请求区域 |
| `params` | JSON 字符串 | 包含 `Api`、`Data.cornerstoneParam`、`V` |

`params` 示例：

```json
{
  "Api": "zeldaHttp.apikeyMgr./tokenplan/personal/api/v2/usage",
  "Data": {
    "cornerstoneParam": {
      "domain": "platform.qianwenai.com",
      "consoleSite": "QIANWENAI",
      "console": "ONE_CONSOLE",
      "xsp_lang": "zh-CN",
      "protocol": "V2",
      "productCode": "p_efm"
    }
  },
  "V": "1.0"
}
```

## 3. 响应字段映射

业务数据位于 `data.DataV2.data.data`。

| 响应字段 | 类型 | 本地展示 | 说明 |
| --- | --- | --- | --- |
| `per5HourPercentage` | number | `5 小时` 窗口 | 已用百分比，剩余百分比按 `100 - value` 计算 |
| `per1WeekPercentage` | number | `每周` 窗口 | 剩余百分比，直接作为剩余值展示 |
| `per1WeekResetTime` | number | 每周重置时间 | 毫秒时间戳 |

示例：如果接口返回 `per5HourPercentage = 0.0`、`per1WeekPercentage = 1.0`，TokenBall 会显示 5 小时剩余 `100%`、每周剩余 `1%`。

## 4. 相关补充接口

当前实现保留以下旧控制台接口作为补充信息来源，用于尝试读取主套餐席位或加油包明细；个人版 Token Plan 的主要展示不依赖它们。

| 接口 | 用途 | 说明 |
| --- | --- | --- |
| `BssOpenAPI-V3/GetSeatSubscriptionSummary` | 主套餐席位信息 | 对个人版 Token Plan 不作为主路径 |
| `BssOpenAPI-V3/GetSubscriptionDetail` | 订阅明细和加油包 | 成功时追加展示加油包额度 |
| `zeldaHttp.apikeyMgr./tokenplan/personal/api/v2/subscription` | 个人版订阅 | 页面源码可见，当前未接入 |
| `zeldaHttp.apikeyMgr./tokenplan/personal/api/v2/quota-config` | 个人版额度配置 | 页面源码可见，当前未接入 |

## 5. 排查要点

| 现象 | 优先检查 |
| --- | --- |
| 测试连接失败 | Cookie 是否来自 `platform.qianwenai.com` 登录态，请求 `tool/user/info.json` 是否能返回 `data.secToken` |
| 连接成功但没有额度 | 是否已触发额度刷新；当前前端测试成功后会自动刷新一次 |
| 只有空账号提示 | `usage` 响应中是否缺少 `DataV2.data.data.per5HourPercentage` 和 `per1WeekPercentage` |
| 每周时间异常 | `per1WeekResetTime` 是否为毫秒时间戳 |
