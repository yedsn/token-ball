# Proposal

## Why

管理页的全账号额度重置表和悬浮详情目前只显示优先周期的重置时间，用户无法直接对照 5 小时、周、月额度各自何时恢复。现有 `QuotaWindow.resetAt` 已保存独立时间，需要让汇总展示完整呈现这些数据。

## What Changes

- 管理页继续每个账号一行，在 5 小时、周、月各列中显示“剩余百分比 + 本周期重置时间”。
- 悬浮详情的三个周期块采用相同信息结构，保留账号到期时间和更新时间。
- 保留优先周期信息与现有账号排序；原单一“重置时间”明确标注为“优先重置”，不再作为所有周期的重置时间。
- 周期不存在、时间缺失或无效时显示 `--`，禁止借用其他周期、账号汇总或订阅到期时间。
- 补齐火山 `afp-five-hour` / “5 小时 AFP”等已有数据的周期识别，两处展示共用识别与取值逻辑。
- 新增周期时间按周期长度使用独立的时间分级颜色：5 小时周期按小时、周周期按天、月周期按更长天数判断，让颜色反映该周期内真正的紧迫程度。

## Capabilities

### New Capabilities

- `quota-period-reset-display`：规定管理页和悬浮详情中 5 小时、周、月额度与独立重置时间的对应展示、缺失处理及兼容行为。

### Modified Capabilities

无。当前 `openspec/specs/` 尚无已发布能力；已有 MVP 变更中的规范保持不动，本次以独立展示能力补充要求。

## Impact

- 前端：`src-ui/src/views/ManagementView.vue`、`src-ui/src/views/HoverPanelView.vue`、`src-ui/src/styles.css`，以及一个轻量共享周期工具和相应回归用例。
- 复用 `src-ui/src/types.ts` 中的 `QuotaAccount.windows`、`QuotaWindow.resetAt`，无需更改类型契约、数据库、Tauri 命令或提供方 API。
- 不改流量球轮播、关键额度计算、同步调度、账号筛选或订阅到期含义；不新增运行时依赖。
