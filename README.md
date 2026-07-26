# TokenBall

<p align="center">
  <img src="src-tauri/icons/icon.png" alt="TokenBall" width="160">
</p>

<p align="center"><strong>一个面向 AI 编程重度用户的桌面额度悬浮球。</strong></p>
<p align="center">把 CLIProxyAPI、火山引擎和千问 Token Plan 的账号额度、窗口余量和恢复时间聚合到桌面常驻窗口里。</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2021-000000?logo=rust" alt="Rust 2021">
  <img src="https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri" alt="Tauri 2">
  <img src="https://img.shields.io/badge/UI-Vue_3-4FC08D?logo=vue.js" alt="Vue 3">
  <img src="https://img.shields.io/badge/Storage-SQLite-003B57?logo=sqlite" alt="SQLite">
  <img src="https://img.shields.io/badge/Provider-CLIProxyAPI%20%7C%20Volcengine%20%7C%20Qianwen-8B5CF6" alt="Providers">
  <a href="https://github.com/yedsn/token-ball/actions/workflows/pages.yml">
    <img src="https://github.com/yedsn/token-ball/actions/workflows/pages.yml/badge.svg?branch=master" alt="Docs Site">
  </a>
</p>

<p align="center">
  <a href="https://yedsn.github.io/token-ball/">官网</a>
  ·
  <a href="https://gitee.com/hongxiaojian/token-ball">Gitee</a>
  ·
  <a href="https://github.com/yedsn/token-ball">GitHub</a>
  ·
  <a href="docs/guide/quick-start.md">使用说明</a>
</p>

---

## 一句话介绍

TokenBall 使用 **Tauri 2 + Rust + Vue 3** 构建，目标是把分散在多个 AI 编程服务里的额度信息放到桌面上，减少反复打开控制台、切账号、查用量的成本。

它适合这些场景：

- 同时使用多个 Codex / Coding / Agent / Token Plan 账号
- 需要随时知道哪个账号额度最低、哪个窗口即将恢复
- 使用 CLIProxyAPI 统一管理多个 Codex 账号
- 使用火山引擎方舟 Coding Plan / Agent Plan
- 使用千问 Token Plan 个人版并需要查看 5 小时和每周窗口

## 为什么值得用

| 方向 | 你得到什么 |
| --- | --- |
| 桌面常驻 | 透明悬浮额度球展示总余量、账号数量、连接状态和自定义内容 |
| 多 Provider 聚合 | 支持 CLIProxyAPI、火山引擎、千问 Token Plan，多实例统一展示 |
| 关键窗口优先 | 自动选择当前最紧张的额度窗口，避免被平均值误导 |
| 悬停详情 | 鼠标悬停显示账号列表、5 小时 / 周 / 月窗口、到期时间和使用量 |
| 本地存储 | SQLite 保存连接、账号、额度快照、显示设置和插件清单 |
| 桌面体验 | 系统托盘、独立管理窗口、悬浮窗拖动、刷新按钮和窗口显示控制 |

## 核心能力

### 额度悬浮球

- 显示总剩余额度、可用账号数量、连接状态或自定义内容
- 4 秒自动轮播，鼠标悬停暂停
- 根据剩余比例显示正常、预警、紧张和缓存状态
- 右下角可选刷新按钮
- 双击打开管理窗口，右键隐藏悬浮球

### 悬停详情面板

- 展示启用账号数量和等效剩余账号数
- 按到期时间排序展示账号额度
- 同时显示 5 小时、每周、每月窗口
- 显示当前关键窗口、用量、恢复时间和所属实例
- 支持悬停期间保持面板，离开后延迟关闭

### 管理窗口

- 总览所有 Provider 实例和账号额度
- 新增、编辑、测试、停用和删除实例
- 配置流量球显示项、动画、刷新按钮和程序图标
- 管理本地插件清单和自定义显示内容

## 支持的 Provider

### CLIProxyAPI

- 配置一个或多个 CLIProxyAPI 实例
- 通过 Management Key 读取账号和额度
- 展示账号状态、关键窗口、请求统计和恢复时间

### 火山引擎

- 支持官方 OpenAPI 渠道和控制台页面渠道
- 支持 Coding Plan 与 Agent Plan
- 支持 AK/SK、Region、Service、ProjectName、SeatID 等配置

### 千问 Token Plan

- 使用千问控制台登录态 Cookie 调用内部用量接口
- 展示个人版 Token Plan 的 5 小时窗口和每周窗口
- 接口细节见 [`docs/qianwen-token-plan-api.md`](docs/qianwen-token-plan-api.md)

## 快速开始

### 1. 准备环境

- Node.js 18+，推荐 Node.js 22
- npm
- Rust / Cargo
- Tauri 2 桌面开发依赖
- Windows 需要 WebView2 Runtime 与 Visual Studio Build Tools

更完整说明见 [`docs/develop/setup.md`](docs/develop/setup.md)。

### 2. 启动桌面应用

```bash
npm install
npm run tauri:dev
```

如果只想启动 Web 前端：

```bash
npm run dev
```

默认前端地址：`http://127.0.0.1:1420`

### 3. 构建发布包

```bash
npm run tauri:build
```

构建产物位于 `src-tauri/target/release/bundle/`。

## 文档站点

本仓库已配置 VitePress 文档站点：

```bash
npm run docs:dev
npm run docs:build
npm run docs:preview
```

文档入口：[`docs/index.md`](docs/index.md)

## 项目结构

```text
src-ui/                 Vue 3 前端
src-tauri/              Tauri 2 + Rust 后端
docs/                   VitePress 文档站点
scripts/release/        发布辅助脚本
.github/workflows/      GitHub Actions
```

更完整说明见 [`docs/reference/project-structure.md`](docs/reference/project-structure.md)。

## 项目状态

TokenBall 仍处于早期版本，接口和数据模型会继续随 Provider 实际接口变化调整。涉及 Cookie、Access Key、Management Key 的功能请只在可信本机环境使用，并注意不要把本地数据库、日志或配置文件提交到仓库。

## License

本项目基于 [MIT License](LICENSE) 开源。
