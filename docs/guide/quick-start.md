# 快速开始

TokenBall 目前适合从源码运行和本地构建。首次使用建议先启动桌面应用，再进入管理窗口配置 Provider 实例。

## 准备环境

- Node.js 18+，推荐 Node.js 22
- npm
- Rust / Cargo
- Windows：Visual Studio Build Tools、WebView2 Runtime
- macOS：Xcode Command Line Tools

开发环境细节见 [`/develop/setup`](/develop/setup)。

## 从源码运行

在仓库根目录执行：

```bash
npm install
npm run tauri:dev
```

如果只需要调试前端页面：

```bash
npm run dev
```

默认前端地址是 `http://127.0.0.1:1420`，Tauri 配置会使用同一个端口。

## 首次配置

启动后进入管理窗口：

1. 点击新增实例，选择 Provider：`CLIProxyAPI`、`火山引擎` 或 `千问`。
2. 填写实例名称、服务地址和凭证信息。
3. 点击保存。
4. 点击测试，确认连接可用。
5. 回到总览页，点击刷新同步额度。

配置完成后，桌面额度球会按显示设置轮播展示总余量、可用账号数量、连接状态和账号额度。

## 悬浮球操作

| 操作 | 效果 |
| --- | --- |
| 鼠标悬停 | 打开额度详情面板 |
| 双击 | 打开管理窗口总览 |
| 右键 | 隐藏额度球 |
| 点击刷新图标 | 立即刷新全部额度 |
| 拖动 | 移动额度球位置 |

## 下一步

- 了解额度和窗口概念：[`/guide/core-concepts`](/guide/core-concepts)
- 按实例完成接入：[`/guide/instance-onboarding`](/guide/instance-onboarding)
- 查看字段速查：[`/guide/providers`](/guide/providers)
- 查看常用命令：[`/reference/commands`](/reference/commands)
