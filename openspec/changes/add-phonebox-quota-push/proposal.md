# Proposal

## Why

希望在 PhoneBox 手机面板上查看 TokenBall 已采集的多平台额度，而不维护定制版 PhoneBox 或重复配置平台凭据。PhoneBox 已提供推送接口和自定义 HTML 数据通道，可以通过 TokenBall 主动推送与可导入页面模板完成集成。

## What Changes

- TokenBall 增加默认关闭的 PhoneBox 集成，配置同机 Agent 地址，支持保存、立即推送及查看最近投递结果。
- 后端在额度刷新、实例范围变化后推送最新展示快照，并通过轻量心跳在 PhoneBox 重启后自动恢复；投递失败不改变平台采集结果。
- 使用现有 `POST /api/push/tokenball`，发送版本化、白名单字段、脱敏的额度数据；兼容普通推送数值组件，并包含自定义 HTML 所需的账号及周期详情。
- 区分最后投递时间、实际采集时间、平台异常与数据过期，防止重发旧缓存被视为新额度。
- 在 TokenBall 仓库提供可导入的 PhoneBox 页面 JSON、自定义 HTML 源模板及接入说明，展示总览、账号额度、各周期恢复时间和缓存状态。
- 第一版只接入同机 PhoneBox：不开放 TokenBall HTTP 服务，不支持跨机器投递，不修改 PhoneBox 源码，不提供手机端账号管理或触发平台刷新的操作。

## Capabilities

### New Capabilities

- `phonebox-quota-push`：定义集成配置、额度展示契约、安全范围、投递生命周期及故障隔离。
- `phonebox-quota-panel`：定义无需修改 PhoneBox 即可导入的展示模板、周期取值与过期状态。

### Modified Capabilities

无。当前 `openspec/specs/` 没有已发布规格；本次不修改已有额度采集、同步调度和桌面显示的业务要求。

## Impact

- Rust 后端：新增 PhoneBox 集成模块、独立集成设置与 Tauri 命令，接入启动、额度刷新及实例启停/删除/导入事件；复用 SQLite `settings` 与现有 `reqwest`，不新增数据库业务表或 HTTP 服务端依赖。
- Vue 前端：管理窗口通用设置中新增 PhoneBox 区域，并补充类型、调用封装和状态展示。
- 集成资源：计划新增 `integrations/phonebox/` 页面模板及配套生成/检查入口，新增 `docs/guide/phonebox.md`。
- 外部系统：依赖 PhoneBox 现有推送接口、`phonebox.on('push', ...)` 与页面导入格式；仅由用户手动导入/保存页面，不自动改写 PhoneBox 设置。
- 验证：补充快照映射、隐私、超时/重试、配置兼容、模板渲染及体积限制测试，并进行同机 Agent 联调。
