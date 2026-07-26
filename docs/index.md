---
layout: home
hero:
  name: TokenBall
  text: 桌面 AI 额度悬浮球
  tagline: 聚合 CLIProxyAPI、火山引擎和千问 Token Plan 的账号额度、窗口余量与恢复时间，让关键额度状态常驻桌面。
  image:
    src: /logo.png
    alt: TokenBall
  actions:
    - theme: brand
      text: 快速开始
      link: /guide/quick-start
    - theme: alt
      text: Provider 配置
      link: /guide/providers
    - theme: alt
      text: GitHub 仓库
      link: https://github.com/yedsn/token-ball
features:
  - icon: "🟢"
    title: 桌面常驻额度球
    details: 透明悬浮窗展示总余量、可用账号、连接状态和自定义内容，支持拖动、隐藏和快速刷新。
  - icon: "📊"
    title: 关键窗口优先
    details: 从多个统计窗口中选择当前最紧张的额度窗口，优先展示真正限制继续使用的余量。
  - icon: "🔌"
    title: 多 Provider 聚合
    details: 支持 CLIProxyAPI、火山引擎和千问 Token Plan，可配置多个实例统一查看。
  - icon: "🪟"
    title: 悬停详情面板
    details: 鼠标悬停即可查看账号列表、5 小时 / 周 / 月窗口、到期时间、使用量和所属实例。
  - icon: "🧭"
    title: 管理窗口
    details: 在独立窗口中维护实例、测试连接、刷新额度、调整显示内容和管理插件清单。
  - icon: "💾"
    title: 本地优先
    details: 使用 SQLite 保存连接、额度快照和显示设置，接口失败时保留上次缓存数据。
---

<div class="tb-home-section">

## 三个窗口

<div class="tb-home-grid">
  <div class="tb-home-card">
    <strong>额度球</strong>
    <span>常驻桌面，轮播展示总额度、账号数量、连接状态和自定义显示项。</span>
  </div>
  <div class="tb-home-card">
    <strong>悬停面板</strong>
    <span>快速查看账号排序、窗口余量、到期时间和最新同步状态。</span>
  </div>
  <div class="tb-home-card">
    <strong>管理窗口</strong>
    <span>新增 Provider 实例、保存凭证、测试连接、配置外观和插件清单。</span>
  </div>
</div>

</div>

<div class="tb-home-section">

## 支持的 Provider

<div class="tb-provider-grid">
  <div class="tb-provider-card">
    <strong>CLIProxyAPI</strong>
    <span>读取多个 Codex 账号的额度、状态、请求统计和恢复时间。</span>
  </div>
  <div class="tb-provider-card">
    <strong>火山引擎</strong>
    <span>支持 Coding Plan 与 Agent Plan，可使用官方 OpenAPI 或控制台页面渠道。</span>
  </div>
  <div class="tb-provider-card">
    <strong>千问 Token Plan</strong>
    <span>通过控制台登录态 Cookie 查询个人版 5 小时窗口和每周窗口。</span>
  </div>
</div>

</div>

<div class="tb-home-section">

## 从这里开始

- 想本地跑起来：看 [`/guide/quick-start`](/guide/quick-start)
- 想配置额度来源：看 [`/guide/providers`](/guide/providers)
- 想了解核心概念：看 [`/guide/core-concepts`](/guide/core-concepts)
- 想参与开发：看 [`/develop/setup`](/develop/setup)
- 想查项目结构和命令：看 [`/reference/project-structure`](/reference/project-structure)

</div>
